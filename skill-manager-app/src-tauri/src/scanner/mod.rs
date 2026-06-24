mod agents;
mod codex;
mod collector;
mod dedup;
mod openclaw;
mod parser;
mod utils;

use crate::models::{AgentDir, IssueLevel, ScanIssue, ScanResult, Skill};
use crate::platform;
use agents::{discover_agent_dirs, update_agent_counts};
use collector::{
    collect_direct_skill_entries, collect_skill_entries, collect_skill_entries_with_options,
    CollectOptions, SKILL_INDEX_FILENAME,
};
use dedup::dedup_skills;
use openclaw::{
    extra_skill_dirs, find_openclaw_package_dir, openclaw_loaded_skills, openclaw_workspace_paths,
    read_openclaw_config, OpenClawLoadedSkill, OPENCLAW_SOURCE_AGENTS_PERSONAL,
    OPENCLAW_SOURCE_AGENTS_PROJECT, OPENCLAW_SOURCE_BUNDLED, OPENCLAW_SOURCE_EXTRA,
    OPENCLAW_SOURCE_MANAGED, OPENCLAW_SOURCE_WORKSPACE,
};
use parser::{parse_skill_md, ParseError};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use thiserror::Error;
use utils::{category_from_path, resolve_real_path, skill_id};

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("Unable to resolve the current user home directory")]
    MissingHome,
    #[error(transparent)]
    Parse(#[from] ParseError),
}

#[derive(Debug, Clone)]
struct SkillRoot {
    path: PathBuf,
    openclaw_source: Option<&'static str>,
    include_system: bool,
}

pub fn scan_skills() -> Result<ScanResult, ScanError> {
    let started_at = Instant::now();
    let home = platform::home_dir().ok_or(ScanError::MissingHome)?;

    let mut agents = discover_agent_dirs(&home);
    let mut skills = Vec::new();
    let mut issues = Vec::new();

    for agent in &mut agents {
        let roots = agent_skill_roots(&home, agent, &mut issues);
        agent.exists = !roots.is_empty();
        let openclaw_loaded_skills = if agent.key == "openclaw" && agent.exists {
            match openclaw_loaded_skills() {
                Ok(skills) => Some(skills),
                Err(message) => {
                    issues.push(ScanIssue {
                        path: agent.path.clone(),
                        level: IssueLevel::Warning,
                        message: format!("Unable to read OpenClaw loaded skills: {message}"),
                    });
                    None
                }
            }
        } else {
            None
        };

        for root in roots {
            let entries = if agent.key == "openclaw" {
                collect_direct_skill_entries(&root.path, &mut issues, true)
            } else if root.include_system {
                collect_skill_entries_with_options(
                    &root.path,
                    &mut issues,
                    CollectOptions {
                        include_system: true,
                    },
                )
            } else {
                collect_skill_entries(&root.path, &mut issues)
            };

            for entry in entries {
                match scan_skill_entry(agent, &root.path, entry) {
                    Ok(Some(mut skill)) => {
                        if should_include_skill(
                            &mut skill,
                            openclaw_loaded_skills.as_ref(),
                            root.openclaw_source,
                        ) {
                            skills.push(skill);
                        }
                    }
                    Ok(None) => {}
                    Err(error) => issues.push(error_to_issue(error)),
                }
            }
        }
    }

    let skills = dedup_skills(skills);
    update_agent_counts(&mut agents, &skills);

    Ok(ScanResult {
        skills,
        agents,
        issues,
        duration_ms: started_at.elapsed().as_millis() as u64,
    })
}

fn agent_skill_roots(home: &Path, agent: &AgentDir, issues: &mut Vec<ScanIssue>) -> Vec<SkillRoot> {
    let mut roots = Vec::new();

    match agent.key.as_str() {
        "openclaw" => push_openclaw_skill_roots(&mut roots, home, issues),
        "codex" => push_codex_skill_roots(&mut roots, home),
        _ => push_existing_root(&mut roots, agent.path.clone(), None, false),
    }

    roots
}

fn push_codex_skill_roots(roots: &mut Vec<SkillRoot>, home: &Path) {
    push_existing_root(roots, home.join(".codex/skills"), None, true);
    push_existing_root(roots, home.join(".agents/skills"), None, false);

    for plugin_skills in codex::plugin_skill_roots(home) {
        push_existing_root(roots, plugin_skills, None, false);
    }
}

