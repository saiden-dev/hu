//! OAuth 2.0 authentication flow for Slack
//!
//! Implements the browser-based OAuth flow to obtain bot tokens.

use anyhow::Result;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

use super::config::{load_config, update_oauth_tokens};

const SLACK_AUTH_URL: &str = "https://slack.com/oauth/v2/authorize";
const SLACK_TOKEN_URL: &str = "https://slack.com/api/oauth.v2.access";

/// OAuth scopes needed for Slack bot access
const OAUTH_SCOPES: &str =
    "channels:read,channels:history,chat:write,search:read,users:read,groups:read";

/// Result of the OAuth flow
pub struct OAuthResult {
    /// Whether authentication succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Slack workspace name if successful
    pub team_name: Option<String>,
}

impl OAuthResult {
    const fn success(team_name: String) -> Self {
        Self {
            success: true,
            error: None,
            team_name: Some(team_name),
        }
    }

    const fn failure(error: String) -> Self {
        Self {
            success: false,
            error: Some(error),
            team_name: None,
        }
    }
}

/// Token response from Slack OAuth
#[derive(serde::Deserialize)]
struct TokenResponse {
    ok: bool,
    access_token: Option<String>,
    team: Option<TeamInfo>,
    error: Option<String>,
}

/// Team info from OAuth response
#[derive(serde::Deserialize)]
struct TeamInfo {
    id: String,
    name: String,
}

/// Generate a random state parameter for OAuth
fn generate_state() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 16] = rng.gen();
    hex::encode(bytes)
}

/// Build the OAuth authorization URL
fn build_authorization_url(client_id: &str, redirect_uri: &str, state: &str) -> String {
    let params = [
        ("client_id", client_id),
        ("scope", OAUTH_SCOPES),
        ("redirect_uri", redirect_uri),
        ("state", state),
    ];

    let query = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    format!("{}?{}", SLACK_AUTH_URL, query)
}

/// Exchange authorization code for tokens
async fn exchange_code_for_tokens(
    client: &reqwest::Client,
    code: &str,
    redirect_uri: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<TokenResponse> {
    let response = client
        .post(SLACK_TOKEN_URL)
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code", code),
            ("redirect_uri", redirect_uri),
        ])
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("Token exchange failed ({status}): {body}"));
    }

    let token_resp: TokenResponse = response.json().await?;

    if !token_resp.ok {
        let error = token_resp
            .error
            .unwrap_or_else(|| "Unknown error".to_string());
        return Err(anyhow::anyhow!(format!("Token exchange failed: {}", error)));
    }

    Ok(token_resp)
}

/// Parse the OAuth callback request to extract code and state
fn parse_callback_request(request: &str) -> Option<(String, String)> {
    let path = request.split_whitespace().nth(1)?;
    let query = path.split('?').nth(1)?;

    let mut code = None;
    let mut state = None;

    for param in query.split('&') {
        let mut parts = param.splitn(2, '=');
        let key = parts.next()?;
        let value = parts.next().unwrap_or("");

        match key {
            "code" => code = Some(urlencoding::decode(value).ok()?.into_owned()),
            "state" => state = Some(urlencoding::decode(value).ok()?.into_owned()),
            _ => {}
        }
    }

    Some((code?, state?))
}

/// Send HTTP response to browser
async fn send_response(
    stream: &mut tokio::net::TcpStream,
    status: &str,
    title: &str,
    message: &str,
) -> std::io::Result<()> {
    let body = format!(
        r#"<!DOCTYPE html>
<html>
<head><title>{}</title></head>
<body style="font-family: sans-serif; padding: 40px; text-align: center;">
<h1>{}</h1>
<p>{}</p>
<p>You can close this window.</p>
</body>
</html>"#,
        title, title, message
    );

    let response = format!(
        "HTTP/1.1 {}\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        body.len(),
        body
    );

    stream.write_all(response.as_bytes()).await
}

