use crate::platform;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Default)]
pub struct ScannedPaths(Mutex<HashSet<String>>);

impl ScannedPaths {
    pub fn replace<'a>(&self, paths: impl IntoIterator<Item = &'a Path>) -> Result<(), String> {
        let mut scanned = self
            .0
            .lock()
            .map_err(|_| "Scanned path state is unavailable".to_string())?;
        scanned.clear();
        scanned.extend(paths.into_iter().map(platform::path_key));
        Ok(())
    }

    pub fn resolve(&self, path: &str) -> Result<PathBuf, String> {
        let requested = PathBuf::from(path);
        let normalized = platform::normalize_path(&requested);
        let canonical = normalized
            .canonicalize()
            .map_err(|_| format!("Path does not exist or is not accessible: {path}"))?;

        let scanned = self
            .0
            .lock()
            .map_err(|_| "Scanned path state is unavailable".to_string())?;
        if !scanned.contains(&platform::path_key(&canonical)) {
            return Err(format!(
                "Path was not found in the last scan results: {path}"
            ));
        }

        Ok(canonical)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn resolves_only_paths_from_the_latest_scan() {
        let root = TempDir::new().unwrap();
        let other = TempDir::new().unwrap();
        let state = ScannedPaths::default();

        state.replace([root.path()]).unwrap();

        assert_eq!(
            state.resolve(root.path().to_str().unwrap()).unwrap(),
            root.path().canonicalize().unwrap()
        );
        assert!(state.resolve(other.path().to_str().unwrap()).is_err());
    }
}
