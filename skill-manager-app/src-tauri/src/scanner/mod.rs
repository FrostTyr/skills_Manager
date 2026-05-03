use crate::models::{AgentDir, IssueLevel, ScanIssue, ScanResult, Skill};
use serde_yml::Value;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("Unable to resolve the current user home directory")]
    MissingHome,
    #[error("Unable to read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Invalid YAML frontmatter in {path}: {source}")]
    Yaml {
        path: PathBuf,
        #[source]
        source: serde_yml::Error,
    },
    #[error("Missing required field `name` in {path}")]
    MissingName { path: PathBuf },
}

#[derive(Debug, Clone)]
struct AgentConfig {
    key: &'static str,
    label: &'static str,
    relative_path: &'static str,
}

#[derive(Debug, Default)]
struct SkillMetadata {
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    author: Option<String>,
    category: Option<String>,
    custom_tags: Vec<String>,
    requires_agent: Option<String>,
}

#[derive(Debug, Clone)]
struct SkillRoot {
    path: PathBuf,
    openclaw_source: Option<&'static str>,
}

#[derive(Debug, Clone)]
struct OpenClawLoadedSkill {
    description: Option<String>,
    source: String,
}

const AGENTS: &[AgentConfig] = &[
    AgentConfig {
        key: "hermes",
        label: "Hermes",
        relative_path: ".hermes/skills",
    },
    AgentConfig {
        key: "codex",
        label: "Codex",
        relative_path: ".codex/skills",
    },
    AgentConfig {
        key: "claude",
        label: "Claude",
        relative_path: ".claude/skills",
    },
    AgentConfig {
        key: "openclaw",
        label: "OpenClaw",
        relative_path: ".openclaw/skills",
    },
];

const SKILL_INDEX_FILENAME: &str = "SKILL.md";
const EXCLUDED_SKILL_DIRS: &[&str] = &[".git", ".github", ".hub", ".system"];
const OPENCLAW_SOURCE_WORKSPACE: &str = "openclaw-workspace";
const OPENCLAW_SOURCE_AGENTS_PROJECT: &str = "agents-skills-project";
const OPENCLAW_SOURCE_AGENTS_PERSONAL: &str = "agents-skills-personal";
const OPENCLAW_SOURCE_MANAGED: &str = "openclaw-managed";
const OPENCLAW_SOURCE_BUNDLED: &str = "openclaw-bundled";
const OPENCLAW_SOURCE_EXTRA: &str = "openclaw-extra";

pub fn scan_skills() -> Result<ScanResult, ScanError> {
    let started_at = Instant::now();
    let home = std::env::var_os("HOME").ok_or(ScanError::MissingHome)?;
    let home = PathBuf::from(home);

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
                    issues.push(issue(
                        agent.path.clone(),
                        IssueLevel::Warning,
                        format!("Unable to read OpenClaw loaded skills: {message}"),
                    ));
                    None
                }
            }
        } else {
            None
        };

        for root in roots {
            let entries = if agent.key == "openclaw" {
                collect_direct_skill_entries(&root.path, &mut issues, true)
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

fn openclaw_loaded_skills() -> Result<HashMap<String, OpenClawLoadedSkill>, String> {
    let output = Command::new("openclaw")
        .args(["skills", "list", "--eligible", "--json"])
        .output()
        .map_err(|error| error.to_string())?;

    if !output.status.success() {
        return Err(format!("openclaw exited with status {}", output.status));
    }

    let stdout = String::from_utf8(output.stdout).map_err(|error| error.to_string())?;
    let value = parse_openclaw_skills_json(&stdout)?;

    let skills = value
        .get("skills")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "missing skills list".to_string())?
        .iter()
        .filter_map(|entry| {
            if entry.get("eligible").and_then(serde_json::Value::as_bool) != Some(true) {
                return None;
            }
            let name = entry.get("name")?.as_str()?.to_string();
            let source = entry.get("source")?.as_str()?.to_string();
            let description = entry
                .get("description")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned);
            Some((
                name,
                OpenClawLoadedSkill {
                    description,
                    source,
                },
            ))
        })
        .collect();

    Ok(skills)
}

