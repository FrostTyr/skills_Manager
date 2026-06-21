use crate::models::{AppOption, ScanResult, SkillFileContent, SkillFileEntry};
use crate::scanner;
use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use tauri::State;

pub struct ScannedPaths(pub Mutex<HashSet<PathBuf>>);

const MAX_SKILL_FILES: usize = 250;
const MAX_FILE_BYTES: u64 = 1_500_000;
const EXCLUDED_DIRS: &[&str] = &[".git", "node_modules", "target", "__pycache__"];

#[tauri::command]
pub async fn scan_skills(state: State<'_, ScannedPaths>) -> Result<ScanResult, String> {
    let result = tauri::async_runtime::spawn_blocking(scanner::scan_skills)
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())?;

    {
        let mut paths = state.0.lock().unwrap();
        paths.clear();
        for skill in &result.skills {
            paths.insert(skill.path.clone());
            if skill.path != skill.real_path {
                paths.insert(skill.real_path.clone());
            }
        }
    }

    Ok(result)
}

#[tauri::command]
pub async fn reveal_in_finder(path: String, state: State<'_, ScannedPaths>) -> Result<(), String> {
    let path = validate_scanned_path(path, &state)?;
    Command::new("open")
        .arg("-R")
        .arg(path)
        .status()
        .map_err(|error| format!("Unable to open Finder: {error}"))?
        .success()
        .then_some(())
        .ok_or_else(|| "Finder returned a non-zero exit status".to_string())
}

#[tauri::command]
pub async fn open_in_editor(
    path: String,
    editor: String,
    state: State<'_, ScannedPaths>,
) -> Result<(), String> {
    let path = validate_scanned_path(path, &state)?;
    let editor_name = match editor.as_str() {
        "cursor" => "Cursor",
        "vscode" => "Visual Studio Code",
        _ => return Err("Unsupported editor".to_string()),
    };

    Command::new("open")
        .arg("-a")
        .arg(editor_name)
        .arg(path)
        .status()
        .map_err(|error| format!("Unable to open editor: {error}"))?
        .success()
        .then_some(())
        .ok_or_else(|| format!("{editor_name} returned a non-zero exit status"))
}

#[tauri::command]
pub async fn list_skill_files(
    path: String,
    state: State<'_, ScannedPaths>,
) -> Result<Vec<SkillFileEntry>, String> {
    let root = validate_scanned_path(path, &state)?;
    let mut files = Vec::new();
    collect_skill_files(&root, &root, 0, &mut files)?;
    Ok(files)
}

#[tauri::command]
pub async fn read_skill_file(
    path: String,
    relative_path: String,
    state: State<'_, ScannedPaths>,
) -> Result<SkillFileContent, String> {
    let root = validate_scanned_path(path, &state)?;
    let relative = validate_relative_path(&relative_path)?;
    let requested = root.join(relative);
    let canonical = requested
        .canonicalize()
        .map_err(|_| format!("File does not exist or is not accessible: {relative_path}"))?;

    if !canonical.starts_with(&root) {
        return Err("Requested file is outside the scanned Skill directory".to_string());
    }

    let metadata = canonical
        .metadata()
        .map_err(|error| format!("Unable to inspect file: {error}"))?;
    if !metadata.is_file() {
        return Err("Requested path is not a file".to_string());
    }
    if metadata.len() > MAX_FILE_BYTES {
        return Err(format!(
            "File is too large to preview ({:.1} MB)",
            metadata.len() as f64 / 1_000_000.0
        ));
    }

    let bytes = fs::read(&canonical).map_err(|error| format!("Unable to read file: {error}"))?;
    if bytes.iter().take(8_192).any(|byte| *byte == 0) {
        return Err("Binary files cannot be previewed".to_string());
    }

    let content = String::from_utf8(bytes)
        .map_err(|_| "This file is not valid UTF-8 and cannot be previewed".to_string())?;
    let extension = canonical
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    Ok(SkillFileContent {
        relative_path,
        content,
        language: language_for_extension(&extension).to_string(),
        is_markdown: matches!(extension.as_str(), "md" | "mdx" | "markdown"),
        size: metadata.len(),
    })
}

#[tauri::command]
pub async fn available_apps() -> Vec<AppOption> {
    supported_apps()
        .iter()
        .filter(|app| app.key == "finder" || app_is_installed(app.app_name))
        .map(|app| AppOption {
            key: app.key.to_string(),
            label: app.label.to_string(),
        })
        .collect()
}

#[tauri::command]
pub async fn open_in_app(
    path: String,
    app: String,
    state: State<'_, ScannedPaths>,
) -> Result<(), String> {
    let path = validate_scanned_path(path, &state)?;
    let option = supported_apps()
        .iter()
        .find(|option| option.key == app)
        .ok_or_else(|| "Unsupported application".to_string())?;

    if option.key != "finder" && !app_is_installed(option.app_name) {
        return Err(format!("{} is not installed", option.label));
    }

    Command::new("open")
        .arg("-a")
        .arg(option.app_name)
        .arg(path)
        .status()
        .map_err(|error| format!("Unable to open {}: {error}", option.label))?
        .success()
        .then_some(())
        .ok_or_else(|| format!("{} returned a non-zero exit status", option.label))
}

