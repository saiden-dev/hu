use super::*;

#[test]
fn ensure_configured_fails_when_not_configured() {
    let config = SlackConfig {
        oauth: config::OAuthConfig::default(),
        default_channel: String::new(),
        is_configured: false,
    };
    let result = ensure_configured(&config);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not configured"));
}

#[test]
fn ensure_configured_succeeds_when_configured() {
    let config = SlackConfig {
        oauth: config::OAuthConfig {
            client_id: None,
            client_secret: None,
            bot_token: Some("xoxb-test".to_string()),
            user_token: None,
            team_id: Some("T123".to_string()),
            team_name: Some("Test".to_string()),
        },
        default_channel: String::new(),
        is_configured: true,
    };
    let result = ensure_configured(&config);
    assert!(result.is_ok());
}

#[test]
fn ensure_user_token_fails_when_missing() {
    let config = SlackConfig {
        oauth: config::OAuthConfig {
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
    let result = ensure_user_token(&config);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("User token required"));
}

#[test]
fn ensure_user_token_succeeds_when_present() {
    let config = SlackConfig {
        oauth: config::OAuthConfig {
            client_id: None,
            client_secret: None,
            bot_token: Some("xoxb-test".to_string()),
            user_token: Some("xoxp-test".to_string()),
            team_id: None,
            team_name: None,
        },
        default_channel: String::new(),
        is_configured: true,
    };
    let result = ensure_user_token(&config);
    assert!(result.is_ok());
}

#[test]
fn parse_auth_response_with_all_fields() {
    let json = serde_json::json!({
        "ok": true,
        "user_id": "U12345",
        "user": "alice",
        "team_id": "T12345",
        "team": "Acme Corp"
    });
    let info = parse_auth_response(&json);
    assert_eq!(info.user_id, "U12345");
    assert_eq!(info.user, "alice");
    assert_eq!(info.team_id, "T12345");
    assert_eq!(info.team, "Acme Corp");
}

#[test]
fn parse_auth_response_with_missing_fields() {
    let json = serde_json::json!({"ok": true});
    let info = parse_auth_response(&json);
    assert_eq!(info.user_id, "unknown");
    assert_eq!(info.user, "unknown");
    assert_eq!(info.team_id, "");
    assert_eq!(info.team, "Unknown");
}

#[test]
fn validate_bot_token_valid() {
    assert!(validate_bot_token("xoxb-1234-5678").is_ok());
}

#[test]
fn validate_bot_token_invalid() {
    let result = validate_bot_token("xoxp-wrong");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("xoxb-"));
}

#[test]
fn validate_user_token_valid() {
    assert!(validate_user_token("xoxp-1234-5678").is_ok());
}

#[test]
fn validate_user_token_invalid() {
    let result = validate_user_token("xoxb-wrong");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("xoxp-"));
}

#[test]
fn compute_tidy_summary_empty() {
    let results = vec![];
    let summary = compute_tidy_summary(&results);
    assert_eq!(summary.marked_read, 0);
    assert_eq!(summary.has_mentions, 0);
    assert_eq!(summary.already_read, 0);
}

#[test]
fn compute_tidy_summary_mixed_results() {
    let results = vec![
        tidy::TidyResult {
            channel_name: "general".to_string(),
            action: tidy::TidyAction::MarkedRead,
        },
        tidy::TidyResult {
            channel_name: "random".to_string(),
            action: tidy::TidyAction::MarkedRead,
        },
        tidy::TidyResult {
            channel_name: "dev".to_string(),
            action: tidy::TidyAction::HasMention("@you".to_string()),
        },
        tidy::TidyResult {
            channel_name: "announcements".to_string(),
            action: tidy::TidyAction::Skipped,
        },
        tidy::TidyResult {
            channel_name: "ops".to_string(),
            action: tidy::TidyAction::Skipped,
        },
        tidy::TidyResult {
            channel_name: "team".to_string(),
            action: tidy::TidyAction::Skipped,
        },
    ];
    let summary = compute_tidy_summary(&results);
    assert_eq!(summary.marked_read, 2);
    assert_eq!(summary.has_mentions, 1);
    assert_eq!(summary.already_read, 3);
}

#[test]
fn compute_tidy_summary_all_marked() {
    let results = vec![
        tidy::TidyResult {
            channel_name: "a".to_string(),
            action: tidy::TidyAction::MarkedRead,
        },
        tidy::TidyResult {
            channel_name: "b".to_string(),
            action: tidy::TidyAction::MarkedRead,
        },
    ];
    let summary = compute_tidy_summary(&results);
    assert_eq!(summary.marked_read, 2);
    assert_eq!(summary.has_mentions, 0);
    assert_eq!(summary.already_read, 0);
}

#[test]
fn config_path_returns_some() {
    let path = config_path();
    assert!(path.is_some());
    let p = path.expect("should have a path");
    assert!(p.to_string_lossy().contains("settings.toml"));
}
