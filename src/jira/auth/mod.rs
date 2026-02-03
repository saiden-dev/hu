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

#[cfg(test)]
mod tests;

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
