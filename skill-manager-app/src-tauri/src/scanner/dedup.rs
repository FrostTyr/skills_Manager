use crate::models::Skill;
use std::collections::HashMap;
use std::path::PathBuf;

pub fn dedup_skills(skills: Vec<Skill>) -> Vec<Skill> {
    let mut seen_paths: HashMap<PathBuf, usize> = HashMap::new();
    let mut deduped: Vec<Skill> = Vec::new();

    for skill in skills {
        let path_key = if skill.is_broken_link {
            skill.path.clone()
        } else {
            skill.real_path.clone()
        };

        if let Some(&idx) = seen_paths.get(&path_key) {
            merge_skill_sources(&mut deduped[idx], &skill);
        } else {
            let idx = deduped.len();
            seen_paths.insert(path_key, idx);
            deduped.push(skill);
        }
    }

    deduped.sort_by(|a, b| {
        a.source_agents
            .first()
            .map(String::as_str)
            .unwrap_or("")
            .cmp(b.source_agents.first().map(String::as_str).unwrap_or(""))
            .then_with(|| a.name.cmp(&b.name))
    });

    deduped
}

fn merge_skill_sources(target: &mut Skill, source: &Skill) {
    for agent in &source.source_agents {
        if !target.source_agents.contains(agent) {
            target.source_agents.push(agent.clone());
        }
    }
    for label in &source.source_agent_labels {
        if !target.source_agent_labels.contains(label) {
            target.source_agent_labels.push(label.clone());
        }
    }
}
