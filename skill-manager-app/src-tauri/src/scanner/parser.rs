use serde_yml::Value;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Unable to read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Invalid YAML frontmatter in {path}: {source}")]
    Yaml {
        path: PathBuf,
        #[source]
        source: serde_yml::Error,
    },
    #[error("Missing required field `name` in {path}")]
    MissingName { path: PathBuf },
}

#[derive(Debug, Default)]
pub struct SkillMetadata {
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub category: Option<String>,
    pub custom_tags: Vec<String>,
    pub requires_agent: Option<String>,
}

pub fn parse_skill_md(path: &Path, raw: &str) -> Result<(SkillMetadata, String), ParseError> {
    let Some(stripped) = raw.strip_prefix("---") else {
        return Ok((SkillMetadata::default(), raw.to_string()));
    };

    let stripped = stripped
        .strip_prefix("\r\n")
        .or_else(|| stripped.strip_prefix('\n'))
        .unwrap_or(stripped);

    let Some((frontmatter, body)) = split_frontmatter(stripped) else {
        return Ok((SkillMetadata::default(), raw.to_string()));
    };

    let value: Value = serde_yml::from_str(frontmatter).map_err(|source| ParseError::Yaml {
        path: path.to_path_buf(),
        source,
    })?;
    let map = value.as_mapping().cloned().unwrap_or_default();

    let get = |key: &str| map.get(Value::String(key.to_string()));

    Ok((
        SkillMetadata {
            name: get("name").and_then(value_to_string),
            version: get("version").and_then(value_to_string),
            description: get("description").and_then(value_to_string),
            author: get("author").and_then(value_to_string),
            category: get("category").and_then(value_to_string),
            custom_tags: get("tags").map(value_to_string_vec).unwrap_or_default(),
            requires_agent: get("requires_agent").and_then(value_to_string),
        },
        body.trim_start_matches(['\n', '\r']).to_string(),
    ))
}

pub fn split_frontmatter(input: &str) -> Option<(&str, &str)> {
    for marker in ["\n---\n", "\r\n---\r\n", "\n---\r\n", "\r\n---\n"] {
        if let Some(index) = input.find(marker) {
            let frontmatter = &input[..index];
            let body = &input[index + marker.len()..];
            return Some((frontmatter, body));
        }
    }

    None
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.trim().to_string()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
    .filter(|value| !value.is_empty())
}

fn value_to_string_vec(value: &Value) -> Vec<String> {
    match value {
        Value::Sequence(items) => items.iter().filter_map(value_to_string).collect(),
        Value::String(value) => value
            .split(',')
            .map(str::trim)
            .filter(|tag| !tag.is_empty())
            .map(ToOwned::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}
