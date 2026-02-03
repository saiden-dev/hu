use super::*;

#[test]
fn test_truncate_short_string() {
    assert_eq!(truncate("hello", 10), "hello");
}

#[test]
fn test_truncate_exact_length() {
    assert_eq!(truncate("hello", 5), "hello");
}

#[test]
fn test_truncate_long_string() {
    assert_eq!(truncate("hello world", 8), "hello...");
}

#[test]
fn test_truncate_very_short_max() {
    assert_eq!(truncate("hello", 3), "...");
}

#[test]
fn test_clean_message_text_user_mention_with_display() {
    let lookup = HashMap::new();
    assert_eq!(
        clean_message_text("<@U12345|John Doe>", &lookup),
        "@John Doe"
    );
}

#[test]
fn test_clean_message_text_user_mention_with_lookup() {
    let mut lookup = HashMap::new();
    lookup.insert("U12345".to_string(), "johndoe".to_string());
    assert_eq!(clean_message_text("<@U12345>", &lookup), "@johndoe");
}

#[test]
fn test_clean_message_text_user_mention_without_lookup() {
    let lookup = HashMap::new();
    assert_eq!(clean_message_text("<@U12345>", &lookup), "@U12345");
}

#[test]
fn test_clean_message_text_channel_mention() {
    let lookup = HashMap::new();
    assert_eq!(clean_message_text("<#C12345|general>", &lookup), "#general");
}

#[test]
fn test_clean_message_text_channel_mention_no_name() {
    let lookup = HashMap::new();
    assert_eq!(clean_message_text("<#C12345>", &lookup), "#C12345");
}

#[test]
fn test_clean_message_text_special_mention() {
    let lookup = HashMap::new();
    assert_eq!(clean_message_text("<!here>", &lookup), "@here");
    assert_eq!(clean_message_text("<!channel>", &lookup), "@channel");
    assert_eq!(clean_message_text("<!everyone>", &lookup), "@everyone");
}

#[test]
fn test_clean_message_text_url_with_display() {
    let lookup = HashMap::new();
    assert_eq!(
        clean_message_text("<https://example.com|Example Site>", &lookup),
        "Example Site"
    );
}

#[test]
fn test_clean_message_text_plain_url() {
    let lookup = HashMap::new();
    assert_eq!(
        clean_message_text("<https://example.com>", &lookup),
        "https://example.com"
    );
}

#[test]
fn test_clean_message_text_mixed() {
    let mut lookup = HashMap::new();
    lookup.insert("U12345".to_string(), "bob".to_string());
    assert_eq!(
        clean_message_text("Hey <@U12345>, check <#C99999|dev>!", &lookup),
        "Hey @bob, check #dev!"
    );
}

#[test]
fn test_format_channel_name_regular() {
    let lookup = HashMap::new();
    assert_eq!(format_channel_name("general", &lookup), "#general");
}

#[test]
fn test_format_channel_name_mpdm() {
    let lookup = HashMap::new();
    assert_eq!(
        format_channel_name("mpdm-alice--bob--charlie-1", &lookup),
        "@alice, @bob, @charlie"
    );
}

#[test]
fn test_format_channel_name_user_id_with_lookup() {
    let mut lookup = HashMap::new();
    lookup.insert("U04H482TK6Z".to_string(), "alice".to_string());
    assert_eq!(format_channel_name("U04H482TK6Z", &lookup), "@alice");
}

#[test]
fn test_format_channel_name_user_id_without_lookup() {
    let lookup = HashMap::new();
    assert_eq!(format_channel_name("U04H482TK6Z", &lookup), "DM");
}

#[test]
fn test_format_timestamp_valid() {
    // 2024-01-01 00:00:00 UTC
    let result = format_timestamp("1704067200.123456");
    assert_eq!(result, "2024-01-01 00:00");
}

#[test]
fn test_format_timestamp_no_decimal() {
    let result = format_timestamp("1704067200");
    assert_eq!(result, "2024-01-01 00:00");
}

