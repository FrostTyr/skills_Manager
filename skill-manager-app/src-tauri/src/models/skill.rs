use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub real_path: PathBuf,
    pub is_symlink: bool,
    pub is_broken_link: bool,
    pub description: String,
    pub version: String,
    pub author: Option<String>,
    pub category: Option<String>,
    pub custom_tags: Vec<String>,
    pub requires_agent: Option<String>,
    pub source_agents: Vec<String>,
    pub source_agent_labels: Vec<String>,
    pub body: String,
    pub warnings: Vec<String>,
}