fn push_openclaw_skill_roots(roots: &mut Vec<SkillRoot>, home: &Path, issues: &mut Vec<ScanIssue>) {
    let openclaw_home = home.join(".openclaw");
    let config = read_openclaw_config(&openclaw_home, issues);

    for workspace in openclaw_workspace_paths(home, config.as_ref()) {
        push_existing_root(
            roots,
            workspace.join(".agents/skills"),
            Some(OPENCLAW_SOURCE_AGENTS_PROJECT),
            false,
        );
        push_existing_root(
            roots,
            workspace.join("skills"),
            Some(OPENCLAW_SOURCE_WORKSPACE),
            false,
        );
    }

    push_existing_root(
        roots,
        home.join(".agents/skills"),
        Some(OPENCLAW_SOURCE_AGENTS_PERSONAL),
        false,
    );

    push_existing_root(
        roots,
        openclaw_home.join("skills"),
        Some(OPENCLAW_SOURCE_MANAGED),
        false,
    );

    if let Some(managed_dir) = config
        .as_ref()
        .and_then(|c| c.get("managedSkillsDir"))
        .and_then(serde_json::Value::as_str)
    {
        push_existing_root(
            roots,
            openclaw::expand_tilde(managed_dir, home),
            Some(OPENCLAW_SOURCE_MANAGED),
            false,
        );
    }

    if let Some(package_dir) = find_openclaw_package_dir(home) {
        push_existing_root(
            roots,
            package_dir.join("skills"),
            Some(OPENCLAW_SOURCE_BUNDLED),
            false,
        );
    }

    for extra_dir in extra_skill_dirs(config.as_ref(), home) {
        push_existing_root(roots, extra_dir, Some(OPENCLAW_SOURCE_EXTRA), false);
    }

    if let Some(extensions_dir) = config
        .as_ref()
        .and_then(|c| c.get("extensionsDir"))
        .and_then(serde_json::Value::as_str)
    {
        push_extension_skill_roots(roots, &openclaw::expand_tilde(extensions_dir, home));
    }
}

fn push_extension_skill_roots(roots: &mut Vec<SkillRoot>, extensions_dir: &Path) {
    let entries = match fs::read_dir(extensions_dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        push_existing_root(
            roots,
            entry.path().join("skills"),
            Some(OPENCLAW_SOURCE_EXTRA),
            false,
        );
    }
}

fn push_existing_root(
    roots: &mut Vec<SkillRoot>,
    path: PathBuf,
    openclaw_source: Option<&'static str>,
    include_system: bool,
) {
    if !path.is_dir() {
        return;
    }

    let key = platform::path_key(&path);
    if roots
        .iter()
        .any(|root| platform::path_key(&root.path) == key)
    {
        return;
    }

    roots.push(SkillRoot {
        path,
        openclaw_source,
        include_system,
    });
}

fn scan_skill_entry(
    agent: &AgentDir,
    source_root: &Path,
    path: PathBuf,
) -> Result<Option<Skill>, ScanError> {
    let is_symlink = path.is_symlink();
    let (real_path, is_broken_link) = resolve_real_path(&path, is_symlink);

    if is_broken_link {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("broken-symlink")
            .to_string();

        return Ok(Some(Skill {
            id: skill_id(&agent.key, &path),
            name,
            path,
            real_path,
            is_symlink: true,
            is_broken_link: true,
            description: "Symlink target is missing".to_string(),
            version: "Unknown".to_string(),
            author: None,
            category: Some("other".to_string()),
            custom_tags: Vec::new(),
            requires_agent: None,
            source_agents: vec![agent.key.clone()],
            source_agent_labels: vec![agent.label.clone()],
            body: "This Skill cannot be previewed because its symlink target no longer exists."
                .to_string(),
            warnings: vec!["Symlink target not found".to_string()],
        }));
    }

    let skill_md_path = path.join(SKILL_INDEX_FILENAME);
    let raw = fs::read_to_string(&skill_md_path).map_err(|source| ParseError::Read {
        path: skill_md_path.clone(),
        source,
    })?;

    let (metadata, body) = parse_skill_md(&skill_md_path, &raw)?;

    let Some(name) = metadata.name else {
        return Err(ParseError::MissingName {
            path: skill_md_path,
        }
        .into());
    };

    let mut warnings = Vec::new();
    if metadata.version.is_none() {
        warnings.push("Missing version field".to_string());
    }
    if metadata.description.is_none() {
        warnings.push("Missing description field".to_string());
    }

    let category = metadata
        .category
        .or_else(|| category_from_path(source_root, &path))
        .or(Some("other".to_string()));

    Ok(Some(Skill {
        id: skill_id(&agent.key, &path),
        name,
        path,
        real_path,
        is_symlink,
        is_broken_link,
        description: metadata
            .description
            .unwrap_or_else(|| "(No description)".to_string()),
        version: metadata.version.unwrap_or_else(|| "Unknown".to_string()),
        author: metadata.author,
        category,
        custom_tags: metadata.custom_tags,
        requires_agent: metadata.requires_agent,
        source_agents: vec![agent.key.clone()],
        source_agent_labels: vec![agent.label.clone()],
        body,
        warnings,
    }))
}

