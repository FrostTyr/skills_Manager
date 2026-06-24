use crate::models::{IssueLevel, ScanIssue};
use crate::platform;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub const SKILL_INDEX_FILENAME: &str = "SKILL.md";
const EXCLUDED_SKILL_DIRS: &[&str] = &[".git", ".github", ".hub"];

#[derive(Debug, Clone, Copy, Default)]
pub struct CollectOptions {
    pub include_system: bool,
}

pub fn collect_direct_skill_entries(
    root: &Path,
    issues: &mut Vec<ScanIssue>,
    enforce_root_boundary: bool,
) -> Vec<PathBuf> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(source) => {
            issues.push(ScanIssue {
                path: root.to_path_buf(),
                level: IssueLevel::Warning,
                message: format!("Unable to read Skill directory: {source}"),
            });
            return Vec::new();
        }
    };

    let mut skill_entries = Vec::new();
    for entry in entries {
        match entry {
            Ok(entry) => {
                let path = entry.path();
                if is_excluded_skill_dir(&path, CollectOptions::default()) {
                    continue;
                }
                if enforce_root_boundary && !is_path_within_root(root, &path) {
                    continue;
                }
                if path.join(SKILL_INDEX_FILENAME).is_file() {
                    skill_entries.push(path);
                }
            }
            Err(source) => issues.push(ScanIssue {
                path: root.to_path_buf(),
                level: IssueLevel::Warning,
                message: format!("Unable to read one directory entry: {source}"),
            }),
        }
    }

    skill_entries.sort_by(|a, b| {
        a.strip_prefix(root)
            .unwrap_or(a)
            .cmp(b.strip_prefix(root).unwrap_or(b))
    });
    skill_entries
}

pub fn collect_skill_entries(root: &Path, issues: &mut Vec<ScanIssue>) -> Vec<PathBuf> {
    collect_skill_entries_with_options(root, issues, CollectOptions::default())
}

pub fn collect_skill_entries_with_options(
    root: &Path,
    issues: &mut Vec<ScanIssue>,
    options: CollectOptions,
) -> Vec<PathBuf> {
    let mut entries = Vec::new();
    let mut visited = HashSet::new();
    collect_skill_entries_inner(root, root, issues, &mut entries, &mut visited, options);
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
    visited: &mut HashSet<String>,
    options: CollectOptions,
) {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(source) => {
            issues.push(ScanIssue {
                path: path.to_path_buf(),
                level: IssueLevel::Error,
                message: format!("Unable to read {}: {source}", path.display()),
            });
            return;
        }
    };

    let is_symlink = metadata.file_type().is_symlink();

    if is_symlink {
        let target_exists = path.exists();
        if !target_exists {
            entries.push(path.to_path_buf());
            return;
        }
    }

    let is_dir = metadata.is_dir() || (is_symlink && path.is_dir());
    if !is_dir && !is_symlink {
        return;
    }

    if path != root && is_excluded_skill_dir(path, options) {
        return;
    }

    let canonical = platform::path_key(path);
    if !visited.insert(canonical) {
        return;
    }

    if path.join(SKILL_INDEX_FILENAME).is_file() {
        entries.push(path.to_path_buf());
        return;
    }

    let read_dir = match fs::read_dir(path) {
        Ok(read_dir) => read_dir,
        Err(_) => return,
    };

    for entry in read_dir.flatten() {
        collect_skill_entries_inner(root, &entry.path(), issues, entries, visited, options);
    }
}

fn is_excluded_skill_dir(path: &Path, options: CollectOptions) -> bool {
    let name = path.file_name().and_then(|name| name.to_str());
    name.is_some_and(|name| {
        EXCLUDED_SKILL_DIRS.contains(&name) || (name == ".system" && !options.include_system)
    })
}

fn is_path_within_root(root: &Path, path: &Path) -> bool {
    platform::path_is_within(root, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn system_skills_are_included_only_when_requested() {
        let home = TempDir::new().unwrap();
        let root = home.path().join("skills");
        fs::create_dir_all(root.join(".system/skill-creator")).unwrap();
        fs::write(
            root.join(".system/skill-creator/SKILL.md"),
            "---\nname: skill-creator\n---",
        )
        .unwrap();

        let mut issues = Vec::new();
        assert!(collect_skill_entries(&root, &mut issues).is_empty());

        let entries = collect_skill_entries_with_options(
            &root,
            &mut issues,
            CollectOptions {
                include_system: true,
            },
        );
        assert_eq!(entries, vec![root.join(".system/skill-creator")]);
    }
}
