use crate::models::{SkillFileContent, SkillFileEntry};
use crate::platform;
use std::fs;
use std::path::{Component, Path, PathBuf};

const MAX_SKILL_FILES: usize = 250;
const MAX_FILE_BYTES: u64 = 1_500_000;
const EXCLUDED_DIRS: &[&str] = &[".git", "node_modules", "target", "__pycache__"];

pub fn list(root: &Path) -> Result<Vec<SkillFileEntry>, String> {
    let mut files = Vec::new();
    collect(root, root, 0, &mut files)?;
    Ok(files)
}

pub fn read(root: &Path, relative_path: &str) -> Result<SkillFileContent, String> {
    let relative = validate_relative_path(relative_path)?;
    let requested = root.join(relative);
    let canonical = requested
        .canonicalize()
        .map_err(|_| format!("File does not exist or is not accessible: {relative_path}"))?;

    if !platform::path_is_within(root, &canonical) {
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
        relative_path: relative_path.to_string(),
        content,
        language: language_for_extension(&extension).to_string(),
        is_markdown: matches!(extension.as_str(), "md" | "mdx" | "markdown"),
        size: metadata.len(),
    })
}

fn collect(
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
        let is_directory = file_type.is_dir();
        output.push(SkillFileEntry {
            relative_path: relative.to_string_lossy().replace('\\', "/"),
            name: entry.file_name().to_string_lossy().to_string(),
            is_directory,
            depth,
        });

        if is_directory {
            collect(root, &path, depth + 1, output)?;
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

        let files = list(temp.path()).unwrap();

        assert!(files
            .iter()
            .any(|file| file.relative_path == "references/guide.md"));
        assert!(files.iter().any(|file| file.relative_path == "SKILL.md"));
        assert!(!files
            .iter()
            .any(|file| file.relative_path.contains("node_modules")));
    }
}
