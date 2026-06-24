use crate::platform;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

#[derive(Debug, Deserialize)]
struct PluginList {
    #[serde(default)]
    installed: Vec<InstalledPlugin>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstalledPlugin {
    name: String,
    marketplace_name: String,
    version: String,
    installed: bool,
    enabled: bool,
    source: PluginSource,
}

#[derive(Debug, Deserialize)]
struct PluginSource {
    path: PathBuf,
}

pub fn plugin_skill_roots(home: &Path) -> Vec<PathBuf> {
    installed_plugin_skill_roots(home).unwrap_or_else(|| cached_plugin_skill_roots(home))
}

fn installed_plugin_skill_roots(home: &Path) -> Option<Vec<PathBuf>> {
    let executable = find_codex_executable(home)?;
    let output = Command::new(executable)
        .args(["plugin", "list", "--json"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let value = parse_json_after_log_noise(&String::from_utf8_lossy(&output.stdout))?;
    let plugins: PluginList = serde_json::from_value(value).ok()?;
    Some(
        plugins
            .installed
            .into_iter()
            .filter(|plugin| plugin.installed && plugin.enabled)
            .filter_map(|plugin| plugin_skill_root(home, &plugin))
            .collect(),
    )
}

fn plugin_skill_root(home: &Path, plugin: &InstalledPlugin) -> Option<PathBuf> {
    let cached = home
        .join(".codex/plugins/cache")
        .join(&plugin.marketplace_name)
        .join(&plugin.name)
        .join(&plugin.version)
        .join("skills");
    if cached.is_dir() {
        return Some(cached);
    }

    let source = plugin.source.path.join("skills");
    source.is_dir().then_some(source)
}

fn cached_plugin_skill_roots(home: &Path) -> Vec<PathBuf> {
    let cache = home.join(".codex/plugins/cache");
    let mut newest_by_plugin: HashMap<String, (SystemTime, PathBuf)> = HashMap::new();

    for marketplace in read_directories(&cache) {
        for plugin in read_directories(&marketplace) {
            let Some(plugin_name) = plugin
                .file_name()
                .and_then(|name| name.to_str())
                .map(ToString::to_string)
            else {
                continue;
            };

            for version in read_directories(&plugin) {
                let skills = version.join("skills");
                if !skills.is_dir() {
                    continue;
                }
                let modified = version
                    .metadata()
                    .and_then(|metadata| metadata.modified())
                    .unwrap_or(SystemTime::UNIX_EPOCH);
                let current = newest_by_plugin.get(&plugin_name);
                if current.is_none_or(|(current_modified, _)| modified > *current_modified) {
                    newest_by_plugin.insert(plugin_name.clone(), (modified, skills));
                }
            }
        }
    }

    let mut roots = newest_by_plugin
        .into_values()
        .map(|(_, path)| path)
        .collect::<Vec<_>>();
    roots.sort();
    roots
}

fn read_directories(root: &Path) -> Vec<PathBuf> {
    fs::read_dir(root)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect()
}

fn find_codex_executable(home: &Path) -> Option<PathBuf> {
    platform::find_executable("codex").or_else(|| {
        codex_fallback_executables(home)
            .into_iter()
            .find(|path| path.is_file() || path.is_symlink())
    })
}

#[cfg(unix)]
fn codex_fallback_executables(home: &Path) -> Vec<PathBuf> {
    vec![
        home.join(".npm-global/bin/codex"),
        PathBuf::from("/opt/homebrew/bin/codex"),
        PathBuf::from("/usr/local/bin/codex"),
    ]
}

#[cfg(windows)]
fn codex_fallback_executables(home: &Path) -> Vec<PathBuf> {
    vec![
        home.join(r"AppData\Roaming\npm\codex.exe"),
        home.join(r"AppData\Roaming\npm\codex.cmd"),
    ]
}

fn parse_json_after_log_noise(stdout: &str) -> Option<serde_json::Value> {
    stdout.char_indices().find_map(|(index, character)| {
        (character == '{')
            .then(|| serde_json::from_str(&stdout[index..]).ok())
            .flatten()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn parses_plugin_list_after_cli_warning() {
        let value = parse_json_after_log_noise(
            "warning: aliases unavailable\n{\"installed\":[],\"available\":[]}",
        )
        .unwrap();

        assert!(value["installed"].as_array().unwrap().is_empty());
    }

    #[test]
    fn resolves_enabled_plugin_to_cross_platform_home_cache() {
        let home = TempDir::new().unwrap();
        let skills = home
            .path()
            .join(".codex/plugins/cache/market/plugin/1.0.0/skills");
        fs::create_dir_all(&skills).unwrap();
        let plugin = InstalledPlugin {
            name: "plugin".to_string(),
            marketplace_name: "market".to_string(),
            version: "1.0.0".to_string(),
            installed: true,
            enabled: true,
            source: PluginSource {
                path: PathBuf::from("unused"),
            },
        };

        assert_eq!(plugin_skill_root(home.path(), &plugin), Some(skills));
    }

    #[test]
    fn cache_fallback_keeps_only_newest_copy_of_a_plugin() {
        let home = TempDir::new().unwrap();
        let older = home
            .path()
            .join(".codex/plugins/cache/market-a/figma/1/skills");
        let newer = home
            .path()
            .join(".codex/plugins/cache/market-b/figma/2/skills");
        fs::create_dir_all(&older).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        fs::create_dir_all(&newer).unwrap();

        let roots = cached_plugin_skill_roots(home.path());

        assert_eq!(roots, vec![newer]);
    }
}
