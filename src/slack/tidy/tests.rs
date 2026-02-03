use super::*;

#[test]
fn test_user_info_creation() {
    let info = UserInfo {
        user_id: "U12345".to_string(),
        name: "Alice".to_string(),
        full_name: "Alice Smith".to_string(),
    };
    assert_eq!(info.user_id, "U12345");
    assert_eq!(info.name, "Alice");
    assert_eq!(info.full_name, "Alice Smith");
}

#[test]
fn test_tidy_result_debug() {
    let result = TidyResult {
        channel_name: "general".to_string(),
        action: TidyAction::MarkedRead,
    };
    let debug = format!("{:?}", result);
    assert!(debug.contains("general"));
    assert!(debug.contains("MarkedRead"));
}

#[test]
fn test_tidy_action_skipped_debug() {
    let action = TidyAction::Skipped;
    assert_eq!(format!("{:?}", action), "Skipped");
}

#[test]
fn test_tidy_action_marked_read_debug() {
    let action = TidyAction::MarkedRead;
    assert_eq!(format!("{:?}", action), "MarkedRead");
}

#[test]
fn test_tidy_action_has_mention_debug() {
    let action = TidyAction::HasMention("@alice mentioned you".to_string());
    let debug = format!("{:?}", action);
    assert!(debug.contains("HasMention"));
    assert!(debug.contains("@alice mentioned you"));
}

#[test]
fn test_get_display_name_with_name() {
    let channel = ChannelListItem {
        id: "C12345".to_string(),
        name: Some("general".to_string()),
        user: None,
        is_member: Some(true),
        is_im: None,
    };
    assert_eq!(get_display_name(&channel), "general");
}

#[test]
fn test_get_display_name_dm() {
    let channel = ChannelListItem {
        id: "D12345".to_string(),
        name: None,
        user: Some("U67890".to_string()),
        is_member: None,
        is_im: Some(true),
    };
    assert_eq!(get_display_name(&channel), "DM:U67890");
}

#[test]
fn test_get_display_name_fallback_to_id() {
    let channel = ChannelListItem {
        id: "G12345".to_string(),
        name: None,
        user: None,
        is_member: None,
        is_im: None,
    };
    assert_eq!(get_display_name(&channel), "G12345");
}

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
fn test_find_mention_direct_user_mention() {
    let messages = vec![HistoryMessage {
        ts: "1704067200.123456".to_string(),
        text: Some("Hey <@U12345> check this out".to_string()),
    }];
    let user_info = UserInfo {
        user_id: "U12345".to_string(),
        name: "Alice".to_string(),
        full_name: "Alice Smith".to_string(),
    };

    let result = find_mention(&messages, &user_info);
    assert!(result.is_some());
    assert!(result.unwrap().contains("@mention"));
}

#[test]
fn test_find_mention_name_match() {
    let messages = vec![HistoryMessage {
        ts: "1704067200.123456".to_string(),
        text: Some("Hey Alice, how are you?".to_string()),
    }];
    let user_info = UserInfo {
        user_id: "U12345".to_string(),
        name: "Alice".to_string(),
        full_name: "Alice Smith".to_string(),
    };

    let result = find_mention(&messages, &user_info);
    assert!(result.is_some());
    assert!(result.unwrap().contains("name 'Alice'"));
}

#[test]
fn test_find_mention_full_name_match() {
    let messages = vec![HistoryMessage {
        ts: "1704067200.123456".to_string(),
        text: Some("I talked to Alice Smith yesterday".to_string()),
    }];
    let user_info = UserInfo {
        user_id: "U12345".to_string(),
        name: "Bob".to_string(),
        full_name: "Alice Smith".to_string(),
    };

    let result = find_mention(&messages, &user_info);
    assert!(result.is_some());
    assert!(result.unwrap().contains("full name"));
}

#[test]
fn test_find_mention_case_insensitive() {
    let messages = vec![HistoryMessage {
        ts: "1704067200.123456".to_string(),
        text: Some("ALICE is here".to_string()),
    }];
    let user_info = UserInfo {
        user_id: "U12345".to_string(),
        name: "alice".to_string(),
        full_name: "Alice Smith".to_string(),
    };

    let result = find_mention(&messages, &user_info);
    assert!(result.is_some());
}

