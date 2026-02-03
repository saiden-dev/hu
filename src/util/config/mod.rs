use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[cfg(test)]
mod tests;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Credentials {
    #[serde(default)]
    pub github: Option<GithubCredentials>,
    #[serde(default)]
    pub jira: Option<JiraCredentials>,
    #[serde(default)]
    pub brave: Option<BraveCredentials>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BraveCredentials {
    pub api_key: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GithubCredentials {
    pub token: String,
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct JiraCredentials {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64, // Unix timestamp
    pub cloud_id: String,
    pub site_url: String,
}

/// Returns the config directory path
pub fn config_dir() -> Result<PathBuf> {
    let proj_dirs = directories::ProjectDirs::from("", "", "hu")
        .context("Could not determine config directory")?;
    Ok(proj_dirs.config_dir().to_path_buf())
}

/// Returns the path to credentials.toml
fn credentials_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("credentials.toml"))
}

/// Load credentials from config dir
pub fn load_credentials() -> Result<Credentials> {
    let path = credentials_path()?;
    load_credentials_from(&path)
}

/// Load credentials from a specific path (testable)
pub fn load_credentials_from(path: &PathBuf) -> Result<Credentials> {
    if !path.exists() {
        return Ok(Credentials::default());
    }

    let contents =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;

    toml::from_str(&contents).with_context(|| format!("Failed to parse {}", path.display()))
}

/// Save credentials to config dir
pub fn save_credentials(creds: &Credentials) -> Result<()> {
    let path = credentials_path()?;
    save_credentials_to(creds, &path)
}

/// Save credentials to a specific path (testable)
pub fn save_credentials_to(creds: &Credentials, path: &PathBuf) -> Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)
            .with_context(|| format!("Failed to create directory {}", dir.display()))?;
    }

    let contents = toml::to_string_pretty(creds).context("Failed to serialize credentials")?;

    fs::write(path, contents).with_context(|| format!("Failed to write {}", path.display()))?;

    Ok(())
}