fn parse_openclaw_skills_json(stdout: &str) -> Result<serde_json::Value, String> {
    let mut last_error = None;

    for (index, _) in stdout.char_indices().filter(|(_, ch)| *ch == '{').rev() {
        match serde_json::from_str::<serde_json::Value>(&stdout[index..]) {
            Ok(value)
                if value
                    .get("skills")
                    .and_then(serde_json::Value::as_array)
                    .is_some() =>
            {
                return Ok(value);
            }
            Ok(_) => {
                last_error = Some("JSON did not contain a top-level skills array".to_string());
            }
            Err(error) => {
                last_error = Some(error.to_string());
            }
        }
    }

    Err(last_error.unwrap_or_else(|| "openclaw did not print JSON".to_string()))
}

fn agent_skill_roots(home: &Path, agent: &AgentDir, issues: &mut Vec<ScanIssue>) -> Vec<SkillRoot> {
    if agent.key == "openclaw" {
        return openclaw_skill_roots(home, issues);
    }

    let mut roots = Vec::new();
    push_existing_root(&mut roots, agent.path.clone(), None);
    roots
}

fn openclaw_skill_roots(home: &Path, issues: &mut Vec<ScanIssue>) -> Vec<SkillRoot> {
    let mut roots = Vec::new();
    let openclaw_home = home.join(".openclaw");
    let config = read_openclaw_config(&openclaw_home, issues);

    // Official OpenClaw precedence: workspace-local skills first, then user/global
    // skills, bundled skills, and finally plugin/extra directories.
    for workspace in openclaw_workspace_paths(home, &openclaw_home, config.as_ref()) {
        push_existing_root(
            &mut roots,
            workspace.join(".agents/skills"),
            Some(OPENCLAW_SOURCE_AGENTS_PROJECT),
        );
        push_existing_root(
            &mut roots,
            workspace.join("skills"),
            Some(OPENCLAW_SOURCE_WORKSPACE),
        );
    }

    push_existing_root(
        &mut roots,
        home.join(".agents/skills"),
        Some(OPENCLAW_SOURCE_AGENTS_PERSONAL),
    );
    push_existing_root(
        &mut roots,
        openclaw_home.join("skills"),
        Some(OPENCLAW_SOURCE_MANAGED),
    );

    if let Some(package_dir) = find_openclaw_package_dir(home) {
        push_existing_root(
            &mut roots,
            package_dir.join("skills"),
            Some(OPENCLAW_SOURCE_BUNDLED),
        );
    }

    push_extension_skill_roots(&mut roots, &openclaw_home.join("extensions"));
    push_openclaw_extra_skill_roots(&mut roots, home, config.as_ref());

    roots
}

fn read_openclaw_config(
    openclaw_home: &Path,
    issues: &mut Vec<ScanIssue>,
) -> Option<serde_json::Value> {
    let config_path = openclaw_home.join("openclaw.json");
    let raw = match fs::read_to_string(&config_path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == ErrorKind::NotFound => return None,
        Err(error) => {
            issues.push(issue(
                config_path,
                IssueLevel::Warning,
                format!("Unable to read OpenClaw config: {error}"),
            ));
            return None;
        }
    };

    match serde_json::from_str(&raw) {
        Ok(value) => Some(value),
        Err(error) => {
            issues.push(issue(
                config_path,
                IssueLevel::Warning,
                format!("Invalid OpenClaw config JSON: {error}"),
            ));
            None
        }
    }
}

