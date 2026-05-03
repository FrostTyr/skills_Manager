use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct OpenClawLoadedSkill {
    pub description: Option<String>,
    pub source: String,
}

pub const OPENCLAW_SOURCE_WORKSPACE: &str = "openclaw-workspace";
pub const OPENCLAW_SOURCE_AGENTS_PROJECT: &str = "agents-skills-project";
pub const OPENCLAW_SOURCE_AGENTS_PERSONAL: &str = "agents-skills-personal";
pub const OPENCLAW_SOURCE_MANAGED: &str = "openclaw-managed";
pub const OPENCLAW_SOURCE_BUNDLED: &str = "openclaw-bundled";
pub const OPENCLAW_SOURCE_EXTRA: &str = "openclaw-extra";

pub fn openclaw_loaded_skills() -> Result<HashMap<String, OpenClawLoadedSkill>, String> {
    use std::process::Command;

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
                .map(ToString::to_string);
            Some((name, OpenClawLoadedSkill { description, source }))
        })
        .collect();

    Ok(skills)
}

pub fn parse_openclaw_skills_json(stdout: &str) -> Result<serde_json::Value, String> {
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

pub fn read_openclaw_config(
    openclaw_home: &std::path::Path,
    issues: &mut Vec<crate::models::ScanIssue>,
) -> Option<serde_json::Value> {
    use crate::models::{IssueLevel, ScanIssue};
    use std::fs;
    use std::io::ErrorKind;

    let config_path = openclaw_home.join("openclaw.json");
    if !config_path.exists() {
        return None;
    }

    let raw = match fs::read_to_string(&config_path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == ErrorKind::NotFound => return None,
        Err(error) => {
            issues.push(ScanIssue {
                path: config_path,
                level: IssueLevel::Warning,
                message: format!("Unable to read OpenClaw config: {error}"),
            });
            return None;
        }
    };

    match serde_json::from_str(&raw) {
        Ok(value) => Some(value),
        Err(error) => {
            issues.push(ScanIssue {
                path: config_path,
                level: IssueLevel::Warning,
                message: format!("Invalid OpenClaw config JSON: {error}"),
            });
            None
        }
    }
}

pub fn openclaw_workspace_paths(
    home: &std::path::Path,
    config: Option<&serde_json::Value>,
) -> Vec<PathBuf> {
    let mut workspaces = Vec::new();
    let openclaw_home = home.join(".openclaw");

    if let Some(config) = config {
        if let Some(default_ws) = config
            .get("agents")
            .and_then(|a| a.get("defaults"))
            .and_then(|d| d.get("workspace"))
            .and_then(serde_json::Value::as_str)
        {
            workspaces.push(expand_tilde(default_ws, home));
        }

        if let Some(entries) = config
            .get("agents")
            .and_then(|a| a.get("entries"))
            .and_then(serde_json::Value::as_object)
        {
            for entry in entries.values() {
                if let Some(ws) = entry.get("workspace").and_then(serde_json::Value::as_str) {
                    workspaces.push(expand_tilde(ws, home));
                }
            }
        }

        if let Some(list) = config
            .get("agents")
            .and_then(|a| a.get("list"))
            .and_then(serde_json::Value::as_array)
        {
            for entry in list {
                if let Some(ws) = entry.get("workspace").and_then(serde_json::Value::as_str) {
                    workspaces.push(expand_tilde(ws, home));
                }
            }
        }
    }

    workspaces.push(openclaw_home.join("agents/main/workspace"));
    workspaces.push(openclaw_home.join("workspace"));

    workspaces
}

pub fn extra_skill_dirs(config: Option<&serde_json::Value>, home: &std::path::Path) -> Vec<PathBuf> {
    let Some(config) = config else {
        return Vec::new();
    };

    config
        .get("skills")
        .and_then(|s| s.get("load"))
        .and_then(|l| l.get("extraDirs"))
        .and_then(serde_json::Value::as_array)
        .map(|dirs| {
            dirs.iter()
                .filter_map(serde_json::Value::as_str)
                .map(|dir| expand_tilde(dir, home))
                .collect()
        })
        .unwrap_or_default()
}

fn expand_tilde(path: &str, home: &std::path::Path) -> PathBuf {
    if path == "~" {
        home.to_path_buf()
    } else if let Some(rest) = path.strip_prefix("~/") {
        home.join(rest)
    } else {
        PathBuf::from(path)
    }
}

pub fn find_openclaw_package_dir(home: &std::path::Path) -> Option<PathBuf> {
    use super::utils::{find_executable, resolve_real_path};

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

pub fn find_ancestor_with_skills(path: &std::path::Path, max_depth: usize) -> Option<PathBuf> {
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
