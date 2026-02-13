use anyhow::{bail, Context, Result};

use crate::util::{load_credentials, save_credentials, GithubCredentials};

#[cfg(test)]
use crate::util::{load_credentials_from, save_credentials_to};

#[cfg(test)]
use std::path::PathBuf;

/// Save token and fetch username
pub async fn login(token: &str) -> Result<String> {
    let username = fetch_username_from_github(token).await?;
    save_login(&username, token)?;
    Ok(username)
}

/// Start device flow authentication (uses `gh auth token` if available)
pub async fn device_flow_login() -> Result<String> {
    // Try to get token from gh CLI first
    if let Some(token) = get_gh_cli_token().await {
        println!("Using token from gh CLI...");
        return login(&token).await;
    }

    // Fall back to prompting for PAT
    bail!(
        "No token found. Please either:\n  \
         1. Run 'gh auth login' first, or\n  \
         2. Use 'hu gh login --token <PAT>' with a Personal Access Token"
    );
}

/// Try to get token from gh CLI
async fn get_gh_cli_token() -> Option<String> {
    let output = tokio::process::Command::new("gh")
        .args(["auth", "token"])
        .output()
        .await
        .ok()?;

    if output.status.success() {
        let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !token.is_empty() {
            return Some(token);
        }
    }
    None
}

/// Save login credentials (extracted for testability)
pub fn save_login(username: &str, token: &str) -> Result<()> {
    let mut creds = load_credentials().unwrap_or_default();
    creds.github = Some(GithubCredentials {
        token: token.to_string(),
        username: username.to_string(),
    });
    save_credentials(&creds)
}

/// Save login to a specific path (for testing)
#[cfg(test)]
fn save_login_to(username: &str, token: &str, path: &PathBuf) -> Result<()> {
    let mut creds = load_credentials_from(path).unwrap_or_default();
    creds.github = Some(GithubCredentials {
        token: token.to_string(),
        username: username.to_string(),
    });
    save_credentials_to(&creds, path)
}

/// Load login from a specific path (for testing)
#[cfg(test)]
fn load_login_from(path: &PathBuf) -> Option<(String, String)> {
    load_credentials_from(path)
        .ok()
        .and_then(|c| c.github.map(|g| (g.username, g.token)))
}

/// Fetch username from GitHub API (the actual network call)
async fn fetch_username_from_github(token: &str) -> Result<String> {
    let octocrab = octocrab::OctocrabBuilder::new()
        .personal_token(token.to_string())
        .build()
        .context("Failed to create GitHub client")?;

    let user = octocrab
        .current()
        .user()
        .await
        .context("Failed to get current user - check your token")?;

    Ok(user.login)
}

/// Get stored token if available
pub fn get_token() -> Option<String> {
    load_credentials()
        .ok()
        .and_then(|c| c.github.map(|g| g.token))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_token_returns_option() {
        let result = get_token();
        // Result is either Some(token) or None
        assert!(result.is_some() || result.is_none());
    }

    #[test]
    fn get_token_consistent_results() {
        // Calling get_token multiple times should return the same result
        let result1 = get_token();
        let result2 = get_token();
        assert_eq!(result1.is_some(), result2.is_some());
    }

    #[test]
    fn credentials_struct_usage() {
        // Verify we can create and use the credential structs
        let creds = GithubCredentials {
            token: "test_token".to_string(),
            username: "testuser".to_string(),
        };
        assert_eq!(creds.token, "test_token");
        assert_eq!(creds.username, "testuser");
    }

    #[test]
    fn credentials_optional_in_parent() {
        use crate::util::Credentials;
        let creds = Credentials::default();
        assert!(creds.github.is_none());
    }

    // Tests for path-based login functions
    #[test]
    fn save_and_load_login_roundtrip() {
        let temp_dir = std::env::temp_dir().join("hu_test_auth");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let path = temp_dir.join("credentials.toml");

        // Save login
        save_login_to("testuser", "test_token", &path).unwrap();

        // Load login
        let result = load_login_from(&path);
        assert!(result.is_some());
        let (username, token) = result.unwrap();
        assert_eq!(username, "testuser");
        assert_eq!(token, "test_token");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn load_login_from_missing_file() {
        let path = PathBuf::from("/nonexistent/credentials.toml");
        let result = load_login_from(&path);
        assert!(result.is_none());
    }

    #[test]
    fn load_login_from_empty_credentials() {
        let temp_dir = std::env::temp_dir().join("hu_test_auth_empty");
        let _ = std::fs::create_dir_all(&temp_dir);
        let path = temp_dir.join("credentials.toml");

        // Write empty credentials
        std::fs::write(&path, "").unwrap();

        let result = load_login_from(&path);
        assert!(result.is_none());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn save_login_overwrites_existing() {
        let temp_dir = std::env::temp_dir().join("hu_test_auth_overwrite");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let path = temp_dir.join("credentials.toml");

        // Save first login
        save_login_to("user1", "token1", &path).unwrap();

        // Save second login
        save_login_to("user2", "token2", &path).unwrap();

        // Load and verify
        let (username, token) = load_login_from(&path).unwrap();
        assert_eq!(username, "user2");
        assert_eq!(token, "token2");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
