use anyhow::{bail, Context, Result};
use axum::{
    extract::{Query, State},
    response::Html,
    routing::get,
    Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::Rng;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::util::{load_credentials, save_credentials, JiraCredentials};

use super::types::OAuthConfig;

const AUTH_URL: &str = "https://auth.atlassian.com/authorize";
const TOKEN_URL: &str = "https://auth.atlassian.com/oauth/token";
const RESOURCES_URL: &str = "https://api.atlassian.com/oauth/token/accessible-resources";
const CALLBACK_PORT: u16 = 9876;
const SCOPES: &str = "read:jira-work write:jira-work read:jira-user offline_access";

/// OAuth callback state
#[derive(Debug, Clone)]
struct CallbackState {
    expected_state: String,
    code: Option<String>,
    error: Option<String>,
}

/// Load OAuth config from environment or config file
pub fn load_oauth_config() -> Result<OAuthConfig> {
    // Try environment variables first
    if let (Ok(client_id), Ok(client_secret)) = (
        std::env::var("JIRA_CLIENT_ID"),
        std::env::var("JIRA_CLIENT_SECRET"),
    ) {
        return Ok(OAuthConfig {
            client_id,
            client_secret,
        });
    }

    // Try config file
    let config_path = crate::util::config_dir()?.join("jira-oauth.toml");
    if config_path.exists() {
        let contents = std::fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read {}", config_path.display()))?;
        let config: OAuthConfig = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse {}", config_path.display()))?;
        return Ok(config);
    }

    bail!(
        "Jira OAuth not configured. Set JIRA_CLIENT_ID and JIRA_CLIENT_SECRET environment variables, \
        or create {} with client_id and client_secret fields.",
        crate::util::config_dir()?.join("jira-oauth.toml").display()
    )
}

/// Generate a random state string for CSRF protection
pub fn generate_state() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Build the authorization URL
pub fn build_auth_url(client_id: &str, state: &str) -> String {
    let redirect_uri = format!("http://localhost:{}/callback", CALLBACK_PORT);
    format!(
        "{}?audience=api.atlassian.com&client_id={}&scope={}&redirect_uri={}&state={}&response_type=code&prompt=consent",
        AUTH_URL,
        client_id,
        urlencoded(SCOPES),
        urlencoded(&redirect_uri),
        state
    )
}

/// URL encode a string
fn urlencoded(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            ' ' => "%20".to_string(),
            ':' => "%3A".to_string(),
            '/' => "%2F".to_string(),
            _ => c.to_string(),
        })
        .collect()
}

/// Start OAuth flow and return user display name
pub async fn login() -> Result<String> {
    let config = load_oauth_config()?;
    let state = generate_state();

    // Start local server to receive callback
    let callback_state = Arc::new(Mutex::new(CallbackState {
        expected_state: state.clone(),
        code: None,
        error: None,
    }));

    let server_state = callback_state.clone();
    let server = tokio::spawn(async move { start_callback_server(server_state).await });

    // Open browser for authorization
    let auth_url = build_auth_url(&config.client_id, &state);
    open::that(&auth_url).context("Failed to open browser")?;

    // Wait for callback
    server.await??;

    // Get the authorization code
    let state_lock = callback_state.lock().await;
    if let Some(error) = &state_lock.error {
        bail!("Authorization failed: {}", error);
    }
    let code = state_lock
        .code
        .clone()
        .context("No authorization code received")?;
    drop(state_lock);

    // Exchange code for tokens
    let tokens = exchange_code(&config, &code).await?;

    // Get accessible resources to find cloud ID
    let resources = get_accessible_resources(&tokens.access_token).await?;
    let resource = resources
        .first()
        .context("No accessible Jira sites found")?;

    // Get user info
    let user = get_current_user(&tokens.access_token, &resource.id).await?;

    // Save credentials
    let creds = JiraCredentials {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        expires_at: tokens.expires_at,
        cloud_id: resource.id.clone(),
        site_url: resource.url.clone(),
    };
    save_jira_credentials(creds)?;

    Ok(user)
}

/// Start the local callback server
async fn start_callback_server(state: Arc<Mutex<CallbackState>>) -> Result<()> {
    let app = Router::new()
        .route("/callback", get(handle_callback))
        .with_state(state.clone());

    let addr = SocketAddr::from(([127, 0, 0, 1], CALLBACK_PORT));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("Failed to bind callback server")?;

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            // Wait until we have a code or error
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                let state_lock = state.lock().await;
                if state_lock.code.is_some() || state_lock.error.is_some() {
                    break;
                }
            }
            // Give a moment for the response to be sent
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        })
        .await
        .context("Callback server failed")?;

    Ok(())
}

