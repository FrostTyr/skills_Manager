use crate::models::AgentDir;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub key: &'static str,
    pub label: &'static str,
    pub relative_path: &'static str,
}

pub const AGENTS: &[AgentConfig] = &[
    AgentConfig {
        key: "hermes",
        label: "Hermes",
        relative_path: ".hermes/skills",
    },
    AgentConfig {
        key: "codex",
        label: "Codex",
        relative_path: ".codex/skills",
    },
    AgentConfig {
        key: "claude",
        label: "Claude CLI",
        relative_path: ".claude/skills",
    },
    AgentConfig {
        key: "openclaw",
        label: "OpenClaw",
        relative_path: ".openclaw/skills",
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
                exists: path.exists(),
                path,
                skill_count: 0,
            }
        })
        .collect()
}

pub fn update_agent_counts(agents: &mut [AgentDir], skills: &[crate::models::Skill]) {
    for agent in agents {
        agent.skill_count = skills
            .iter()
            .filter(|skill| skill.source_agents.contains(&agent.key))
            .count();
    }
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
}
