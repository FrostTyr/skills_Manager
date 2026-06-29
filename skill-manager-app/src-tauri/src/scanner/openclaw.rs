use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::utils::background_command;

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
    use crate::platform;

    let home = platform::home_dir().ok_or_else(|| "home directory was not found".to_string())?;
    let executable = find_openclaw_executable(&home)
        .ok_or_else(|| "openclaw executable was not found".to_string())?;
    let mut command = background_command(&executable);
    configure_openclaw_command(&mut command, &home);
    let output = command
        .args(["skills", "list", "--eligible", "--json"])
        .output()
        .map_err(|error| format!("unable to start {}: {error}", executable.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("openclaw exited with status {}", output.status)
        } else {
            format!("openclaw exited with status {}: {stderr}", output.status)
        });
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

fn find_openclaw_executable(home: &Path) -> Option<PathBuf> {
    crate::platform::find_executable("openclaw").or_else(|| {
        platform_openclaw_executables(home)
            .into_iter()
            .find(|path| path.is_file() || path.is_symlink())
    })
}

fn configure_openclaw_command(command: &mut Command, home: &Path) {
    if let Ok(path) = std::env::join_paths(openclaw_path_entries(home)) {
        command.env("PATH", path);
    }
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

pub fn extra_skill_dirs(
    config: Option<&serde_json::Value>,
    home: &std::path::Path,
) -> Vec<PathBuf> {
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

pub(crate) fn expand_tilde(path: &str, home: &std::path::Path) -> PathBuf {
    let Some(rest) = path.strip_prefix('~') else {
        return PathBuf::from(path);
    };

    if rest.is_empty() {
        home.to_path_buf()
    } else if rest.starts_with('/') || rest.starts_with('\\') {
        home.join(rest.trim_start_matches(['/', '\\']))
    } else {
        PathBuf::from(path)
    }
}

pub fn find_openclaw_package_dir(home: &std::path::Path) -> Option<PathBuf> {
    find_openclaw_package_dir_with_npm_root(home, npm_global_root())
}

pub(crate) fn find_openclaw_package_dir_with_npm_root(
    home: &std::path::Path,
    npm_root: Option<PathBuf>,
) -> Option<PathBuf> {
    use super::utils::{find_executable, resolve_real_path};

    if let Some(path) = find_executable("openclaw") {
        let real_path = resolve_real_path(&path, path.is_symlink()).0;
        if let Some(package_dir) = find_ancestor_with_skills(&real_path, 6) {
            return Some(package_dir);
        }
    }

    find_package_in_roots(npm_root, platform_openclaw_fallbacks(home))
}

pub(crate) fn find_package_in_roots(
    npm_root: Option<PathBuf>,
    fallbacks: impl IntoIterator<Item = PathBuf>,
) -> Option<PathBuf> {
    npm_root
        .map(|root| root.join("openclaw"))
        .into_iter()
        .chain(fallbacks)
        .find(|path| path.join("skills").is_dir())
}

fn npm_global_root() -> Option<PathBuf> {
    use super::utils::find_executable;

    let npm = find_executable("npm")?;
    let output = background_command(npm).args(["root", "-g"]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8(output.stdout).ok()?;
    let path = path.trim();
    (!path.is_empty()).then(|| PathBuf::from(path))
}

#[cfg(unix)]
fn platform_openclaw_fallbacks(home: &std::path::Path) -> Vec<PathBuf> {
    vec![
        home.join(".npm-global/lib/node_modules/openclaw"),
        PathBuf::from("/opt/homebrew/lib/node_modules/openclaw"),
        PathBuf::from("/usr/local/lib/node_modules/openclaw"),
    ]
}

#[cfg(unix)]
fn platform_openclaw_executables(home: &Path) -> Vec<PathBuf> {
    vec![
        home.join(".npm-global/bin/openclaw"),
        PathBuf::from("/opt/homebrew/bin/openclaw"),
        PathBuf::from("/usr/local/bin/openclaw"),
    ]
}

#[cfg(unix)]
fn openclaw_path_entries(home: &Path) -> Vec<PathBuf> {
    let mut paths = vec![
        home.join(".npm-global/bin"),
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/bin"),
        PathBuf::from("/bin"),
        PathBuf::from("/usr/sbin"),
        PathBuf::from("/sbin"),
    ];
    paths.extend(
        std::env::var_os("PATH")
            .into_iter()
            .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>()),
    );
    dedup_paths(paths)
}

#[cfg(windows)]
fn platform_openclaw_fallbacks(_home: &std::path::Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(app_data) = std::env::var_os("APPDATA") {
        paths.push(PathBuf::from(app_data).join("npm/node_modules/openclaw"));
    }
    if let Some(program_files) = std::env::var_os("ProgramFiles") {
        paths.push(PathBuf::from(program_files).join("nodejs/node_modules/openclaw"));
    }
    paths
}

#[cfg(windows)]
fn platform_openclaw_executables(home: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(app_data) = std::env::var_os("APPDATA") {
        let npm = PathBuf::from(app_data).join("npm");
        paths.push(npm.join("openclaw.exe"));
        paths.push(npm.join("openclaw.cmd"));
    }
    paths.push(home.join(r"AppData\Roaming\npm\openclaw.exe"));
    paths.push(home.join(r"AppData\Roaming\npm\openclaw.cmd"));
    paths
}

#[cfg(windows)]
fn openclaw_path_entries(home: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(app_data) = std::env::var_os("APPDATA") {
        paths.push(PathBuf::from(app_data).join("npm"));
    }
    paths.push(home.join(r"AppData\Roaming\npm"));
    if let Some(program_files) = std::env::var_os("ProgramFiles") {
        paths.push(PathBuf::from(program_files).join("nodejs"));
    }
    paths.extend(
        std::env::var_os("PATH")
            .into_iter()
            .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>()),
    );
    dedup_paths(paths)
}

fn dedup_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = Vec::<OsString>::new();
    let mut deduped = Vec::new();
    for path in paths {
        let key = path.as_os_str().to_os_string();
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        deduped.push(path);
    }
    deduped
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