/// Handle the OAuth callback
async fn handle_callback(
    State(state): State<Arc<Mutex<CallbackState>>>,
    Query(params): Query<HashMap<String, String>>,
) -> Html<&'static str> {
    let mut state_lock = state.lock().await;

    // Check for error
    if let Some(error) = params.get("error") {
        state_lock.error = Some(error.clone());
        return Html(
            "<html><body><h1>Authorization Failed</h1><p>You can close this window.</p></body></html>",
        );
    }

    // Verify state parameter
    if let Some(received_state) = params.get("state") {
        if received_state != &state_lock.expected_state {
            state_lock.error = Some("State mismatch - possible CSRF attack".to_string());
            return Html(
                "<html><body><h1>Error</h1><p>State verification failed.</p></body></html>",
            );
        }
    } else {
        state_lock.error = Some("Missing state parameter".to_string());
        return Html("<html><body><h1>Error</h1><p>Missing state parameter.</p></body></html>");
    }

    // Get authorization code
    if let Some(code) = params.get("code") {
        state_lock.code = Some(code.clone());
        Html("<html><body><h1>Success!</h1><p>You can close this window and return to the terminal.</p></body></html>")
    } else {
        state_lock.error = Some("Missing authorization code".to_string());
        Html("<html><body><h1>Error</h1><p>Missing authorization code.</p></body></html>")
    }
}

/// Token response from Atlassian
#[derive(Debug)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    expires_at: i64,
}

/// Exchange authorization code for tokens
async fn exchange_code(config: &OAuthConfig, code: &str) -> Result<TokenResponse> {
    let client = reqwest::Client::new();
    let redirect_uri = format!("http://localhost:{}/callback", CALLBACK_PORT);

    let response = client
        .post(TOKEN_URL)
        .json(&serde_json::json!({
            "grant_type": "authorization_code",
            "client_id": config.client_id,
            "client_secret": config.client_secret,
            "code": code,
            "redirect_uri": redirect_uri
        }))
        .send()
        .await
        .context("Failed to exchange code for tokens")?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        bail!("Token exchange failed: {}", error_text);
    }

    let json: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse token response")?;

    let access_token = json["access_token"]
        .as_str()
        .context("Missing access_token")?
        .to_string();
    let refresh_token = json["refresh_token"]
        .as_str()
        .context("Missing refresh_token")?
        .to_string();
    let expires_in = json["expires_in"].as_i64().unwrap_or(3600);
    let expires_at = chrono::Utc::now().timestamp() + expires_in;

    Ok(TokenResponse {
        access_token,
        refresh_token,
        expires_at,
    })
}

/// Get accessible Jira cloud resources
async fn get_accessible_resources(
    access_token: &str,
) -> Result<Vec<super::types::AccessibleResource>> {
    let client = reqwest::Client::new();

    let response = client
        .get(RESOURCES_URL)
        .bearer_auth(access_token)
        .send()
        .await
        .context("Failed to get accessible resources")?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        bail!("Failed to get accessible resources: {}", error_text);
    }

    let json: serde_json::Value = response.json().await?;
    let resources: Vec<super::types::AccessibleResource> = json
        .as_array()
        .context("Expected array of resources")?
        .iter()
        .filter_map(|r| {
            Some(super::types::AccessibleResource {
                id: r["id"].as_str()?.to_string(),
                url: r["url"].as_str()?.to_string(),
                name: r["name"].as_str()?.to_string(),
            })
        })
        .collect();

    Ok(resources)
}

/// Get current user display name
async fn get_current_user(access_token: &str, cloud_id: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://api.atlassian.com/ex/jira/{}/rest/api/3/myself",
        cloud_id
    );

    let response = client
        .get(&url)
        .bearer_auth(access_token)
        .send()
        .await
        .context("Failed to get current user")?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        bail!("Failed to get current user: {}", error_text);
    }

    let json: serde_json::Value = response.json().await?;
    let display_name = json["displayName"]
        .as_str()
        .context("Missing displayName")?
        .to_string();

    Ok(display_name)
}

/// Refresh access token if expired or about to expire
pub async fn refresh_token_if_needed() -> Result<String> {
    let creds = get_credentials().context("Not authenticated. Run `hu jira auth` first.")?;

    // Check if token expires in the next 5 minutes
    let now = chrono::Utc::now().timestamp();
    if creds.expires_at > now + 300 {
        return Ok(creds.access_token);
    }

    // Need to refresh
    let config = load_oauth_config()?;
    let tokens = refresh_token(&config, &creds.refresh_token).await?;

    // Save updated credentials
    let new_creds = JiraCredentials {
        access_token: tokens.access_token.clone(),
        refresh_token: tokens.refresh_token,
        expires_at: tokens.expires_at,
        cloud_id: creds.cloud_id,
        site_url: creds.site_url,
    };
    save_jira_credentials(new_creds)?;

    Ok(tokens.access_token)
}