/// Run the OAuth authorization flow
///
/// Starts a local server, opens the browser, and waits for the callback.
pub async fn run_oauth_flow(port: u16) -> Result<OAuthResult> {
    let config = load_config()?;

    let client_id = config.oauth.client_id.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "client_id not configured. Set slack.oauth.client_id in ~/.config/hu/settings.toml"
        )
    })?;

    let client_secret = config.oauth.client_secret.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "client_secret not configured. Set slack.oauth.client_secret in ~/.config/hu/settings.toml"
        )
    })?;

    let redirect_uri = format!("http://localhost:{}/callback", port);
    let state = generate_state();
    let auth_url = build_authorization_url(client_id, &redirect_uri, &state);

    // Start local server
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .map_err(|e| {
            anyhow::anyhow!(format!(
                "Failed to start local server on port {}: {}",
                port, e
            ))
        })?;

    println!("\nOpen this URL in your browser to authorize:\n");
    println!("{}\n", auth_url);
    println!("Waiting for authorization...");

    // Try to open browser
    if let Err(_e) = open::that(&auth_url) {
        // debug!("Failed to open browser: {}", _e);
    }

    // Wait for callback with timeout
    let ctx = CallbackContext {
        listener: &listener,
        expected_state: &state,
        redirect_uri: &redirect_uri,
        client_id,
        client_secret,
    };

    tokio::time::timeout(Duration::from_secs(300), handle_callback(ctx))
        .await
        .unwrap_or_else(|_| {
            Ok(OAuthResult::failure(
                "Authorization timed out after 5 minutes".to_string(),
            ))
        })
}

/// Context for handling the OAuth callback
struct CallbackContext<'a> {
    listener: &'a TcpListener,
    expected_state: &'a str,
    redirect_uri: &'a str,
    client_id: &'a str,
    client_secret: &'a str,
}

/// Handle the OAuth callback - accepts connections and processes the callback
async fn handle_callback(ctx: CallbackContext<'_>) -> Result<OAuthResult> {
    loop {
        let (mut stream, _) = ctx
            .listener
            .accept()
            .await
            .map_err(|e| anyhow::anyhow!(format!("Failed to accept connection: {}", e)))?;

        let mut reader = BufReader::new(&mut stream);
        let mut request_line = String::new();
        reader
            .read_line(&mut request_line)
            .await
            .map_err(|e| anyhow::anyhow!(format!("Failed to read request: {}", e)))?;

        // Skip non-callback requests (favicon, etc.)
        if !request_line.contains("/callback") {
            send_response(&mut stream, "404 Not Found", "Not Found", "")
                .await
                .ok();
            continue;
        }

        return process_callback(&mut stream, &request_line, &ctx).await;
    }
}

/// Process the OAuth callback request
async fn process_callback(
    stream: &mut tokio::net::TcpStream,
    request_line: &str,
    ctx: &CallbackContext<'_>,
) -> Result<OAuthResult> {
    // Parse callback parameters
    let Some((code, returned_state)) = parse_callback_request(request_line) else {
        send_response(
            stream,
            "400 Bad Request",
            "Invalid Request",
            "Missing code or state",
        )
        .await
        .ok();
        return Ok(OAuthResult::failure(
            "Missing code or state parameter".to_string(),
        ));
    };

    // Verify state
    if returned_state != ctx.expected_state {
        send_response(stream, "400 Bad Request", "Invalid State", "State mismatch")
            .await
            .ok();
        return Ok(OAuthResult::failure(
            "State mismatch - possible CSRF attack".to_string(),
        ));
    }

    // Exchange code for tokens
    let http = reqwest::Client::new();
    let tokens = match exchange_code_for_tokens(
        &http,
        &code,
        ctx.redirect_uri,
        ctx.client_id,
        ctx.client_secret,
    )
    .await
    {
        Ok(t) => t,
        Err(e) => {
            send_response(
                stream,
                "500 Internal Server Error",
                "Token Exchange Failed",
                &e.to_string(),
            )
            .await
            .ok();
            return Ok(OAuthResult::failure(e.to_string()));
        }
    };

    // Save tokens and complete
    complete_auth(stream, &tokens).await
}