fn openclaw_workspace_paths(
    home: &Path,
    openclaw_home: &Path,
    config: Option<&serde_json::Value>,
) -> Vec<PathBuf> {
    let mut workspaces = Vec::new();

    if let Some(path) = config
        .and_then(|value| value.pointer("/agents/defaults/workspace"))
        .and_then(serde_json::Value::as_str)
    {
        push_unique_path(&mut workspaces, expand_home_path(path, home));
    }

    if let Some(entries) = config
        .and_then(|value| value.pointer("/agents/entries"))
        .and_then(serde_json::Value::as_object)
    {
        for entry in entries.values() {
            if let Some(path) = entry.get("workspace").and_then(serde_json::Value::as_str) {
                push_unique_path(&mut workspaces, expand_home_path(path, home));
            }
        }
    }

    if let Some(entries) = config
        .and_then(|value| value.pointer("/agents/list"))
        .and_then(serde_json::Value::as_array)
    {
        for entry in entries {
            if let Some(path) = entry.get("workspace").and_then(serde_json::Value::as_str) {
                push_unique_path(&mut workspaces, expand_home_path(path, home));
            }
        }
    }

    push_unique_path(&mut workspaces, openclaw_home.join("agents/main/workspace"));
    push_unique_path(&mut workspaces, openclaw_home.join("workspace"));

    workspaces
}

fn push_openclaw_extra_skill_roots(
    roots: &mut Vec<SkillRoot>,
    home: &Path,
    config: Option<&serde_json::Value>,
) {
    let Some(extra_dirs) = config
        .and_then(|value| value.pointer("/skills/load/extraDirs"))
        .and_then(serde_json::Value::as_array)
    else {
        return;
    };

    for dir in extra_dirs {
        if let Some(path) = dir.as_str() {
            push_existing_root(
                roots,
                expand_home_path(path, home),
                Some(OPENCLAW_SOURCE_EXTRA),
            );
        }
    }
}

fn expand_home_path(path: &str, home: &Path) -> PathBuf {
    if path == "~" {
        return home.to_path_buf();
    }

    if let Some(stripped) = path.strip_prefix("~/") {
        return home.join(stripped);
    }

    PathBuf::from(path)
}

fn push_unique_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    let key = normalize_path(&path);
    if paths.iter().any(|existing| normalize_path(existing) == key) {
        return;
    }

    paths.push(path);
}

fn normalize_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| {
        let expanded = expand_tilde_path(path);
        if expanded.is_relative() {
            env::current_dir()
                .map(|cwd| cwd.join(&expanded))
                .unwrap_or(expanded)
        } else {
            expanded
        }
    })
}

fn expand_tilde_path(path: &Path) -> PathBuf {
    let Some(path_string) = path.to_str() else {
        return path.to_path_buf();
    };
    let Some(home) = env::var_os("HOME").map(PathBuf::from) else {
        return path.to_path_buf();
    };

    if path_string == "~" {
        return home;
    }

    if let Some(stripped) = path_string.strip_prefix("~/") {
        return home.join(stripped);
    }

    path.to_path_buf()
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
        );
    }
}

fn find_openclaw_package_dir(home: &Path) -> Option<PathBuf> {
    if let Some(path) = find_executable("openclaw") {
        let real_path = resolve_real_path(&path, path.is_symlink()).0;
        if let Some(package_dir) = find_ancestor_with_skills(&real_path, 6) {
            return Some(package_dir);
        }
    }

    [
        home.join(".npm-global/lib/node_modules/openclaw"),
        PathBuf::from("/opt/homebrew/lib/node_modules/openclaw"),
        PathBuf::from("/usr/local/lib/node_modules/openclaw"),
    ]
    .into_iter()
    .find(|path| path.join("skills").is_dir())
}