#[test]
fn test_format_timestamp_invalid() {
    let result = format_timestamp("invalid");
    assert_eq!(result, "invalid");
}

#[test]
fn test_output_channels_empty() {
    // Just verify it doesn't panic
    let channels: Vec<SlackChannel> = vec![];
    let result = output_channels(&channels, OutputFormat::Table);
    assert!(result.is_ok());
}

#[test]
fn test_output_channels_json() {
    let channels = vec![SlackChannel {
        id: "C12345".to_string(),
        name: "general".to_string(),
        is_private: false,
        is_member: true,
        topic: Some("General discussion".to_string()),
        purpose: None,
        num_members: Some(100),
        created: 1704067200,
    }];
    let result = output_channels(&channels, OutputFormat::Json);
    assert!(result.is_ok());
}

#[test]
fn test_output_channel_detail_table() {
    let channel = SlackChannel {
        id: "C12345".to_string(),
        name: "general".to_string(),
        is_private: true,
        is_member: false,
        topic: Some("Topic".to_string()),
        purpose: Some("Purpose".to_string()),
        num_members: Some(50),
        created: 1704067200,
    };
    let result = output_channel_detail(&channel, OutputFormat::Table);
    assert!(result.is_ok());
}

#[test]
fn test_output_messages_empty() {
    let messages: Vec<SlackMessage> = vec![];
    let result = output_messages(&messages, "general", OutputFormat::Table);
    assert!(result.is_ok());
}

#[test]
fn test_output_messages_json() {
    let messages = vec![SlackMessage {
        msg_type: "message".to_string(),
        user: Some("U12345".to_string()),
        text: "Hello world".to_string(),
        ts: "1704067200.123456".to_string(),
        thread_ts: None,
        reply_count: Some(5),
        username: Some("alice".to_string()),
    }];
    let result = output_messages(&messages, "general", OutputFormat::Json);
    assert!(result.is_ok());
}

#[test]
fn test_output_users_empty() {
    let users: Vec<SlackUser> = vec![];
    let result = output_users(&users, OutputFormat::Table);
    assert!(result.is_ok());
}

#[test]
fn test_output_users_json() {
    let users = vec![SlackUser {
        id: "U12345".to_string(),
        team_id: Some("T12345".to_string()),
        name: "alice".to_string(),
        real_name: Some("Alice Smith".to_string()),
        is_bot: false,
        deleted: false,
        tz: Some("America/New_York".to_string()),
    }];
    let result = output_users(&users, OutputFormat::Json);
    assert!(result.is_ok());
}

#[test]
fn test_output_search_results_empty() {
    let results = SlackSearchResult {
        total: 0,
        matches: vec![],
    };
    let lookup = HashMap::new();
    let result = output_search_results(&results, OutputFormat::Table, &lookup);
    assert!(result.is_ok());
}

#[test]
fn test_output_search_results_json() {
    use crate::slack::types::{SlackSearchChannel, SlackSearchMatch};
    let results = SlackSearchResult {
        total: 1,
        matches: vec![SlackSearchMatch {
            channel: SlackSearchChannel {
                id: "C12345".to_string(),
                name: "general".to_string(),
            },
            user: Some("U12345".to_string()),
            username: Some("alice".to_string()),
            text: "Hello world".to_string(),
            ts: "1704067200.123456".to_string(),
            permalink: Some("https://slack.com/...".to_string()),
        }],
    };
    let lookup = HashMap::new();
    let result = output_search_results(&results, OutputFormat::Json, &lookup);
    assert!(result.is_ok());
}

#[test]
fn test_output_channels_table_with_data() {
    let channels = vec![
        SlackChannel {
            id: "C12345".to_string(),
            name: "general".to_string(),
            is_private: false,
            is_member: true,
            topic: Some("General discussion".to_string()),
            purpose: None,
            num_members: Some(100),
            created: 1704067200,
        },
        SlackChannel {
            id: "C67890".to_string(),
            name: "private-team".to_string(),
            is_private: true,
            is_member: false,
            topic: None,
            purpose: None,
            num_members: None,
            created: 1704067200,
        },
    ];
    let result = output_channels(&channels, OutputFormat::Table);
    assert!(result.is_ok());
}

