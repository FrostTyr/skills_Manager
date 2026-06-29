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

#[cfg(test)]
mod tests {
    use super::*;

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
}