fn find_ancestor_with_skills(path: &Path, max_depth: usize) -> Option<PathBuf> {
    let mut current = if path.is_dir() {
        Some(path)
    } else {
        path.parent()
    };

    for _ in 0..=max_depth {
        let dir = current?;
        if dir.join("skills").is_dir() {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }

    None
}

fn find_executable(name: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    env::split_paths(&path_var)
        .map(|path| path.join(name))
        .find(|path| path.is_file() || path.is_symlink())
}

fn push_existing_root(
    roots: &mut Vec<SkillRoot>,
    path: PathBuf,
    openclaw_source: Option<&'static str>,
) {
    if !path.is_dir() {
        return;
    }

    let key = normalize_path(&path);
    if roots.iter().any(|root| normalize_path(&root.path) == key) {
        return;
    }

    roots.push(SkillRoot {
        path,
        openclaw_source,
    });
}

fn update_agent_counts(agents: &mut [AgentDir], skills: &[Skill]) {
    for agent in agents {
        agent.skill_count = skills
            .iter()
            .filter(|skill| skill.source_agents.contains(&agent.key))
            .count();
    }
}

fn discover_agent_dirs(home: &Path) -> Vec<AgentDir> {
    AGENTS
        .iter()
        .map(|agent| {
            let path = home.join(agent.relative_path);
            AgentDir {
                key: agent.key.to_string(),
                label: agent.label.to_string(),
                exists: path.exists(),
                path,
                skill_count: 0,
            }
        })
        .collect()
}

fn collect_direct_skill_entries(
    root: &Path,
    issues: &mut Vec<ScanIssue>,
    enforce_root_boundary: bool,
) -> Vec<PathBuf> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(source) => {
            issues.push(issue(
                root.to_path_buf(),
                IssueLevel::Warning,
                format!("Unable to read Skill directory: {source}"),
            ));
            return Vec::new();
        }
    };

    let mut skill_entries = Vec::new();
    for entry in entries {
        match entry {
            Ok(entry) => {
                let path = entry.path();
                if is_excluded_skill_dir(&path) {
                    continue;
                }
                if enforce_root_boundary && !is_path_within_root(root, &path) {
                    continue;
                }
                if path.join(SKILL_INDEX_FILENAME).is_file() {
                    skill_entries.push(path);
                }
            }
            Err(source) => issues.push(issue(
                root.to_path_buf(),
                IssueLevel::Warning,
                format!("Unable to read one directory entry: {source}"),
            )),
        }
    }

    skill_entries.sort_by(|a, b| {
        a.strip_prefix(root)
            .unwrap_or(a)
            .cmp(b.strip_prefix(root).unwrap_or(b))
    });
    skill_entries
}

fn is_path_within_root(root: &Path, path: &Path) -> bool {
    let Ok(root) = root.canonicalize() else {
        return false;
    };
    let Ok(path) = path.canonicalize() else {
        return false;
    };

    path.starts_with(root)
}

fn collect_skill_entries(root: &Path, issues: &mut Vec<ScanIssue>) -> Vec<PathBuf> {
    let mut entries = Vec::new();
    let mut visited = HashSet::new();
    collect_skill_entries_inner(root, root, issues, &mut entries, &mut visited);
    entries.sort_by(|a, b| {
        a.strip_prefix(root)
            .unwrap_or(a)
            .cmp(b.strip_prefix(root).unwrap_or(b))
    });
    entries
}

fn collect_skill_entries_inner(
    root: &Path,
    path: &Path,
    issues: &mut Vec<ScanIssue>,
    entries: &mut Vec<PathBuf>,
    visited: &mut HashSet<PathBuf>,
) {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(source) => {
            issues.push(error_to_issue(ScanError::Read {
                path: path.to_path_buf(),
                source,
            }));
            return;
        }
    };

    let is_symlink = metadata.file_type().is_symlink();
    let is_dir = metadata.is_dir() || (is_symlink && path.is_dir());
    if !is_dir && !is_symlink {
        return;
    }

    if path != root && is_excluded_skill_dir(path) {
        return;
    }

    let (real_path, is_broken_link) = resolve_real_path(path, is_symlink);
    if is_broken_link {
        entries.push(path.to_path_buf());
        return;
    }

    if path.join(SKILL_INDEX_FILENAME).is_file() {
        entries.push(path.to_path_buf());
    }

    let visit_key = real_path.canonicalize().unwrap_or(real_path);
    if !visited.insert(visit_key) {
        return;
    }

    let child_entries = match fs::read_dir(path) {
        Ok(child_entries) => child_entries,
        Err(source) => {
            issues.push(issue(
                path.to_path_buf(),
                IssueLevel::Warning,
                format!("Unable to read Skill directory: {source}"),
            ));
            return;
        }
    };

    for child in child_entries {
        match child {
            Ok(child) => collect_skill_entries_inner(root, &child.path(), issues, entries, visited),
            Err(source) => issues.push(issue(
                path.to_path_buf(),
                IssueLevel::Warning,
                format!("Unable to read one directory entry: {source}"),
            )),
        }
    }
}