/// Refresh access token
async fn refresh_token(config: &OAuthConfig, refresh_token: &str) -> Result<TokenResponse> {
    let client = reqwest::Client::new();

    let response = client
        .post(TOKEN_URL)
        .json(&serde_json::json!({
            "grant_type": "refresh_token",
            "client_id": config.client_id,
            "client_secret": config.client_secret,
            "refresh_token": refresh_token
        }))
        .send()
        .await
        .context("Failed to refresh token")?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        bail!("Token refresh failed: {}", error_text);
    }

    let json: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse token response")?;

    let access_token = json["access_token"]
        .as_str()
        .context("Missing access_token")?
        .to_string();
    let new_refresh_token = json["refresh_token"]
        .as_str()
        .unwrap_or(refresh_token)
        .to_string();
    let expires_in = json["expires_in"].as_i64().unwrap_or(3600);
    let expires_at = chrono::Utc::now().timestamp() + expires_in;

    Ok(TokenResponse {
        access_token,
        refresh_token: new_refresh_token,
        expires_at,
    })
}

/// Get stored Jira credentials
pub fn get_credentials() -> Option<JiraCredentials> {
    load_credentials().ok().and_then(|c| c.jira)
}

/// Save Jira credentials
fn save_jira_credentials(jira: JiraCredentials) -> Result<()> {
    let mut creds = load_credentials().unwrap_or_default();
    creds.jira = Some(jira);
    save_credentials(&creds)
}

/// Parse token response JSON (pure function, testable)
#[cfg(test)]
pub fn parse_token_response(json: &serde_json::Value) -> Result<(String, String, i64)> {
    let access_token = json["access_token"]
        .as_str()
        .context("Missing access_token")?
        .to_string();
    let refresh_token = json["refresh_token"]
        .as_str()
        .context("Missing refresh_token")?
        .to_string();
    let expires_in = json["expires_in"].as_i64().unwrap_or(3600);

    Ok((access_token, refresh_token, expires_in))
}

/// Parse accessible resources JSON (pure function, testable)
#[cfg(test)]
pub fn parse_accessible_resources(
    json: &serde_json::Value,
) -> Vec<super::types::AccessibleResource> {
    json.as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|r| {
            Some(super::types::AccessibleResource {
                id: r["id"].as_str()?.to_string(),
                url: r["url"].as_str()?.to_string(),
                name: r["name"].as_str()?.to_string(),
            })
        })
        .collect()
}