#[test]
fn test_find_mention_no_match() {
    let messages = vec![HistoryMessage {
        ts: "1704067200.123456".to_string(),
        text: Some("Just a regular message".to_string()),
    }];
    let user_info = UserInfo {
        user_id: "U12345".to_string(),
        name: "Alice".to_string(),
        full_name: "Alice Smith".to_string(),
    };

    let result = find_mention(&messages, &user_info);
    assert!(result.is_none());
}

#[test]
fn test_find_mention_empty_messages() {
    let messages: Vec<HistoryMessage> = vec![];
    let user_info = UserInfo {
        user_id: "U12345".to_string(),
        name: "Alice".to_string(),
        full_name: "Alice Smith".to_string(),
    };

    let result = find_mention(&messages, &user_info);
    assert!(result.is_none());
}

#[test]
fn test_find_mention_message_without_text() {
    let messages = vec![HistoryMessage {
        ts: "1704067200.123456".to_string(),
        text: None,
    }];
    let user_info = UserInfo {
        user_id: "U12345".to_string(),
        name: "Alice".to_string(),
        full_name: "Alice Smith".to_string(),
    };

    let result = find_mention(&messages, &user_info);
    assert!(result.is_none());
}

#[test]
fn test_conversations_list_response_deserialize() {
    let json = r#"{
            "channels": [
                {"id": "C12345", "name": "general", "is_member": true},
                {"id": "D67890", "user": "U99999", "is_im": true}
            ],
            "response_metadata": {"next_cursor": "abc123"}
        }"#;

    let response: ConversationsListResponse = serde_json::from_str(json).unwrap();
    assert_eq!(response.channels.len(), 2);
    assert_eq!(response.channels[0].id, "C12345");
    assert_eq!(response.channels[1].user, Some("U99999".to_string()));
}

#[test]
fn test_channel_list_item_deserialize() {
    let json = r#"{"id": "C12345", "name": "test", "is_member": true, "is_im": false}"#;
    let item: ChannelListItem = serde_json::from_str(json).unwrap();
    assert_eq!(item.id, "C12345");
    assert_eq!(item.name, Some("test".to_string()));
    assert_eq!(item.is_member, Some(true));
    assert_eq!(item.is_im, Some(false));
}

#[test]
fn test_conversations_info_response_deserialize() {
    let json = r#"{
            "channel": {
                "last_read": "1704067200.000000",
                "latest": {"ts": "1704067300.000000"}
            }
        }"#;

    let response: ConversationsInfoResponse = serde_json::from_str(json).unwrap();
    assert_eq!(
        response.channel.last_read,
        Some("1704067200.000000".to_string())
    );
    assert_eq!(response.channel.latest.unwrap().ts, "1704067300.000000");
}

#[test]
fn test_history_response_deserialize() {
    let json = r#"{
            "messages": [
                {"ts": "1704067200.123456", "text": "Hello"},
                {"ts": "1704067100.123456"}
            ]
        }"#;

    let response: HistoryResponse = serde_json::from_str(json).unwrap();
    assert_eq!(response.messages.len(), 2);
    assert_eq!(response.messages[0].ts, "1704067200.123456");
    assert_eq!(response.messages[0].text, Some("Hello".to_string()));
}

#[test]
fn test_mark_request_serialize() {
    let request = MarkRequest {
        channel: "C12345".to_string(),
        ts: "1704067200.123456".to_string(),
    };

    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("C12345"));
    assert!(json.contains("1704067200.123456"));
}

#[test]
fn test_mark_response_deserialize() {
    let json = r#"{}"#;
    let response: MarkResponse = serde_json::from_str(json).unwrap();
    // Just verify it deserializes without error
    let _ = response;
}

#[test]
fn test_response_metadata_deserialize() {
    let json = r#"{"next_cursor": "cursor123"}"#;
    let meta: ResponseMetadata = serde_json::from_str(json).unwrap();
    assert_eq!(meta.next_cursor, Some("cursor123".to_string()));
}

#[test]
fn test_channel_info_item_deserialize() {
    let json = r#"{"last_read": "1704067200.000000"}"#;
    let item: ChannelInfoItem = serde_json::from_str(json).unwrap();
    assert_eq!(item.last_read, Some("1704067200.000000".to_string()));
    assert!(item.latest.is_none());
}

#[test]
fn test_latest_message_deserialize() {
    let json = r#"{"ts": "1704067200.123456"}"#;
    let latest: LatestMessage = serde_json::from_str(json).unwrap();
    assert_eq!(latest.ts, "1704067200.123456");
}
