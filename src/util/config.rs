use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Credentials {
    #[serde(default)]
    pub github: Option<GithubCredentials>,
    #[serde(default)]
    pub jira: Option<JiraCredentials>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_dir_returns_path() {
        let dir = config_dir().unwrap();
        assert!(dir.to_string_lossy().contains("hu"));
    }

    #[test]
    fn credentials_path_is_in_config_dir() {
        let path = credentials_path().unwrap();
        assert!(path.to_string_lossy().contains("hu"));
        assert!(path.to_string_lossy().ends_with("credentials.toml"));
    }

    #[test]
    fn credentials_serialize_deserialize() {
        let creds = Credentials {
            github: Some(GithubCredentials {
                token: "test_token".to_string(),
                username: "testuser".to_string(),
            }),
            jira: None,
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
        assert!(creds.jira.is_none());
    }

    #[test]
    fn credentials_without_github_parses() {
        let toml_str = "";
        let creds: Credentials = toml::from_str(toml_str).unwrap();
        assert!(creds.github.is_none());
        assert!(creds.jira.is_none());
    }

    #[test]
    fn credentials_toml_format() {
        let creds = Credentials {
            github: Some(GithubCredentials {
                token: "ghp_abc123".to_string(),
                username: "octocat".to_string(),
            }),
            jira: None,
        };

        let toml_str = toml::to_string_pretty(&creds).unwrap();
        assert!(toml_str.contains("[github]"));
        assert!(toml_str.contains("token = \"ghp_abc123\""));
        assert!(toml_str.contains("username = \"octocat\""));
    }

    #[test]
    fn github_credentials_clone() {
        let creds = GithubCredentials {
            token: "token".to_string(),
            username: "user".to_string(),
        };
        let cloned = creds.clone();
        assert_eq!(cloned.token, creds.token);
        assert_eq!(cloned.username, creds.username);
    }

    #[test]
    fn credentials_debug_format() {
        let creds = Credentials::default();
        let debug_str = format!("{:?}", creds);
        assert!(debug_str.contains("Credentials"));
    }

    #[test]
    fn github_credentials_debug_format() {
        let creds = GithubCredentials {
            token: "token".to_string(),
            username: "user".to_string(),
        };
        let debug_str = format!("{:?}", creds);
        assert!(debug_str.contains("GithubCredentials"));
    }

    #[test]
    fn load_credentials_handles_missing_file() {
        // load_credentials returns Ok with default if file doesn't exist
        // This tests the path exists check
        let result = load_credentials();
        // Either returns existing creds or default
        assert!(result.is_ok());
    }

    #[test]
    fn credentials_path_parent_exists() {
        let path = credentials_path().unwrap();
        let parent = path.parent();
        assert!(parent.is_some());
    }

    #[test]
    fn config_dir_is_absolute() {
        let dir = config_dir().unwrap();
        assert!(dir.is_absolute());
    }

    // File I/O tests with temp files
    #[test]
    fn save_and_load_credentials_roundtrip() {
        let temp_dir = std::env::temp_dir().join("hu_test_config");
        let _ = std::fs::remove_dir_all(&temp_dir); // Clean up from previous runs
        let path = temp_dir.join("credentials.toml");

        let creds = Credentials {
            github: Some(GithubCredentials {
                token: "test_token_123".to_string(),
                username: "testuser".to_string(),
            }),
            jira: None,
        };

        // Save
        save_credentials_to(&creds, &path).unwrap();
        assert!(path.exists());

        // Load
        let loaded = load_credentials_from(&path).unwrap();
        assert!(loaded.github.is_some());
        let gh = loaded.github.unwrap();
        assert_eq!(gh.token, "test_token_123");
        assert_eq!(gh.username, "testuser");

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn load_credentials_from_missing_file() {
        let path = PathBuf::from("/nonexistent/path/credentials.toml");
        let creds = load_credentials_from(&path).unwrap();
        assert!(creds.github.is_none());
    }

    #[test]
    fn load_credentials_from_empty_file() {
        let temp_dir = std::env::temp_dir().join("hu_test_empty");
        let _ = std::fs::create_dir_all(&temp_dir);
        let path = temp_dir.join("credentials.toml");

        std::fs::write(&path, "").unwrap();
        let creds = load_credentials_from(&path).unwrap();
        assert!(creds.github.is_none());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn load_credentials_from_partial_file() {
        let temp_dir = std::env::temp_dir().join("hu_test_partial");
        let _ = std::fs::create_dir_all(&temp_dir);
        let path = temp_dir.join("credentials.toml");

        std::fs::write(&path, "[github]\ntoken = \"abc\"\nusername = \"user\"").unwrap();
        let creds = load_credentials_from(&path).unwrap();
        assert!(creds.github.is_some());
        assert_eq!(creds.github.unwrap().token, "abc");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn save_credentials_creates_parent_dirs() {
        let temp_dir = std::env::temp_dir().join("hu_test_nested/a/b/c");
        let _ = std::fs::remove_dir_all(std::env::temp_dir().join("hu_test_nested"));
        let path = temp_dir.join("credentials.toml");

        let creds = Credentials::default();
        save_credentials_to(&creds, &path).unwrap();
        assert!(path.exists());

        let _ = std::fs::remove_dir_all(std::env::temp_dir().join("hu_test_nested"));
    }

    #[test]
    fn save_credentials_overwrites_existing() {
        let temp_dir = std::env::temp_dir().join("hu_test_overwrite");
        let _ = std::fs::create_dir_all(&temp_dir);
        let path = temp_dir.join("credentials.toml");

        // Save first version
        let creds1 = Credentials {
            github: Some(GithubCredentials {
                token: "old".to_string(),
                username: "old".to_string(),
            }),
            jira: None,
        };
        save_credentials_to(&creds1, &path).unwrap();

        // Save second version
        let creds2 = Credentials {
            github: Some(GithubCredentials {
                token: "new".to_string(),
                username: "new".to_string(),
            }),
            jira: None,
        };
        save_credentials_to(&creds2, &path).unwrap();

        // Load and verify
        let loaded = load_credentials_from(&path).unwrap();
        assert_eq!(loaded.github.unwrap().token, "new");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    // JiraCredentials tests
    #[test]
    fn jira_credentials_default() {
        let creds = JiraCredentials::default();
        assert_eq!(creds.access_token, "");
        assert_eq!(creds.refresh_token, "");
        assert_eq!(creds.expires_at, 0);
        assert_eq!(creds.cloud_id, "");
        assert_eq!(creds.site_url, "");
    }

    #[test]
    fn jira_credentials_clone() {
        let creds = JiraCredentials {
            access_token: "access".to_string(),
            refresh_token: "refresh".to_string(),
            expires_at: 1234567890,
            cloud_id: "cloud123".to_string(),
            site_url: "https://example.atlassian.net".to_string(),
        };
        let cloned = creds.clone();
        assert_eq!(cloned.access_token, creds.access_token);
        assert_eq!(cloned.refresh_token, creds.refresh_token);
        assert_eq!(cloned.expires_at, creds.expires_at);
        assert_eq!(cloned.cloud_id, creds.cloud_id);
        assert_eq!(cloned.site_url, creds.site_url);
    }

    #[test]
    fn jira_credentials_debug_format() {
        let creds = JiraCredentials::default();
        let debug_str = format!("{:?}", creds);
        assert!(debug_str.contains("JiraCredentials"));
    }

    #[test]
    fn jira_credentials_serialize_deserialize() {
        let creds = Credentials {
            github: None,
            jira: Some(JiraCredentials {
                access_token: "access_token".to_string(),
                refresh_token: "refresh_token".to_string(),
                expires_at: 1234567890,
                cloud_id: "cloud123".to_string(),
                site_url: "https://example.atlassian.net".to_string(),
            }),
        };

        let toml_str = toml::to_string(&creds).unwrap();
        let parsed: Credentials = toml::from_str(&toml_str).unwrap();

        assert!(parsed.jira.is_some());
        let jira = parsed.jira.unwrap();
        assert_eq!(jira.access_token, "access_token");
        assert_eq!(jira.refresh_token, "refresh_token");
        assert_eq!(jira.expires_at, 1234567890);
        assert_eq!(jira.cloud_id, "cloud123");
        assert_eq!(jira.site_url, "https://example.atlassian.net");
    }

    #[test]
    fn jira_credentials_toml_format() {
        let creds = Credentials {
            github: None,
            jira: Some(JiraCredentials {
                access_token: "test_access".to_string(),
                refresh_token: "test_refresh".to_string(),
                expires_at: 9876543210,
                cloud_id: "test_cloud".to_string(),
                site_url: "https://test.atlassian.net".to_string(),
            }),
        };

        let toml_str = toml::to_string_pretty(&creds).unwrap();
        assert!(toml_str.contains("[jira]"));
        assert!(toml_str.contains("access_token = \"test_access\""));
        assert!(toml_str.contains("refresh_token = \"test_refresh\""));
        assert!(toml_str.contains("expires_at = 9876543210"));
        assert!(toml_str.contains("cloud_id = \"test_cloud\""));
        assert!(toml_str.contains("site_url = \"https://test.atlassian.net\""));
    }

    #[test]
    fn save_and_load_jira_credentials_roundtrip() {
        let temp_dir = std::env::temp_dir().join("hu_test_jira_config");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let path = temp_dir.join("credentials.toml");

        let creds = Credentials {
            github: None,
            jira: Some(JiraCredentials {
                access_token: "jira_access".to_string(),
                refresh_token: "jira_refresh".to_string(),
                expires_at: 1111111111,
                cloud_id: "jira_cloud".to_string(),
                site_url: "https://jira.atlassian.net".to_string(),
            }),
        };

        save_credentials_to(&creds, &path).unwrap();
        assert!(path.exists());

        let loaded = load_credentials_from(&path).unwrap();
        assert!(loaded.jira.is_some());
        let jira = loaded.jira.unwrap();
        assert_eq!(jira.access_token, "jira_access");
        assert_eq!(jira.refresh_token, "jira_refresh");
        assert_eq!(jira.expires_at, 1111111111);
        assert_eq!(jira.cloud_id, "jira_cloud");
        assert_eq!(jira.site_url, "https://jira.atlassian.net");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn credentials_with_both_github_and_jira() {
        let creds = Credentials {
            github: Some(GithubCredentials {
                token: "gh_token".to_string(),
                username: "ghuser".to_string(),
            }),
            jira: Some(JiraCredentials {
                access_token: "jira_access".to_string(),
                refresh_token: "jira_refresh".to_string(),
                expires_at: 2222222222,
                cloud_id: "both_cloud".to_string(),
                site_url: "https://both.atlassian.net".to_string(),
            }),
        };

        let toml_str = toml::to_string(&creds).unwrap();
        let parsed: Credentials = toml::from_str(&toml_str).unwrap();

        assert!(parsed.github.is_some());
        assert!(parsed.jira.is_some());
        assert_eq!(parsed.github.unwrap().token, "gh_token");
        assert_eq!(parsed.jira.unwrap().access_token, "jira_access");
    }
}