fn validate_scanned_path(path: String, state: &State<'_, ScannedPaths>) -> Result<PathBuf, String> {
    let requested = PathBuf::from(&path);

    let canonical = requested
        .canonicalize()
        .map_err(|_| format!("Path does not exist or is not accessible: {path}"))?;

    let paths = state.0.lock().unwrap();
    if !paths.contains(&requested) && !paths.contains(&canonical) {
        return Err(format!(
            "Path was not found in the last scan results: {path}"
        ));
    }

    Ok(canonical)
}

fn collect_skill_files(
    root: &Path,
    directory: &Path,
    depth: usize,
    output: &mut Vec<SkillFileEntry>,
) -> Result<(), String> {
    if output.len() >= MAX_SKILL_FILES {
        return Ok(());
    }

    let entries = fs::read_dir(directory)
        .map_err(|error| format!("Unable to read Skill directory: {error}"))?;
    let mut entries = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| name != ".DS_Store" && !EXCLUDED_DIRS.contains(&name))
        })
        .collect::<Vec<_>>();

    entries.sort_by(|a, b| {
        let a_dir = a.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
        let b_dir = b.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
        b_dir.cmp(&a_dir).then_with(|| {
            a.file_name()
                .to_string_lossy()
                .cmp(&b.file_name().to_string_lossy())
        })
    });

    for entry in entries {
        if output.len() >= MAX_SKILL_FILES {
            break;
        }

        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(_) => continue,
        };
        if file_type.is_symlink() {
            continue;
        }

        let path = entry.path();
        let relative = match path.strip_prefix(root) {
            Ok(relative) => relative,
            Err(_) => continue,
        };
        let relative_path = relative.to_string_lossy().replace('\\', "/");
        let name = entry.file_name().to_string_lossy().to_string();
        let is_directory = file_type.is_dir();

        output.push(SkillFileEntry {
            relative_path,
            name,
            is_directory,
            depth,
        });

        if is_directory {
            collect_skill_files(root, &path, depth + 1, output)?;
        }
    }

    Ok(())
}

fn validate_relative_path(path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(path);
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err("Invalid relative file path".to_string());
    }
    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("Invalid relative file path".to_string());
    }
    Ok(path)
}

fn language_for_extension(extension: &str) -> &str {
    match extension {
        "md" | "mdx" | "markdown" => "markdown",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" | "mjs" | "cjs" => "javascript",
        "rs" => "rust",
        "py" => "python",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "sh" | "bash" | "zsh" => "shell",
        "css" => "css",
        "html" | "htm" => "html",
        "vue" | "xml" | "svg" => "xml",
        "sql" => "sql",
        _ => "plaintext",
    }
}

struct SupportedApp {
    key: &'static str,
    label: &'static str,
    app_name: &'static str,
}

fn supported_apps() -> &'static [SupportedApp] {
    &[
        SupportedApp {
            key: "cursor",
            label: "Cursor",
            app_name: "Cursor",
        },
        SupportedApp {
            key: "vscode",
            label: "VS Code",
            app_name: "Visual Studio Code",
        },
        SupportedApp {
            key: "trae",
            label: "Trae",
            app_name: "Trae",
        },
        SupportedApp {
            key: "sublime",
            label: "Sublime Text",
            app_name: "Sublime Text",
        },
        SupportedApp {
            key: "notepadpp",
            label: "Notepad++",
            app_name: "Notepad++",
        },
        SupportedApp {
            key: "warp",
            label: "Warp",
            app_name: "Warp",
        },
        SupportedApp {
            key: "ghostty",
            label: "Ghostty",
            app_name: "Ghostty",
        },
        SupportedApp {
            key: "terminal",
            label: "Terminal",
            app_name: "Terminal",
        },
        SupportedApp {
            key: "finder",
            label: "Finder",
            app_name: "Finder",
        },
    ]
}

fn app_is_installed(app_name: &str) -> bool {
    Command::new("open")
        .arg("-Ra")
        .arg(app_name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn rejects_paths_that_escape_the_skill_root() {
        assert!(validate_relative_path("../secret.txt").is_err());
        assert!(validate_relative_path("/tmp/secret.txt").is_err());
        assert!(validate_relative_path("references/guide.md").is_ok());
    }

    #[test]
    fn collects_nested_files_and_skips_generated_directories() {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join("references")).unwrap();
        fs::create_dir_all(temp.path().join("node_modules/pkg")).unwrap();
        fs::write(temp.path().join("SKILL.md"), "# Skill").unwrap();
        fs::write(temp.path().join("references/guide.md"), "# Guide").unwrap();
        fs::write(temp.path().join("node_modules/pkg/index.js"), "ignored").unwrap();

        let mut files = Vec::new();
        collect_skill_files(temp.path(), temp.path(), 0, &mut files).unwrap();

        assert!(files
            .iter()
            .any(|file| file.relative_path == "references/guide.md"));
        assert!(files.iter().any(|file| file.relative_path == "SKILL.md"));
        assert!(!files
            .iter()
            .any(|file| file.relative_path.contains("node_modules")));
    }
}