fn is_excluded_skill_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| EXCLUDED_SKILL_DIRS.contains(&name))
}

fn scan_skill_entry(
    agent: &AgentDir,
    source_root: &Path,
    path: PathBuf,
) -> Result<Option<Skill>, ScanError> {
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(source) => {
            return Err(ScanError::Read { path, source });
        }
    };

    let is_symlink = metadata.file_type().is_symlink();
    if !metadata.is_dir() && !is_symlink {
        return Ok(None);
    }

    let (real_path, is_broken_link) = resolve_real_path(&path, is_symlink);
    if is_broken_link {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("broken-symlink")
            .to_string();

        return Ok(Some(Skill {
            id: skill_id(&agent.key, &path),
            name,
            path,
            real_path,
            is_symlink,
            is_broken_link,
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
    if !skill_md_path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(&skill_md_path).map_err(|source| ScanError::Read {
        path: skill_md_path.clone(),
        source,
    })?;
    let (metadata, body) = parse_skill_md(&skill_md_path, &raw)?;

    let mut warnings = Vec::new();
    if metadata.version.is_none() {
        warnings.push("Missing version field".to_string());
    }
    if metadata.description.is_none() {
        warnings.push("Missing description field".to_string());
    }
    let name = metadata.name.ok_or_else(|| ScanError::MissingName {
        path: skill_md_path.clone(),
    })?;

    let inferred_category = category_from_path(source_root, &path);

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
        category: metadata
            .category
            .or(inferred_category)
            .or_else(|| Some("other".to_string())),
        custom_tags: metadata.custom_tags,
        requires_agent: metadata.requires_agent,
        source_agents: vec![agent.key.clone()],
        source_agent_labels: vec![agent.label.clone()],
        body,
        warnings,
    }))
}

fn category_from_path(root: &Path, skill_path: &Path) -> Option<String> {
    let relative = skill_path.strip_prefix(root).ok()?;
    let mut components = relative.components();
    let category = components.next()?.as_os_str().to_str()?.trim();

    if components.next().is_none() || category.is_empty() {
        None
    } else {
        Some(category.to_string())
    }
}

fn resolve_real_path(path: &Path, is_symlink: bool) -> (PathBuf, bool) {
    if !is_symlink {
        return (
            path.canonicalize().unwrap_or_else(|_| path.to_path_buf()),
            false,
        );
    }

    let target = match fs::read_link(path) {
        Ok(target) if target.is_absolute() => target,
        Ok(target) => path.parent().unwrap_or_else(|| Path::new("")).join(target),
        Err(_) => return (path.to_path_buf(), true),
    };

    if target.exists() {
        (target.canonicalize().unwrap_or(target), false)
    } else {
        (target, true)
    }
}

fn parse_skill_md(path: &Path, raw: &str) -> Result<(SkillMetadata, String), ScanError> {
    let Some(stripped) = raw.strip_prefix("---") else {
        return Ok((SkillMetadata::default(), raw.to_string()));
    };

    let stripped = stripped
        .strip_prefix("\r\n")
        .or_else(|| stripped.strip_prefix('\n'))
        .unwrap_or(stripped);

    let Some((frontmatter, body)) = split_frontmatter(stripped) else {
        return Ok((SkillMetadata::default(), raw.to_string()));
    };

    let value: Value = serde_yml::from_str(frontmatter).map_err(|source| ScanError::Yaml {
        path: path.to_path_buf(),
        source,
    })?;
    let map = value.as_mapping().cloned().unwrap_or_default();

    let get = |key: &str| map.get(Value::String(key.to_string()));

    Ok((
        SkillMetadata {
            name: get("name").and_then(value_to_string),
            version: get("version").and_then(value_to_string),
            description: get("description").and_then(value_to_string),
            author: get("author").and_then(value_to_string),
            category: get("category").and_then(value_to_string),
            custom_tags: get("tags").map(value_to_string_vec).unwrap_or_default(),
            requires_agent: get("requires_agent").and_then(value_to_string),
        },
        body.trim_start_matches(['\n', '\r']).to_string(),
    ))
}