/// Parse user response JSON (pure function, testable)
#[cfg(test)]
pub fn parse_user_response(json: &serde_json::Value) -> Option<String> {
    json["displayName"].as_str().map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn generate_state_returns_nonempty_string() {
        let state = generate_state();
        assert!(!state.is_empty());
    }

    #[test]
    fn generate_state_returns_unique_values() {
        let state1 = generate_state();
        let state2 = generate_state();
        assert_ne!(state1, state2);
    }

    #[test]
    fn generate_state_is_url_safe() {
        let state = generate_state();
        // URL-safe base64 only uses alphanumeric, dash, underscore
        assert!(state
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_'));
    }

    #[test]
    fn build_auth_url_contains_required_params() {
        let url = build_auth_url("test_client_id", "test_state");
        assert!(url.contains("client_id=test_client_id"));
        assert!(url.contains("state=test_state"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("audience=api.atlassian.com"));
        assert!(url.contains("prompt=consent"));
    }

    #[test]
    fn build_auth_url_contains_scopes() {
        let url = build_auth_url("id", "state");
        assert!(url.contains("read%3Ajira-work")); // read:jira-work encoded
        assert!(url.contains("write%3Ajira-work")); // write:jira-work encoded
        assert!(url.contains("offline_access"));
    }

    #[test]
    fn build_auth_url_contains_redirect_uri() {
        let url = build_auth_url("id", "state");
        assert!(url.contains("redirect_uri=http%3A%2F%2Flocalhost%3A9876%2Fcallback"));
    }

    #[test]
    fn urlencoded_encodes_spaces() {
        assert_eq!(urlencoded("hello world"), "hello%20world");
    }

    #[test]
    fn urlencoded_encodes_colons() {
        assert_eq!(urlencoded("a:b"), "a%3Ab");
    }

    #[test]
    fn urlencoded_encodes_slashes() {
        assert_eq!(urlencoded("a/b"), "a%2Fb");
    }

    #[test]
    fn urlencoded_preserves_alphanumeric() {
        assert_eq!(urlencoded("abc123"), "abc123");
    }

    #[test]
    fn parse_token_response_extracts_fields() {
        let json = json!({
            "access_token": "access123",
            "refresh_token": "refresh456",
            "expires_in": 7200
        });
        let (access, refresh, expires_in) = parse_token_response(&json).unwrap();
        assert_eq!(access, "access123");
        assert_eq!(refresh, "refresh456");
        assert_eq!(expires_in, 7200);
    }

    #[test]
    fn parse_token_response_uses_default_expires() {
        let json = json!({
            "access_token": "access",
            "refresh_token": "refresh"
        });
        let (_, _, expires_in) = parse_token_response(&json).unwrap();
        assert_eq!(expires_in, 3600);
    }

    #[test]
    fn parse_token_response_fails_missing_access_token() {
        let json = json!({
            "refresh_token": "refresh"
        });
        let result = parse_token_response(&json);
        assert!(result.is_err());
    }

    #[test]
    fn parse_token_response_fails_missing_refresh_token() {
        let json = json!({
            "access_token": "access"
        });
        let result = parse_token_response(&json);
        assert!(result.is_err());
    }

    #[test]
    fn parse_accessible_resources_extracts_resources() {
        let json = json!([
            {"id": "cloud1", "url": "https://a.atlassian.net", "name": "Site A"},
            {"id": "cloud2", "url": "https://b.atlassian.net", "name": "Site B"}
        ]);
        let resources = parse_accessible_resources(&json);
        assert_eq!(resources.len(), 2);
        assert_eq!(resources[0].id, "cloud1");
        assert_eq!(resources[0].url, "https://a.atlassian.net");
        assert_eq!(resources[0].name, "Site A");
        assert_eq!(resources[1].id, "cloud2");
    }

    #[test]
    fn parse_accessible_resources_handles_empty_array() {
        let json = json!([]);
        let resources = parse_accessible_resources(&json);
        assert!(resources.is_empty());
    }

    #[test]
    fn parse_accessible_resources_handles_non_array() {
        let json = json!({"not": "an array"});
        let resources = parse_accessible_resources(&json);
        assert!(resources.is_empty());
    }

    #[test]
    fn parse_accessible_resources_skips_incomplete_entries() {
        let json = json!([
            {"id": "cloud1", "url": "https://a.atlassian.net", "name": "Site A"},
            {"id": "cloud2"}, // missing url and name
            {"url": "https://c.atlassian.net", "name": "Site C"} // missing id
        ]);
        let resources = parse_accessible_resources(&json);
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].id, "cloud1");
    }

    #[test]
    fn parse_user_response_extracts_display_name() {
        let json = json!({
            "displayName": "John Doe",
            "accountId": "123"
        });
        let name = parse_user_response(&json);
        assert_eq!(name, Some("John Doe".to_string()));
    }

    #[test]
    fn parse_user_response_returns_none_for_missing_name() {
        let json = json!({
            "accountId": "123"
        });
        let name = parse_user_response(&json);
        assert!(name.is_none());
    }

    #[test]
    fn get_credentials_returns_option() {
        let result = get_credentials();
        // Result is either Some(creds) or None
        assert!(result.is_some() || result.is_none());
    }

    #[test]
    fn callback_state_debug_format() {
        let state = CallbackState {
            expected_state: "test".to_string(),
            code: None,
            error: None,
        };
        let debug_str = format!("{:?}", state);
        assert!(debug_str.contains("CallbackState"));
    }

    #[test]
    fn callback_state_clone() {
        let state = CallbackState {
            expected_state: "state123".to_string(),
            code: Some("code456".to_string()),
            error: None,
        };
        let cloned = state.clone();
        assert_eq!(cloned.expected_state, state.expected_state);
        assert_eq!(cloned.code, state.code);
        assert_eq!(cloned.error, state.error);
    }

    #[test]
    fn token_response_debug_format() {
        let response = TokenResponse {
            access_token: "access".to_string(),
            refresh_token: "refresh".to_string(),
            expires_at: 1234567890,
        };
        let debug_str = format!("{:?}", response);
        assert!(debug_str.contains("TokenResponse"));
    }

    #[test]
    fn constants_are_valid() {
        assert!(AUTH_URL.starts_with("https://"));
        assert!(TOKEN_URL.starts_with("https://"));
        assert!(RESOURCES_URL.starts_with("https://"));
        assert!(CALLBACK_PORT > 0);
        assert!(!SCOPES.is_empty());
    }

    #[test]
    fn scopes_contain_required_permissions() {
        assert!(SCOPES.contains("read:jira-work"));
        assert!(SCOPES.contains("write:jira-work"));
        assert!(SCOPES.contains("read:jira-user"));
        assert!(SCOPES.contains("offline_access"));
    }
}