/// Complete authentication by saving tokens
async fn complete_auth(
    stream: &mut tokio::net::TcpStream,
    tokens: &TokenResponse,
) -> Result<OAuthResult> {
    let access_token = tokens
        .access_token
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No access token in response".to_string()))?;

    let team = tokens
        .team
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No team info in response".to_string()))?;

    // Save tokens
    if let Err(e) = update_oauth_tokens(access_token, &team.id, &team.name) {
        send_response(
            stream,
            "500 Internal Server Error",
            "Failed to Save Tokens",
            &e.to_string(),
        )
        .await
        .ok();
        return Ok(OAuthResult::failure(e.to_string()));
    }

    send_response(
        stream,
        "200 OK",
        "Authorization Successful!",
        &format!("Connected to {}.", team.name),
    )
    .await
    .ok();
    Ok(OAuthResult::success(team.name.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth_result_success() {
        let result = OAuthResult::success("Test Team".to_string());
        assert!(result.success);
        assert!(result.error.is_none());
        assert_eq!(result.team_name, Some("Test Team".to_string()));
    }

    #[test]
    fn test_oauth_result_failure() {
        let result = OAuthResult::failure("auth error".to_string());
        assert!(!result.success);
        assert_eq!(result.error, Some("auth error".to_string()));
        assert!(result.team_name.is_none());
    }

    #[test]
    fn test_generate_state_length() {
        let state = generate_state();
        // 16 bytes encoded as hex = 32 characters
        assert_eq!(state.len(), 32);
    }

    #[test]
    fn test_generate_state_unique() {
        let state1 = generate_state();
        let state2 = generate_state();
        assert_ne!(state1, state2);
    }

    #[test]
    fn test_generate_state_hex_chars() {
        let state = generate_state();
        assert!(state.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_build_authorization_url() {
        let url =
            build_authorization_url("test-client-id", "http://localhost:9877/callback", "abc123");
        assert!(url.starts_with("https://slack.com/oauth/v2/authorize?"));
        assert!(url.contains("client_id=test-client-id"));
        assert!(url.contains("redirect_uri=http%3A%2F%2Flocalhost%3A9877%2Fcallback"));
        assert!(url.contains("state=abc123"));
        assert!(url.contains("scope="));
    }

    #[test]
    fn test_build_authorization_url_encodes_special_chars() {
        let url = build_authorization_url("client&id", "http://localhost/test?a=b", "state value");
        assert!(url.contains("client_id=client%26id"));
        assert!(url.contains("state=state%20value"));
    }

    #[test]
    fn test_parse_callback_request_valid() {
        let request = "GET /callback?code=abc123&state=xyz789 HTTP/1.1";
        let result = parse_callback_request(request);
        assert!(result.is_some());
        let (code, state) = result.unwrap();
        assert_eq!(code, "abc123");
        assert_eq!(state, "xyz789");
    }

    #[test]
    fn test_parse_callback_request_url_encoded() {
        let request = "GET /callback?code=abc%20123&state=xyz%26789 HTTP/1.1";
        let result = parse_callback_request(request);
        assert!(result.is_some());
        let (code, state) = result.unwrap();
        assert_eq!(code, "abc 123");
        assert_eq!(state, "xyz&789");
    }

    #[test]
    fn test_parse_callback_request_missing_code() {
        let request = "GET /callback?state=xyz789 HTTP/1.1";
        let result = parse_callback_request(request);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_callback_request_missing_state() {
        let request = "GET /callback?code=abc123 HTTP/1.1";
        let result = parse_callback_request(request);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_callback_request_no_query() {
        let request = "GET /callback HTTP/1.1";
        let result = parse_callback_request(request);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_callback_request_empty() {
        let request = "";
        let result = parse_callback_request(request);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_callback_request_extra_params() {
        let request = "GET /callback?code=abc&state=xyz&extra=foo HTTP/1.1";
        let result = parse_callback_request(request);
        assert!(result.is_some());
        let (code, state) = result.unwrap();
        assert_eq!(code, "abc");
        assert_eq!(state, "xyz");
    }

    #[test]
    fn test_token_response_deserialize_success() {
        let json =
            r#"{"ok": true, "access_token": "xoxb-test", "team": {"id": "T123", "name": "Test"}}"#;
        let resp: TokenResponse = serde_json::from_str(json).unwrap();
        assert!(resp.ok);
        assert_eq!(resp.access_token, Some("xoxb-test".to_string()));
        assert!(resp.team.is_some());
        let team = resp.team.unwrap();
        assert_eq!(team.id, "T123");
        assert_eq!(team.name, "Test");
    }

    #[test]
    fn test_token_response_deserialize_error() {
        let json = r#"{"ok": false, "error": "invalid_code"}"#;
        let resp: TokenResponse = serde_json::from_str(json).unwrap();
        assert!(!resp.ok);
        assert_eq!(resp.error, Some("invalid_code".to_string()));
        assert!(resp.access_token.is_none());
    }

    #[test]
    fn test_team_info_deserialize() {
        let json = r#"{"id": "T12345", "name": "My Team"}"#;
        let team: TeamInfo = serde_json::from_str(json).unwrap();
        assert_eq!(team.id, "T12345");
        assert_eq!(team.name, "My Team");
    }
}