fn should_include_skill(
    skill: &mut Skill,
    openclaw_loaded_skills: Option<&HashMap<String, OpenClawLoadedSkill>>,
    openclaw_source: Option<&str>,
) -> bool {
    // Non-OpenClaw skills are always included
    if openclaw_source.is_none() {
        return true;
    }

    // OpenClaw skills require loaded_skills map
    let Some(loaded_skills) = openclaw_loaded_skills else {
        return false;
    };

    let Some(loaded) = loaded_skills.get(&skill.name) else {
        return false;
    };

    if openclaw_source != Some(loaded.source.as_str()) {
        return false;
    }

    if let Some(description) = &loaded.description {
        skill.description = description.clone();
    }

    true
}

fn error_to_issue(error: ScanError) -> ScanIssue {
    match error {
        ScanError::MissingHome => ScanIssue {
            path: PathBuf::new(),
            level: IssueLevel::Error,
            message: "Unable to resolve the current user home directory".to_string(),
        },
        ScanError::Parse(ParseError::Read { path, source }) => ScanIssue {
            path,
            level: IssueLevel::Warning,
            message: format!("Unable to read file: {source}"),
        },
        ScanError::Parse(ParseError::Yaml { path, source }) => ScanIssue {
            path,
            level: IssueLevel::Warning,
            message: format!("Invalid YAML frontmatter: {source}"),
        },
        ScanError::Parse(ParseError::MissingName { path }) => ScanIssue {
            path,
            level: IssueLevel::Warning,
            message: "Missing required field `name`".to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openclaw::OpenClawLoadedSkill;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_standard_skill_frontmatter() {
        let raw = r#"---
name: pdf-extractor
version: 1.2.3
description: Extract text from PDFs
author: Frost
category: data
tags:
  - pdf
  - ocr
requires_agent: ">=2.0"
---

# PDF Extractor
"#;

        let (metadata, body) = parse_skill_md(Path::new("SKILL.md"), raw).unwrap();
        assert_eq!(metadata.name.as_deref(), Some("pdf-extractor"));
        assert_eq!(metadata.version.as_deref(), Some("1.2.3"));
        assert_eq!(metadata.custom_tags, vec!["pdf", "ocr"]);
        assert!(body.starts_with("# PDF Extractor"));
    }

    #[test]
    fn detects_frontmatter_boundary() {
        let (frontmatter, body) = parser::split_frontmatter("name: x\n---\n# Body").unwrap();
        assert_eq!(frontmatter, "name: x");
        assert_eq!(body, "# Body");
    }

    #[test]
    fn infers_category_from_nested_skill_path() {
        let root = Path::new("/Users/example/.hermes/skills");
        assert_eq!(
            category_from_path(root, Path::new("/Users/example/.hermes/skills/apple/notes"))
                .as_deref(),
            Some("apple")
        );
        assert_eq!(
            category_from_path(root, Path::new("/Users/example/.hermes/skills/top-level")),
            None
        );
    }

    #[test]
    fn parses_openclaw_json_after_log_noise() {
        let stdout = r#"[INFO] Config loaded: {"debug": true}
[skills] Skipping something noisy
{
  "workspaceDir": "/tmp/workspace",
  "managedSkillsDir": "/tmp/skills",
  "skills": [
    {
      "name": "xlsx",
      "description": "Spreadsheet skill",
      "eligible": true,
      "source": "openclaw-managed"
    }
  ]
}"#;

        let value = openclaw::parse_openclaw_skills_json(stdout).unwrap();
        let skills = value
            .get("skills")
            .and_then(serde_json::Value::as_array)
            .unwrap();

        assert_eq!(skills.len(), 1);
        assert_eq!(
            skills[0].get("name").and_then(serde_json::Value::as_str),
            Some("xlsx")
        );
    }

    #[test]
    fn parses_openclaw_json_with_multiple_json_blocks() {
        let stdout = r#"[INFO] metadata follows
{
  "debug": true
}
[skills] actual output follows
{
  "skills": [
    {
      "name": "xlsx",
      "description": "Spreadsheet skill",
      "eligible": true,
      "source": "openclaw-managed"
    }
  ]
}"#;

        let value = openclaw::parse_openclaw_skills_json(stdout).unwrap();
        let skills = value
            .get("skills")
            .and_then(serde_json::Value::as_array)
            .unwrap();

        assert_eq!(skills.len(), 1);
        assert_eq!(
            skills[0].get("name").and_then(serde_json::Value::as_str),
            Some("xlsx")
        );
    }

    #[test]
    fn openclaw_json_without_skills_array_fails() {
        let stdout = r#"{"debug": true}"#;
        let result = openclaw::parse_openclaw_skills_json(stdout);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("did not contain a top-level skills array"));
    }

    #[cfg(unix)]
    #[test]
    fn openclaw_config_read_error_generates_warning() {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let openclaw_home = temp.path().join(".openclaw");
        fs::create_dir(&openclaw_home).unwrap();
        let config_path = openclaw_home.join("openclaw.json");
        fs::write(&config_path, "{}").unwrap();

        // Make file unreadable
        let mut perms = fs::metadata(&config_path).unwrap().permissions();
        perms.set_mode(0o000);
        fs::set_permissions(&config_path, perms).unwrap();

        let mut issues = Vec::new();
        let result = openclaw::read_openclaw_config(&openclaw_home, &mut issues);

        assert!(result.is_none());
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("Unable to read OpenClaw config"));

        // Restore permissions for cleanup
        let mut perms = fs::metadata(&config_path).unwrap().permissions();
        perms.set_mode(0o644);
        let _ = fs::set_permissions(&config_path, perms);
    }

    #[test]
    fn openclaw_filter_requires_matching_source_and_overrides_description() {
        let loaded_skills = HashMap::from([(
            "xlsx".to_string(),
            OpenClawLoadedSkill {
                description: Some("Loaded description".to_string()),
                source: OPENCLAW_SOURCE_MANAGED.to_string(),
            },
        )]);
        let mut skill = sample_skill("xlsx");

        assert!(should_include_skill(
            &mut skill,
            Some(&loaded_skills),
            Some(OPENCLAW_SOURCE_MANAGED),
        ));
        assert_eq!(skill.description, "Loaded description");
        assert!(!should_include_skill(
            &mut skill,
            Some(&loaded_skills),
            Some(OPENCLAW_SOURCE_BUNDLED),
        ));
    }

    #[test]
    fn invalid_openclaw_config_records_warning() {
        let openclaw_home = unique_temp_dir("invalid-openclaw-config");
        fs::create_dir_all(&openclaw_home).unwrap();
        fs::write(openclaw_home.join("openclaw.json"), "{ invalid json").unwrap();
        let mut issues = Vec::new();

        let config = read_openclaw_config(&openclaw_home, &mut issues);

        assert!(config.is_none());
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("Invalid OpenClaw config JSON"));
        let _ = fs::remove_dir_all(openclaw_home);
    }

    #[cfg(unix)]
    #[test]
    fn finds_openclaw_package_dir_from_nested_executable() {
        let package_dir = unique_temp_dir("openclaw-package");
        let bin_dir = package_dir.join("bin");
        fs::create_dir_all(package_dir.join("skills")).unwrap();
        fs::create_dir_all(&bin_dir).unwrap();
        let executable = bin_dir.join("openclaw");
        fs::write(&executable, "").unwrap();

        assert_eq!(
            openclaw::find_ancestor_with_skills(&executable, 6).as_deref(),
            Some(package_dir.as_path())
        );

        let _ = fs::remove_dir_all(package_dir);
    }

    #[test]
    fn dedup_preserves_same_name_different_real_path() {
        let skill_a = Skill {
            id: "hermes:/path/a".to_string(),
            name: "duplicate-name".to_string(),
            path: PathBuf::from("/path/a"),
            real_path: PathBuf::from("/real/a"),
            is_symlink: false,
            is_broken_link: false,
            description: "Skill A".to_string(),
            version: "1.0.0".to_string(),
            author: None,
            category: Some("test".to_string()),
            custom_tags: Vec::new(),
            requires_agent: None,
            source_agents: vec!["hermes".to_string()],
            source_agent_labels: vec!["Hermes".to_string()],
            body: "Body A".to_string(),
            warnings: Vec::new(),
        };

        let skill_b = Skill {
            id: "hermes:/path/b".to_string(),
            name: "duplicate-name".to_string(),
            path: PathBuf::from("/path/b"),
            real_path: PathBuf::from("/real/b"),
            is_symlink: false,
            is_broken_link: false,
            description: "Skill B".to_string(),
            version: "2.0.0".to_string(),
            author: None,
            category: Some("test".to_string()),
            custom_tags: Vec::new(),
            requires_agent: None,
            source_agents: vec!["hermes".to_string()],
            source_agent_labels: vec!["Hermes".to_string()],
            body: "Body B".to_string(),
            warnings: Vec::new(),
        };

        let deduped = dedup_skills(vec![skill_a, skill_b]);

        assert_eq!(deduped.len(), 2);
        assert_eq!(deduped[0].description, "Skill A");
        assert_eq!(deduped[1].description, "Skill B");
    }

    #[test]
    fn openclaw_cli_failure_excludes_openclaw_candidates() {
        let mut skill = sample_skill("test-skill");

        // Non-OpenClaw skills are always included
        assert!(should_include_skill(&mut skill, None, None));

        // OpenClaw skills require loaded_skills map
        assert!(!should_include_skill(
            &mut skill,
            None,
            Some(OPENCLAW_SOURCE_MANAGED)
        ));
    }

    #[test]
    fn expands_both_tilde_separator_styles() {
        let home = Path::new("/home/example");
        assert_eq!(openclaw::expand_tilde("~", home), home);
        assert_eq!(
            openclaw::expand_tilde("~/skills", home),
            home.join("skills")
        );
        assert_eq!(
            openclaw::expand_tilde(r"~\skills", home),
            home.join("skills")
        );
        assert_eq!(
            openclaw::expand_tilde("~someone/skills", home),
            PathBuf::from("~someone/skills")
        );
    }

    #[test]
    fn npm_global_root_precedes_platform_fallbacks() {
        let npm_root = unique_temp_dir("npm-root");
        let package = npm_root.join("openclaw");
        let fallback = unique_temp_dir("openclaw-fallback");
        fs::create_dir_all(package.join("skills")).unwrap();
        fs::create_dir_all(fallback.join("skills")).unwrap();

        assert_eq!(
            openclaw::find_package_in_roots(Some(npm_root.clone()), vec![fallback.clone()])
                .as_deref(),
            Some(package.as_path())
        );

        let _ = fs::remove_dir_all(npm_root);
        let _ = fs::remove_dir_all(fallback);
    }

    #[test]
    fn openclaw_managed_skills_source_is_correct() {
        let home = platform::home_dir().unwrap();
        let openclaw_home = home.join(".openclaw");
        let managed_path = openclaw_home.join("skills");

        let mut roots = Vec::new();
        let mut issues = Vec::new();
        push_openclaw_skill_roots(&mut roots, &home, &mut issues);

        let managed_root = roots
            .iter()
            .find(|r| platform::path_key(&r.path) == platform::path_key(&managed_path));

        if managed_path.exists() {
            assert!(managed_root.is_some());
            assert_eq!(
                managed_root.unwrap().openclaw_source,
                Some(OPENCLAW_SOURCE_MANAGED)
            );
        }
    }

    #[test]
    fn openclaw_workspace_generates_correct_sources() {
        let home = platform::home_dir().unwrap();
        let workspace = home.join(".openclaw/workspace");

        if !workspace.exists() {
            return;
        }

        let mut roots = Vec::new();
        let mut issues = Vec::new();
        push_openclaw_skill_roots(&mut roots, &home, &mut issues);

        let agents_project = roots
            .iter()
            .find(|r| r.path == workspace.join(".agents/skills"));
        let workspace_skills = roots.iter().find(|r| r.path == workspace.join("skills"));

        if workspace.join(".agents/skills").exists() {
            assert_eq!(
                agents_project.unwrap().openclaw_source,
                Some(OPENCLAW_SOURCE_AGENTS_PROJECT)
            );
        }

        if workspace.join("skills").exists() {
            assert_eq!(
                workspace_skills.unwrap().openclaw_source,
                Some(OPENCLAW_SOURCE_WORKSPACE)
            );
        }
    }

    #[test]
    fn missing_fields_generate_warnings() {
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let skill_dir = temp.path().join("minimal-skill");
        fs::create_dir(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "---\nname: minimal\n---\nBody").unwrap();

        let agent = AgentDir {
            key: "test".to_string(),
            label: "Test".to_string(),
            exists: true,
            path: temp.path().to_path_buf(),
            skill_count: 0,
        };

        let result = scan_skill_entry(&agent, temp.path(), skill_dir).unwrap();
        let skill = result.unwrap();

        assert_eq!(skill.version, "Unknown");
        assert_eq!(skill.description, "(No description)");
        assert_eq!(skill.category, Some("other".to_string()));
        assert!(skill
            .warnings
            .contains(&"Missing version field".to_string()));
        assert!(skill
            .warnings
            .contains(&"Missing description field".to_string()));
    }

    #[cfg(unix)]
    #[test]
    fn broken_symlink_creates_placeholder_skill() {
        use std::os::unix::fs::symlink;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let broken_link = temp.path().join("broken-skill");
        let _ = symlink("/nonexistent/target", &broken_link);

        let agent = AgentDir {
            key: "test".to_string(),
            label: "Test".to_string(),
            exists: true,
            path: temp.path().to_path_buf(),
            skill_count: 0,
        };

        let result = scan_skill_entry(&agent, temp.path(), broken_link).unwrap();
        let skill = result.unwrap();

        assert_eq!(skill.name, "broken-skill");
        assert!(skill.is_symlink);
        assert!(skill.is_broken_link);
        assert_eq!(skill.description, "Symlink target is missing");
        assert_eq!(skill.version, "Unknown");
        assert_eq!(skill.category, Some("other".to_string()));
        assert!(skill
            .warnings
            .contains(&"Symlink target not found".to_string()));
        assert!(skill.body.contains("symlink target no longer exists"));
    }

    #[cfg(windows)]
    #[test]
    fn broken_windows_directory_link_creates_placeholder_skill() {
        use std::os::windows::fs::symlink_dir;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let broken_link = temp.path().join("broken-skill");
        if symlink_dir(temp.path().join("missing-target"), &broken_link).is_err() {
            return;
        }

        let agent = AgentDir {
            key: "test".to_string(),
            label: "Test".to_string(),
            exists: true,
            path: temp.path().to_path_buf(),
            skill_count: 0,
        };
        let skill = scan_skill_entry(&agent, temp.path(), broken_link)
            .unwrap()
            .unwrap();
        assert!(skill.is_broken_link);
        assert!(skill.is_symlink);
    }

    #[cfg(windows)]
    #[test]
    fn windows_junction_resolves_to_real_skill_directory() {
        use std::process::Command;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let target = temp.path().join("包含 空格 target");
        let junction = temp.path().join("junction-skill");
        fs::create_dir_all(&target).unwrap();
        fs::write(
            target.join("SKILL.md"),
            "---\nname: junction-skill\n---\nBody",
        )
        .unwrap();

        let status = Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&junction)
            .arg(&target)
            .status()
            .unwrap();
        if !status.success() {
            return;
        }

        let agent = AgentDir {
            key: "test".to_string(),
            label: "Test".to_string(),
            exists: true,
            path: temp.path().to_path_buf(),
            skill_count: 0,
        };
        let skill = scan_skill_entry(&agent, temp.path(), junction.clone())
            .unwrap()
            .unwrap();
        assert_eq!(
            platform::path_key(&skill.real_path),
            platform::path_key(&target)
        );
        let _ = fs::remove_dir(&junction);
    }

    fn sample_skill(name: &str) -> Skill {
        Skill {
            id: format!("openclaw:/tmp/{name}"),
            name: name.to_string(),
            path: PathBuf::from(format!("/tmp/{name}")),
            real_path: PathBuf::from(format!("/tmp/{name}")),
            is_symlink: false,
            is_broken_link: false,
            description: "Original description".to_string(),
            version: "Unknown".to_string(),
            author: None,
            category: Some("other".to_string()),
            custom_tags: Vec::new(),
            requires_agent: None,
            source_agents: vec!["openclaw".to_string()],
            source_agent_labels: vec!["OpenClaw".to_string()],
            body: String::new(),
            warnings: Vec::new(),
        }
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
    }
}
