use anyhow::{Result, Context};
use std::fs;
use std::path::Path;
use serde_yaml::Value;

/// Read metadata from an existing sidecar file
pub fn read_sidecar_metadata(sidecar_path: &Path) -> Result<serde_yaml::Mapping> {
    let content = fs::read_to_string(sidecar_path)
        .with_context(|| format!("Failed to read sidecar: {}", sidecar_path.display()))?;
    
    // Extract YAML frontmatter between --- markers
    let parts: Vec<&str> = content.split("---").collect();
    if parts.len() < 3 {
        return Err(anyhow::anyhow!("Invalid sidecar format - missing YAML frontmatter"));
    }
    
    let yaml_content = parts[1];
    let yaml_value: Value = serde_yaml::from_str(yaml_content)
        .context("Failed to parse YAML frontmatter")?;
    
    match yaml_value {
        Value::Mapping(map) => Ok(map),
        _ => Err(anyhow::anyhow!("YAML frontmatter is not a mapping")),
    }
}

/// Get rating from sidecar file
pub fn get_sidecar_rating(sidecar_path: &Path) -> Option<i64> {
    read_sidecar_metadata(sidecar_path)
        .ok()?
        .get("rating")
        .and_then(|v| v.as_i64())
}

/// Check if sidecar has AI metadata
pub fn has_ai_metadata(sidecar_path: &Path) -> bool {
    read_sidecar_metadata(sidecar_path)
        .ok()
        .map(|metadata| {
            metadata.contains_key("ai_description") || metadata.contains_key("ai_tags")
        })
        .unwrap_or(false)
}