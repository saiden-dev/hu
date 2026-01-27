use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Credentials {
    #[serde(default)]
    pub github: Option<GithubCredentials>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GithubCredentials {
    pub token: String,
    pub username: String,
}

/// Returns the config directory path (~/.config/hu/)
pub fn config_dir() -> Result<PathBuf> {
    let proj_dirs = directories::ProjectDirs::from("", "", "hu")
        .context("Could not determine config directory")?;
    Ok(proj_dirs.config_dir().to_path_buf())
}

/// Returns the path to credentials.toml
fn credentials_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("credentials.toml"))
}

/// Load credentials from ~/.config/hu/credentials.toml
pub fn load_credentials() -> Result<Credentials> {
    let path = credentials_path()?;
    if !path.exists() {
        return Ok(Credentials::default());
    }

    let contents =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;

    toml::from_str(&contents).with_context(|| format!("Failed to parse {}", path.display()))
}

/// Save credentials to ~/.config/hu/credentials.toml
pub fn save_credentials(creds: &Credentials) -> Result<()> {
    let path = credentials_path()?;
    let dir = path.parent().unwrap();

    fs::create_dir_all(dir)
        .with_context(|| format!("Failed to create directory {}", dir.display()))?;

    let contents = toml::to_string_pretty(creds).context("Failed to serialize credentials")?;

    fs::write(&path, contents).with_context(|| format!("Failed to write {}", path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_dir_returns_path() {
        let dir = config_dir().unwrap();
        assert!(dir.to_string_lossy().contains("hu"));
    }

    #[test]
    fn credentials_serialize_deserialize() {
        let creds = Credentials {
            github: Some(GithubCredentials {
                token: "test_token".to_string(),
                username: "testuser".to_string(),
            }),
        };

        let toml_str = toml::to_string(&creds).unwrap();
        let parsed: Credentials = toml::from_str(&toml_str).unwrap();

        assert!(parsed.github.is_some());
        let gh = parsed.github.unwrap();
        assert_eq!(gh.token, "test_token");
        assert_eq!(gh.username, "testuser");
    }

    #[test]
    fn empty_credentials_default() {
        let creds = Credentials::default();
        assert!(creds.github.is_none());
    }

    #[test]
    fn credentials_without_github_parses() {
        let toml_str = "";
        let creds: Credentials = toml::from_str(toml_str).unwrap();
        assert!(creds.github.is_none());
    }
}