fn split_frontmatter(input: &str) -> Option<(&str, &str)> {
    for marker in ["\n---\n", "\r\n---\r\n", "\n---\r\n", "\r\n---\n"] {
        if let Some(index) = input.find(marker) {
            let frontmatter = &input[..index];
            let body = &input[index + marker.len()..];
            return Some((frontmatter, body));
        }
    }

    None
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.trim().to_string()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
    .filter(|value| !value.is_empty())
}

fn value_to_string_vec(value: &Value) -> Vec<String> {
    match value {
        Value::Sequence(items) => items.iter().filter_map(value_to_string).collect(),
        Value::String(value) => value
            .split(',')
            .map(str::trim)
            .filter(|tag| !tag.is_empty())
            .map(ToOwned::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}

fn skill_id(source_agent: &str, path: &Path) -> String {
    format!("{source_agent}:{}", path.to_string_lossy())
}

/// Merge skills that resolve to the same real path (symlinks shared across agents).
fn dedup_skills(skills: Vec<Skill>) -> Vec<Skill> {
    let mut seen_paths: HashMap<PathBuf, usize> = HashMap::new();
    let mut deduped: Vec<Skill> = Vec::new();

    for skill in skills {
        // Broken symlinks are keyed by their declared path since real_path is unreliable.
        let path_key = if skill.is_broken_link {
            skill.path.clone()
        } else {
            skill.real_path.clone()
        };

        if let Some(&idx) = seen_paths.get(&path_key) {
            merge_skill_sources(&mut deduped[idx], &skill);
        } else {
            let idx = deduped.len();
            seen_paths.insert(path_key, idx);
            deduped.push(skill);
        }
    }

    deduped.sort_by(|a, b| {
        a.source_agents
            .first()
            .map(String::as_str)
            .unwrap_or("")
            .cmp(b.source_agents.first().map(String::as_str).unwrap_or(""))
            .then_with(|| a.name.cmp(&b.name))
    });

    deduped
}

fn merge_skill_sources(target: &mut Skill, source: &Skill) {
    for agent in &source.source_agents {
        if !target.source_agents.contains(agent) {
            target.source_agents.push(agent.clone());
        }
    }
    for label in &source.source_agent_labels {
        if !target.source_agent_labels.contains(label) {
            target.source_agent_labels.push(label.clone());
        }
    }
}

fn issue(path: PathBuf, level: IssueLevel, message: String) -> ScanIssue {
    ScanIssue {
        path,
        level,
        message,
    }
}

fn error_to_issue(error: ScanError) -> ScanIssue {
    match error {
        ScanError::MissingHome => issue(
            PathBuf::new(),
            IssueLevel::Error,
            "Unable to resolve the current user home directory".to_string(),
        ),
        ScanError::Read { path, source } => issue(
            path,
            IssueLevel::Error,
            format!("Unable to read Skill file: {source}"),
        ),
        ScanError::Yaml { path, source } => issue(
            path,
            IssueLevel::Error,
            format!("Invalid YAML frontmatter: {source}"),
        ),
        ScanError::MissingName { path } => issue(
            path,
            IssueLevel::Error,
            "Missing required field `name`".to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_standard_skill_frontmatter() {
        let raw = r#"---
name: pdf-extractor
version: 1.2.3
description: Extract PDFs
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
        let (frontmatter, body) = split_frontmatter("name: x\n---\n# Body").unwrap();
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

        let value = parse_openclaw_skills_json(stdout).unwrap();
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

    #[test]
    fn finds_openclaw_package_dir_from_nested_executable() {
        let package_dir = unique_temp_dir("openclaw-package");
        let bin_dir = package_dir.join("bin");
        fs::create_dir_all(package_dir.join("skills")).unwrap();
        fs::create_dir_all(&bin_dir).unwrap();
        let executable = bin_dir.join("openclaw");
        fs::write(&executable, "").unwrap();

        assert_eq!(
            find_ancestor_with_skills(&executable, 6).as_deref(),
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
        assert!(!should_include_skill(&mut skill, None, Some(OPENCLAW_SOURCE_MANAGED)));
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
        env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
    }
}
