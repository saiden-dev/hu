//! Slack HTTP client
//!
//! Handles API requests with Bot token authentication.

use anyhow::Result;
use reqwest::Client;
use serde::de::DeserializeOwned;
use std::time::Duration;
use tokio::time::sleep;

use super::config::{load_config, SlackConfig};

const SLACK_API_URL: &str = "https://slack.com/api";
const MAX_RETRIES: u32 = 3;
const DEFAULT_RETRY_SECS: u64 = 5;

/// Slack API client
pub struct SlackClient {
    config: SlackConfig,
    http: Client,
}

impl SlackClient {
    /// Create a new Slack client
    pub fn new() -> Result<Self> {
        let config = load_config()?;
        let http = Client::builder()
            .user_agent("hu-cli/0.1.0")
            .no_proxy()
            .build()
            .map_err(|e| anyhow::anyhow!(format!("Failed to create HTTP client: {}", e)))?;
        Ok(Self { config, http })
    }

    /// Create a client for testing with explicit config and http client
    #[cfg(test)]
    pub fn with_config(config: SlackConfig, http: Client) -> Self {
        Self { config, http }
    }

    /// Get a reference to the current config (for testing)
    #[cfg(test)]
    #[must_use]
    pub const fn config(&self) -> &SlackConfig {
        &self.config
    }

