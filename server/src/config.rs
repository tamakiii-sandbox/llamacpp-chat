use std::collections::HashMap;
use serde::Deserialize;
use tokio::fs;
use anyhow::Result;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub models: HashMap<String, ModelConfig>,
    pub default: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ModelConfig {
    pub path: String,
    pub args: Vec<String>,
}

pub async fn load_config(path: &str) -> Result<AppConfig> {
    let content = fs::read_to_string(path).await?;
    let config: AppConfig = serde_json::from_str(&content)?;
    Ok(config)
}
