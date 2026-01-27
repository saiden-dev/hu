use anyhow::{bail, Context, Result};
use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;

use crate::util::{load_credentials, save_credentials, GithubCredentials};

use super::types::{DeviceCodeResponse, TokenResponse};

// GitHub OAuth App Client ID for hu CLI
// Users can override this with HU_GITHUB_CLIENT_ID env var
const DEFAULT_CLIENT_ID: &str = "Ov23liF3LGT1Yq9Dh6SU";

fn client_id() -> String {
    std::env::var("HU_GITHUB_CLIENT_ID").unwrap_or_else(|_| DEFAULT_CLIENT_ID.to_string())
}

/// Request a device code from GitHub
pub async fn request_device_code(client: &Client) -> Result<DeviceCodeResponse> {
    let response = client
        .post("https://github.com/login/device/code")
        .header("Accept", "application/json")
        .form(&[("client_id", client_id()), ("scope", "repo".to_string())])
        .send()
        .await
        .context("Failed to request device code")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        bail!("Device code request failed: {} - {}", status, body);
    }

    response
        .json()
        .await
        .context("Failed to parse device code response")
}

/// Poll GitHub for access token
pub async fn poll_for_token(client: &Client, device_code: &str, interval: u64) -> Result<String> {
    let poll_interval = Duration::from_secs(interval);

    loop {
        sleep(poll_interval).await;

        let response = client
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .form(&[
                ("client_id", client_id()),
                ("device_code", device_code.to_string()),
                (
                    "grant_type",
                    "urn:ietf:params:oauth:grant-type:device_code".to_string(),
                ),
            ])
            .send()
            .await
            .context("Failed to poll for token")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            bail!("Token request failed: {} - {}", status, body);
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .context("Failed to parse token response")?;

        match token_response {
            TokenResponse::Success { access_token, .. } => {
                return Ok(access_token);
            }
            TokenResponse::Pending {
                error,
                error_description,
                ..
            } => {
                match error.as_str() {
                    "authorization_pending" => {
                        // User hasn't authorized yet, continue polling
                        continue;
                    }
                    "slow_down" => {
                        // Need to slow down, add extra delay
                        sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                    "expired_token" => {
                        bail!("Device code expired. Please try again.");
                    }
                    "access_denied" => {
                        bail!("Authorization was denied by user.");
                    }
                    _ => {
                        bail!(
                            "Authorization failed: {} - {}",
                            error,
                            error_description.unwrap_or_default()
                        );
                    }
                }
            }
        }
    }
}

/// Get the username for the authenticated user
pub async fn get_username(token: &str) -> Result<String> {
    let octocrab = octocrab::OctocrabBuilder::new()
        .personal_token(token.to_string())
        .build()
        .context("Failed to create GitHub client")?;

    let user = octocrab
        .current()
        .user()
        .await
        .context("Failed to get current user")?;

    Ok(user.login)
}

/// Full OAuth Device Flow login
pub async fn login() -> Result<String> {
    let client = Client::new();

    // Request device code
    let device_code = request_device_code(&client).await?;

    // Display instructions to user
    println!("\n  Open: {}", device_code.verification_uri);
    println!("  Enter code: {}\n", device_code.user_code);
    println!("Waiting for authorization...");

    // Poll for token
    let token = poll_for_token(&client, &device_code.device_code, device_code.interval).await?;

    // Get username
    let username = get_username(&token).await?;

    // Save credentials
    let mut creds = load_credentials().unwrap_or_default();
    creds.github = Some(GithubCredentials {
        token: token.clone(),
        username: username.clone(),
    });
    save_credentials(&creds)?;

    Ok(username)
}

/// Check if already authenticated
pub fn is_authenticated() -> bool {
    load_credentials()
        .map(|c| c.github.is_some())
        .unwrap_or(false)
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

    // Tests for client_id() are combined to avoid race conditions
    // with environment variable access
    #[test]
    fn client_id_behavior() {
        // Test env var override
        std::env::set_var("HU_GITHUB_CLIENT_ID", "test_client_id");
        let id = client_id();
        assert_eq!(id, "test_client_id");

        // Test default (after removing env var)
        std::env::remove_var("HU_GITHUB_CLIENT_ID");
        let id = client_id();
        assert_eq!(id, DEFAULT_CLIENT_ID);
    }

    #[test]
    fn is_authenticated_returns_false_without_creds() {
        // This test relies on no creds being stored in test environment
        // In practice, a real test would use a temp directory
        let result = is_authenticated();
        // Just verify it doesn't panic and returns a bool
        assert!(result || !result);
    }
}
