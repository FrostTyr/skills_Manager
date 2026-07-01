use crate::models::AgentDir;
use crate::platform;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub key: &'static str,
    pub label: &'static str,
    pub relative_path: &'static str,
    pub executable_name: &'static str,
}

pub const AGENTS: &[AgentConfig] = &[
    AgentConfig {
        key: "hermes",
        label: "Hermes",
        relative_path: ".hermes/skills",
        executable_name: "hermes",
    },
    AgentConfig {
        key: "codex",
        label: "Codex",
        relative_path: ".codex/skills",
        executable_name: "codex",
    },
    AgentConfig {
        key: "claude",
        label: "Claude CLI",
        relative_path: ".claude/skills",
        executable_name: "claude",
    },
    AgentConfig {
        key: "openclaw",
        label: "OpenClaw",
        relative_path: ".openclaw/skills",
        executable_name: "openclaw",
    },
];

pub fn discover_agent_dirs(home: &Path) -> Vec<AgentDir> {
    AGENTS
        .iter()
        .map(|agent| {
            let path = home.join(agent.relative_path);
            AgentDir {
                key: agent.key.to_string(),
                label: agent.label.to_string(),
                exists: false,
                path,
                skill_count: 0,
            }
        })
        .collect()
}

pub fn is_agent_installed(home: &Path, key: &str) -> bool {
    let Some(agent) = AGENTS.iter().find(|agent| agent.key == key) else {
        return false;
    };

    if agent.key == "codex" && codex_data_or_desktop_app_exists(home) {
        return true;
    }

    platform::find_executable(agent.executable_name).is_some()
        || agent_cli_candidates(home, agent)
            .into_iter()
            .any(|path| path.is_file() || path.is_symlink())
}

pub fn update_agent_counts(agents: &mut [AgentDir], skills: &[crate::models::Skill]) {
    for agent in agents {
        agent.skill_count = skills
            .iter()
            .filter(|skill| skill.source_agents.contains(&agent.key))
            .count();
    }
}

#[cfg(unix)]
fn agent_cli_candidates(home: &Path, agent: &AgentConfig) -> Vec<PathBuf> {
    let executable = agent.executable_name;
    vec![
        home.join(".local/bin").join(executable),
        home.join(".npm-global/bin").join(executable),
        PathBuf::from("/opt/homebrew/bin").join(executable),
        PathBuf::from("/usr/local/bin").join(executable),
    ]
}

#[cfg(windows)]
fn agent_cli_candidates(home: &Path, agent: &AgentConfig) -> Vec<PathBuf> {
    let executable = agent.executable_name;
    let mut paths = Vec::new();
    if let Some(app_data) = std::env::var_os("APPDATA") {
        let npm = PathBuf::from(app_data).join("npm");
        paths.push(npm.join(format!("{executable}.exe")));
        paths.push(npm.join(format!("{executable}.cmd")));
    }
    paths.push(home.join(format!(r"AppData\Roaming\npm\{executable}.exe")));
    paths.push(home.join(format!(r"AppData\Roaming\npm\{executable}.cmd")));
    paths
}

fn codex_data_or_desktop_app_exists(home: &Path) -> bool {
    home.join(".codex/skills").is_dir()
        || home.join(".codex/plugins/cache").is_dir()
        || codex_desktop_candidates(home)
            .into_iter()
            .any(|path| path.is_file() || path.is_symlink())
}

#[cfg(target_os = "macos")]
fn codex_desktop_candidates(_home: &Path) -> Vec<PathBuf> {
    vec![PathBuf::from(
        "/Applications/Codex.app/Contents/Resources/codex",
    )]
}

#[cfg(all(unix, not(target_os = "macos")))]
fn codex_desktop_candidates(_home: &Path) -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(windows)]
fn codex_desktop_candidates(home: &Path) -> Vec<PathBuf> {
    let mut paths = codex_windows_desktop_paths(home, "Codex.exe");
    paths.extend(codex_windows_desktop_paths(home, r"resources\codex.exe"));
    paths
}

#[cfg(windows)]
fn codex_windows_desktop_paths(home: &Path, executable: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let relative_locations = [
        format!(r"Programs\Codex\{executable}"),
        format!(r"Programs\OpenAI Codex\{executable}"),
    ];

    if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
        let local_app_data = PathBuf::from(local_app_data);
        paths.extend(relative_locations.iter().map(|path| local_app_data.join(path)));
    }
    paths.extend(
        relative_locations
            .iter()
            .map(|path| home.join("AppData").join("Local").join(path)),
    );
    if let Some(program_files) = std::env::var_os("ProgramFiles") {
        let program_files = PathBuf::from(program_files);
        paths.push(program_files.join("Codex").join(executable));
        paths.push(program_files.join("OpenAI Codex").join(executable));
    }
    if let Some(program_files_x86) = std::env::var_os("ProgramFiles(x86)") {
        let program_files_x86 = PathBuf::from(program_files_x86);
        paths.push(program_files_x86.join("Codex").join(executable));
        paths.push(program_files_x86.join("OpenAI Codex").join(executable));
    }
    paths
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn discovers_all_default_agent_skill_directories() {
        let home = Path::new("/users/example");
        let agents = discover_agent_dirs(home);

        assert_eq!(agents.len(), 4);
        assert_eq!(agents[0].path, home.join(".hermes/skills"));
        assert_eq!(agents[1].path, home.join(".codex/skills"));
        assert_eq!(agents[2].path, home.join(".claude/skills"));
        assert_eq!(agents[3].path, home.join(".openclaw/skills"));
    }

    #[cfg(unix)]
    #[test]
    fn unix_agent_cli_candidates_cover_common_install_locations() {
        let home = Path::new("/users/example");
        let codex = AGENTS.iter().find(|agent| agent.key == "codex").unwrap();
        let candidates = agent_cli_candidates(home, codex);

        assert!(candidates.contains(&home.join(".npm-global/bin/codex")));
        assert!(candidates.contains(&PathBuf::from("/opt/homebrew/bin/codex")));
    }

    #[test]
    fn codex_is_installed_when_skill_data_exists_without_cli() {
        let home = TempDir::new().unwrap();
        fs::create_dir_all(home.path().join(".codex/skills")).unwrap();

        assert!(is_agent_installed(home.path(), "codex"));
    }
}
