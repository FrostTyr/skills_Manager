use crate::models::{IssueLevel, ScanIssue};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub const SKILL_INDEX_FILENAME: &str = "SKILL.md";
const EXCLUDED_SKILL_DIRS: &[&str] = &[".git", ".github", ".hub", ".system"];

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

    if path != root && is_excluded_skill_dir(path) {
        return;
    }

    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
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
        collect_skill_entries_inner(root, &entry.path(), issues, entries, visited);
    }
}

fn is_excluded_skill_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| EXCLUDED_SKILL_DIRS.contains(&name))
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
