#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use std::path::{Component, Path, PathBuf};

#[cfg(unix)]
pub use unix::{available_apps, find_executable, home_dir, open_in_app, reveal_in_file_manager};
#[cfg(windows)]
pub use windows::{available_apps, find_executable, home_dir, open_in_app, reveal_in_file_manager};

fn absolute_fallback(path: &Path) -> PathBuf {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(path)
    };

    let mut cleaned = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                cleaned.pop();
            }
            _ => cleaned.push(component.as_os_str()),
        }
    }
    cleaned
}

pub fn normalize_path(path: &Path) -> PathBuf {
    path.canonicalize()
        .unwrap_or_else(|_| absolute_fallback(path))
}

#[cfg(unix)]
pub fn path_key(path: &Path) -> String {
    normalize_path(path).to_string_lossy().into_owned()
}

#[cfg(windows)]
pub fn path_key(path: &Path) -> String {
    windows::windows_path_key(&normalize_path(path))
}

#[cfg(unix)]
pub fn path_is_within(root: &Path, candidate: &Path) -> bool {
    normalize_path(candidate).starts_with(normalize_path(root))
}

#[cfg(windows)]
pub fn path_is_within(root: &Path, candidate: &Path) -> bool {
    let root = path_key(root);
    let candidate = path_key(candidate);
    candidate == root
        || candidate
            .strip_prefix(&root)
            .is_some_and(|suffix| suffix.starts_with('\\'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_fallback_removes_parent_components() {
        let cleaned = absolute_fallback(Path::new("alpha/beta/../gamma"));
        assert!(cleaned.ends_with(Path::new("alpha/gamma")));
    }

    #[cfg(windows)]
    #[test]
    fn windows_boundary_checks_ignore_case_but_reject_prefix_collisions() {
        let root = Path::new(r"C:\Users\Test\Skills");
        assert!(path_is_within(
            root,
            Path::new(r"c:\users\test\skills\demo")
        ));
        assert!(!path_is_within(
            root,
            Path::new(r"C:\Users\Test\Skills-Escaped")
        ));
    }
}
