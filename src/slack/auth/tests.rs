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
    let url = build_authorization_url("test-client-id", "http://localhost:9877/callback", "abc123");
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