#[test]
fn test_output_channel_detail_json() {
    let channel = SlackChannel {
        id: "C12345".to_string(),
        name: "general".to_string(),
        is_private: false,
        is_member: true,
        topic: None,
        purpose: None,
        num_members: None,
        created: 1704067200,
    };
    let result = output_channel_detail(&channel, OutputFormat::Json);
    assert!(result.is_ok());
}

#[test]
fn test_output_channel_detail_table_public() {
    // Tests the "public" branch (line 166) in table output
    let channel = SlackChannel {
        id: "C12345".to_string(),
        name: "general".to_string(),
        is_private: false, // public channel
        is_member: true,
        topic: Some("General chat".to_string()),
        purpose: Some("For general discussion".to_string()),
        num_members: Some(50),
        created: 1704067200,
    };
    let result = output_channel_detail(&channel, OutputFormat::Table);
    assert!(result.is_ok());
}

#[test]
fn test_output_messages_table_with_data() {
    let messages = vec![
        SlackMessage {
            msg_type: "message".to_string(),
            user: Some("U12345".to_string()),
            text: "Hello world".to_string(),
            ts: "1704067200.123456".to_string(),
            thread_ts: None,
            reply_count: Some(5),
            username: Some("alice".to_string()),
        },
        SlackMessage {
            msg_type: "message".to_string(),
            user: None,
            text: "Another message".to_string(),
            ts: "1704067201.123456".to_string(),
            thread_ts: None,
            reply_count: None,
            username: None,
        },
    ];
    let result = output_messages(&messages, "general", OutputFormat::Table);
    assert!(result.is_ok());
}

#[test]
fn test_output_users_table_with_data() {
    let users = vec![
        SlackUser {
            id: "U12345".to_string(),
            team_id: Some("T12345".to_string()),
            name: "alice".to_string(),
            real_name: Some("Alice Smith".to_string()),
            is_bot: false,
            deleted: false,
            tz: Some("America/New_York".to_string()),
        },
        SlackUser {
            id: "U67890".to_string(),
            team_id: None,
            name: "bob".to_string(),
            real_name: None,
            is_bot: true,
            deleted: false,
            tz: None,
        },
    ];
    let result = output_users(&users, OutputFormat::Table);
    assert!(result.is_ok());
}

#[test]
fn test_output_search_results_table_with_data() {
    use crate::slack::types::{SlackSearchChannel, SlackSearchMatch};
    let results = SlackSearchResult {
        total: 100,
        matches: vec![
            SlackSearchMatch {
                channel: SlackSearchChannel {
                    id: "C12345".to_string(),
                    name: "general".to_string(),
                },
                user: Some("U12345".to_string()),
                username: Some("alice".to_string()),
                text: "Hello world".to_string(),
                ts: "1704067200.123456".to_string(),
                permalink: Some("https://slack.com/...".to_string()),
            },
            SlackSearchMatch {
                channel: SlackSearchChannel {
                    id: "C67890".to_string(),
                    name: "mpdm-alice--bob-1".to_string(),
                },
                user: None,
                username: None,
                text: "<@U12345|Alice> mentioned <#C99999|dev>".to_string(),
                ts: "1704067201.123456".to_string(),
                permalink: None,
            },
        ],
    };
    let mut lookup = HashMap::new();
    lookup.insert("U12345".to_string(), "alice".to_string());
    let result = output_search_results(&results, OutputFormat::Table, &lookup);
    assert!(result.is_ok());
}

#[test]
fn test_output_config_status_all_configured() {
    output_config_status(true, true, Some("Acme Corp"), "#general");
}

#[test]
fn test_output_config_status_not_configured() {
    output_config_status(false, false, None, "");
}

#[test]
fn test_output_config_status_partial() {
    output_config_status(true, false, Some("My Team"), "");
}
