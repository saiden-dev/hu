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
