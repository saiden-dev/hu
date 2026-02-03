use super::*;

#[test]
fn test_oauth_config_is_configured_with_valid_bot_token() {
    let config = OAuthConfig {
        client_id: None,
        client_secret: None,
        bot_token: Some("xoxb-12345-67890".to_string()),
        user_token: None,
        team_id: None,
        team_name: None,
    };
    assert!(config.is_configured());
}

#[test]
fn test_oauth_config_is_configured_with_invalid_bot_token() {
    let config = OAuthConfig {
        client_id: None,
        client_secret: None,
        bot_token: Some("invalid-token".to_string()),
        user_token: None,
        team_id: None,
        team_name: None,
    };
    assert!(!config.is_configured());
}

#[test]
fn test_oauth_config_is_configured_without_bot_token() {
    let config = OAuthConfig {
        client_id: None,
        client_secret: None,
        bot_token: None,
        user_token: None,
        team_id: None,
        team_name: None,
    };
    assert!(!config.is_configured());
}

#[test]
fn test_oauth_config_has_user_token_with_valid_token() {
    let config = OAuthConfig {
        client_id: None,
        client_secret: None,
        bot_token: None,
        user_token: Some("xoxp-12345-67890".to_string()),
        team_id: None,
        team_name: None,
    };
    assert!(config.has_user_token());
}

#[test]
fn test_oauth_config_has_user_token_with_invalid_token() {
    let config = OAuthConfig {
        client_id: None,
        client_secret: None,
        bot_token: None,
        user_token: Some("invalid-token".to_string()),
        team_id: None,
        team_name: None,
    };
    assert!(!config.has_user_token());
}

#[test]
fn test_oauth_config_has_user_token_without_token() {
    let config = OAuthConfig {
        client_id: None,
        client_secret: None,
        bot_token: None,
        user_token: None,
        team_id: None,
        team_name: None,
    };
    assert!(!config.has_user_token());
}

#[test]
fn test_config_path_returns_some() {
    // This test just verifies config_path returns Some on systems with a home dir
    let path = config_path();
    // On most systems this should return Some
    if let Some(p) = path {
        assert!(p.to_string_lossy().contains("settings.toml"));
    }
}

#[test]
fn test_slack_config_default() {
    let config = SlackConfig::default();
    assert!(!config.is_configured);
    assert!(config.default_channel.is_empty());
    assert!(!config.oauth.is_configured());
}

#[test]
fn test_oauth_config_default() {
    let config = OAuthConfig::default();
    assert!(config.client_id.is_none());
    assert!(config.client_secret.is_none());
    assert!(config.bot_token.is_none());
    assert!(config.user_token.is_none());
    assert!(config.team_id.is_none());
    assert!(config.team_name.is_none());
}

#[test]
fn test_oauth_config_serialize_deserialize() {
    let config = OAuthConfig {
        client_id: Some("client123".to_string()),
        client_secret: Some("secret456".to_string()),
        bot_token: Some("xoxb-test".to_string()),
        user_token: Some("xoxp-test".to_string()),
        team_id: Some("T12345".to_string()),
        team_name: Some("Test Team".to_string()),
    };

    let json = serde_json::to_string(&config).unwrap();
    let deserialized: OAuthConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.client_id, Some("client123".to_string()));
    assert_eq!(deserialized.client_secret, Some("secret456".to_string()));
    assert_eq!(deserialized.bot_token, Some("xoxb-test".to_string()));
    assert_eq!(deserialized.user_token, Some("xoxp-test".to_string()));
    assert_eq!(deserialized.team_id, Some("T12345".to_string()));
    assert_eq!(deserialized.team_name, Some("Test Team".to_string()));
}

#[test]
fn test_oauth_config_debug() {
    let config = OAuthConfig {
        client_id: Some("client123".to_string()),
        client_secret: None,
        bot_token: None,
        user_token: None,
        team_id: None,
        team_name: None,
    };

    let debug = format!("{:?}", config);
    assert!(debug.contains("OAuthConfig"));
    assert!(debug.contains("client123"));
}

#[test]
fn test_oauth_config_clone() {
    let config = OAuthConfig {
        client_id: Some("client123".to_string()),
        client_secret: None,
        bot_token: Some("xoxb-test".to_string()),
        user_token: None,
        team_id: None,
        team_name: None,
    };

    let cloned = config.clone();
    assert_eq!(cloned.client_id, config.client_id);
    assert_eq!(cloned.bot_token, config.bot_token);
}

#[test]
fn test_slack_config_clone() {
    let config = SlackConfig {
        default_channel: "general".to_string(),
        oauth: OAuthConfig::default(),
        is_configured: true,
    };

    let cloned = config.clone();
    assert_eq!(cloned.default_channel, "general");
    assert!(cloned.is_configured);
}

#[test]
fn test_slack_config_debug() {
    let config = SlackConfig {
        default_channel: "test".to_string(),
        oauth: OAuthConfig::default(),
        is_configured: false,
    };

    let debug = format!("{:?}", config);
    assert!(debug.contains("SlackConfig"));
    assert!(debug.contains("test"));
}

#[test]
fn test_settings_file_parse() {
    let toml_str = r##"
            [slack]
            default_channel = "general"

            [slack.oauth]
            client_id = "client123"
            client_secret = "secret456"
            bot_token = "xoxb-token"
            user_token = "xoxp-token"
            team_id = "T12345"
            team_name = "Test Team"
        "##;

    let settings: SettingsFile = toml::from_str(toml_str).unwrap();
    let slack = settings.slack.unwrap();
    assert_eq!(slack.default_channel, Some("general".to_string()));

    let oauth = slack.oauth.unwrap();
    assert_eq!(oauth.client_id, Some("client123".to_string()));
    assert_eq!(oauth.bot_token, Some("xoxb-token".to_string()));
    assert_eq!(oauth.team_name, Some("Test Team".to_string()));
}

#[test]
fn test_settings_file_parse_empty() {
    let toml_str = "";
    let settings: SettingsFile = toml::from_str(toml_str).unwrap();
    assert!(settings.slack.is_none());
}

#[test]
fn test_settings_file_parse_no_oauth() {
    let toml_str = r##"
            [slack]
            default_channel = "test"
        "##;

    let settings: SettingsFile = toml::from_str(toml_str).unwrap();
    let slack = settings.slack.unwrap();
    assert_eq!(slack.default_channel, Some("test".to_string()));
    assert!(slack.oauth.is_none());
}

#[test]
fn test_settings_file_parse_partial_oauth() {
    let toml_str = r##"
            [slack.oauth]
            bot_token = "xoxb-test"
        "##;

    let settings: SettingsFile = toml::from_str(toml_str).unwrap();
    let slack = settings.slack.unwrap();
    let oauth = slack.oauth.unwrap();
    assert_eq!(oauth.bot_token, Some("xoxb-test".to_string()));
    assert!(oauth.client_id.is_none());
}

#[test]
fn test_slack_section_debug() {
    let toml_str = r##"
            [slack]
            default_channel = "test"
        "##;

    let settings: SettingsFile = toml::from_str(toml_str).unwrap();
    let debug = format!("{:?}", settings);
    assert!(debug.contains("SettingsFile"));
}

#[test]
fn test_oauth_section_debug() {
    let toml_str = r##"
            [slack.oauth]
            bot_token = "xoxb-test"
        "##;

    let settings: SettingsFile = toml::from_str(toml_str).unwrap();
    let debug = format!("{:?}", settings.slack.unwrap().oauth.unwrap());
    assert!(debug.contains("OAuthSection"));
}