    /// Get the bot token
    fn bot_token(&self) -> Result<&str> {
        self.config
            .oauth
            .bot_token
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("bot_token not configured".to_string()))
    }

    /// Get the user token (required for search API)
    fn user_token(&self) -> Result<&str> {
        self.config.oauth.user_token.as_deref().ok_or_else(|| {
            anyhow::anyhow!("user_token not configured (required for search)".to_string())
        })
    }

    /// Make a GET request to the Slack API
    pub async fn get<T: DeserializeOwned>(&self, method: &str) -> Result<T> {
        let url = format!("{}/{}", SLACK_API_URL, method);
        let token = self.bot_token()?.to_string();

        self.execute_with_retry(|| {
            self.http
                .get(&url)
                .header("Authorization", format!("Bearer {}", token))
                .header("Accept", "application/json")
                .send()
        })
        .await
    }

    /// Make a GET request with query parameters
    pub async fn get_with_params<T: DeserializeOwned>(
        &self,
        method: &str,
        params: &[(&str, &str)],
    ) -> Result<T> {
        let url = format!("{}/{}", SLACK_API_URL, method);
        let token = self.bot_token()?.to_string();
        let params: Vec<(String, String)> = params
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        self.execute_with_retry(|| {
            self.http
                .get(&url)
                .header("Authorization", format!("Bearer {}", token))
                .header("Accept", "application/json")
                .query(&params)
                .send()
        })
        .await
    }

    /// Make a GET request using user token (required for search API)
    pub async fn get_with_user_token<T: DeserializeOwned>(
        &self,
        method: &str,
        params: &[(&str, &str)],
    ) -> Result<T> {
        let url = format!("{}/{}", SLACK_API_URL, method);
        let token = self.user_token()?.to_string();
        let params: Vec<(String, String)> = params
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        self.execute_with_retry(|| {
            self.http
                .get(&url)
                .header("Authorization", format!("Bearer {}", token))
                .header("Accept", "application/json")
                .query(&params)
                .send()
        })
        .await
    }

    /// Make a POST request to the Slack API
    pub async fn post<T, B>(&self, method: &str, body: &B) -> Result<T>
    where
        T: DeserializeOwned,
        B: serde::Serialize + Sync,
    {
        let url = format!("{}/{}", SLACK_API_URL, method);
        let token = self.bot_token()?.to_string();
        let body_json = serde_json::to_string(body)?;

        self.execute_with_retry(|| {
            self.http
                .post(&url)
                .header("Authorization", format!("Bearer {}", token))
                .header("Accept", "application/json")
                .header("Content-Type", "application/json; charset=utf-8")
                .body(body_json.clone())
                .send()
        })
        .await
    }

    /// Make a POST request using user token (required for conversations.mark)
    pub async fn post_with_user_token<T, B>(&self, method: &str, body: &B) -> Result<T>
    where
        T: DeserializeOwned,
        B: serde::Serialize + Sync,
    {
        let url = format!("{}/{}", SLACK_API_URL, method);
        let token = self.user_token()?.to_string();
        let body_json = serde_json::to_string(body)?;

        self.execute_with_retry(|| {
            self.http
                .post(&url)
                .header("Authorization", format!("Bearer {}", token))
                .header("Accept", "application/json")
                .header("Content-Type", "application/json; charset=utf-8")
                .body(body_json.clone())
                .send()
        })
        .await
    }

    /// Handle API response and check for Slack-specific errors
    fn parse_response<T: DeserializeOwned>(&self, text: &str) -> Result<T> {
        // Slack returns { "ok": false, "error": "..." } for API errors
        let value: serde_json::Value = serde_json::from_str(text)
            .map_err(|e| anyhow::anyhow!("Parse error: {}: {}", e, &text[..text.len().min(200)]))?;

        if let Some(ok) = value.get("ok").and_then(serde_json::Value::as_bool) {
            if !ok {
                let error = value
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown error");
                return Err(anyhow::anyhow!(error.to_string()));
            }
        }

        serde_json::from_str(text)
            .map_err(|e| anyhow::anyhow!("Parse error: {}: {}", e, &text[..text.len().min(200)]))
    }

    /// Execute request with retry on rate limit
    async fn execute_with_retry<F, Fut, T>(&self, request_fn: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<reqwest::Response, reqwest::Error>>,
        T: DeserializeOwned,
    {
        let mut retries = 0;

        loop {
            let response = request_fn().await?;
            let status = response.status();

            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                if retries >= MAX_RETRIES {
                    return Err(anyhow::anyhow!(
                        "Rate limited after {} retries",
                        MAX_RETRIES
                    ));
                }

                // Get retry delay from header or use default
                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(DEFAULT_RETRY_SECS);

                eprintln!(
                    "Rate limited, waiting {} seconds... (retry {}/{})",
                    retry_after,
                    retries + 1,
                    MAX_RETRIES
                );
                sleep(Duration::from_secs(retry_after)).await;
                retries += 1;
                continue;
            }

            if !status.is_success() {
                let body = response.text().await.unwrap_or_default();
                return Err(anyhow::anyhow!("HTTP {}: {}", status.as_u16(), body));
            }

            let text = response.text().await?;
            return self.parse_response(&text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slack::config::{OAuthConfig, SlackConfig};

    fn make_test_client() -> SlackClient {
        let config = SlackConfig {
            oauth: OAuthConfig {
                client_id: None,
                client_secret: None,
                bot_token: Some("xoxb-test".to_string()),
                user_token: Some("xoxp-test".to_string()),
                team_id: Some("T12345".to_string()),
                team_name: Some("Test Team".to_string()),
            },
            default_channel: String::new(),
            is_configured: true,
        };
        let http = Client::builder().build().unwrap();
        SlackClient::with_config(config, http)
    }

    #[test]
    fn test_parse_response_success() {
        let client = make_test_client();
        let json = r#"{"ok": true, "name": "test"}"#;

        #[derive(Debug, serde::Deserialize, PartialEq)]
        struct TestResponse {
            ok: bool,
            name: String,
        }

        let result: Result<TestResponse> = client.parse_response(json);
        assert!(result.is_ok());
        let resp = result.unwrap();
        assert!(resp.ok);
        assert_eq!(resp.name, "test");
    }

    #[test]
    fn test_parse_response_slack_error() {
        let client = make_test_client();
        let json = r#"{"ok": false, "error": "channel_not_found"}"#;

        #[derive(Debug, serde::Deserialize)]
        struct TestResponse {
            ok: bool,
        }

        let result: Result<TestResponse> = client.parse_response(json);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("channel_not_found"));
    }

    #[test]
    fn test_parse_response_slack_error_unknown() {
        let client = make_test_client();
        let json = r#"{"ok": false}"#;

        #[derive(Debug, serde::Deserialize)]
        struct TestResponse {
            ok: bool,
        }

        let result: Result<TestResponse> = client.parse_response(json);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("unknown error"));
    }

    #[test]
    fn test_parse_response_invalid_json() {
        let client = make_test_client();
        let json = "not json at all";

        #[derive(Debug, serde::Deserialize)]
        struct TestResponse {
            ok: bool,
        }

        let result: Result<TestResponse> = client.parse_response(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Parse error"));
    }

    #[test]
    fn test_bot_token() {
        let client = make_test_client();
        assert_eq!(client.bot_token().unwrap(), "xoxb-test");
    }

    #[test]
    fn test_user_token() {
        let client = make_test_client();
        assert_eq!(client.user_token().unwrap(), "xoxp-test");
    }

    #[test]
    fn test_bot_token_missing() {
        let config = SlackConfig {
            oauth: OAuthConfig {
                client_id: None,
                client_secret: None,
                bot_token: None,
                user_token: None,
                team_id: None,
                team_name: None,
            },
            default_channel: String::new(),
            is_configured: false,
        };
        let http = Client::builder().build().unwrap();
        let client = SlackClient::with_config(config, http);

        assert!(client.bot_token().is_err());
    }

    #[test]
    fn test_user_token_missing() {
        let config = SlackConfig {
            oauth: OAuthConfig {
                client_id: None,
                client_secret: None,
                bot_token: Some("xoxb-test".to_string()),
                user_token: None,
                team_id: None,
                team_name: None,
            },
            default_channel: String::new(),
            is_configured: true,
        };
        let http = Client::builder().build().unwrap();
        let client = SlackClient::with_config(config, http);

        assert!(client.user_token().is_err());
    }

    #[test]
    fn test_config_accessor() {
        let client = make_test_client();
        assert!(client.config().is_configured);
        assert_eq!(
            client.config().oauth.team_name,
            Some("Test Team".to_string())
        );
    }
}
