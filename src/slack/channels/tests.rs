use super::*;

#[test]
fn test_channel_response_to_slack_channel_full() {
    let response = ChannelResponse {
        id: "C12345".to_string(),
        name: "general".to_string(),
        is_private: Some(true),
        is_member: Some(true),
        topic: Some(TopicResponse {
            value: "Channel topic".to_string(),
        }),
        purpose: Some(TopicResponse {
            value: "Channel purpose".to_string(),
        }),
        num_members: Some(42),
        created: Some(1704067200),
    };

    let channel = SlackChannel::from(response);
    assert_eq!(channel.id, "C12345");
    assert_eq!(channel.name, "general");
    assert!(channel.is_private);
    assert!(channel.is_member);
    assert_eq!(channel.topic, Some("Channel topic".to_string()));
    assert_eq!(channel.purpose, Some("Channel purpose".to_string()));
    assert_eq!(channel.num_members, Some(42));
    assert_eq!(channel.created, 1704067200);
}

#[test]
fn test_channel_response_to_slack_channel_minimal() {
    let response = ChannelResponse {
        id: "C12345".to_string(),
        name: "general".to_string(),
        is_private: None,
        is_member: None,
        topic: None,
        purpose: None,
        num_members: None,
        created: None,
    };

    let channel = SlackChannel::from(response);
    assert_eq!(channel.id, "C12345");
    assert_eq!(channel.name, "general");
    assert!(!channel.is_private);
    assert!(!channel.is_member);
    assert!(channel.topic.is_none());
    assert!(channel.purpose.is_none());
    assert!(channel.num_members.is_none());
    assert_eq!(channel.created, 0);
}

#[test]
fn test_channel_response_empty_topic_filtered() {
    let response = ChannelResponse {
        id: "C12345".to_string(),
        name: "general".to_string(),
        is_private: None,
        is_member: None,
        topic: Some(TopicResponse {
            value: "".to_string(),
        }),
        purpose: Some(TopicResponse {
            value: "".to_string(),
        }),
        num_members: None,
        created: None,
    };

    let channel = SlackChannel::from(response);
    assert!(channel.topic.is_none());
    assert!(channel.purpose.is_none());
}

#[test]
fn test_user_response_to_slack_user_full() {
    let response = UserResponse {
        id: "U12345".to_string(),
        team_id: Some("T12345".to_string()),
        name: "alice".to_string(),
        real_name: Some("Alice Smith".to_string()),
        is_bot: Some(false),
        deleted: Some(false),
        tz: Some("America/New_York".to_string()),
    };

    let user = SlackUser::from(response);
    assert_eq!(user.id, "U12345");
    assert_eq!(user.team_id, Some("T12345".to_string()));
    assert_eq!(user.name, "alice");
    assert_eq!(user.real_name, Some("Alice Smith".to_string()));
    assert!(!user.is_bot);
    assert!(!user.deleted);
    assert_eq!(user.tz, Some("America/New_York".to_string()));
}

#[test]
fn test_user_response_to_slack_user_minimal() {
    let response = UserResponse {
        id: "U12345".to_string(),
        team_id: None,
        name: "alice".to_string(),
        real_name: None,
        is_bot: None,
        deleted: None,
        tz: None,
    };

    let user = SlackUser::from(response);
    assert_eq!(user.id, "U12345");
    assert!(user.team_id.is_none());
    assert_eq!(user.name, "alice");
    assert!(user.real_name.is_none());
    assert!(!user.is_bot);
    assert!(!user.deleted);
    assert!(user.tz.is_none());
}

#[test]
fn test_user_response_to_slack_user_bot() {
    let response = UserResponse {
        id: "U12345".to_string(),
        team_id: None,
        name: "bot".to_string(),
        real_name: None,
        is_bot: Some(true),
        deleted: Some(true),
        tz: None,
    };

    let user = SlackUser::from(response);
    assert!(user.is_bot);
    assert!(user.deleted);
}

#[test]
fn test_user_cache_serialize_deserialize() {
    let mut users = HashMap::new();
    users.insert("U12345".to_string(), "alice".to_string());
    users.insert("U67890".to_string(), "bob".to_string());

    let cache = UserCache {
        created: 1704067200,
        users,
    };

    let json = serde_json::to_string(&cache).unwrap();
    let deserialized: UserCache = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.created, 1704067200);
    assert_eq!(deserialized.users.len(), 2);
    assert_eq!(deserialized.users.get("U12345"), Some(&"alice".to_string()));
}

#[test]
fn test_user_cache_path_is_some() {
    // Should return Some on systems with a home directory
    let path = user_cache_path();
    if let Some(p) = path {
        assert!(p.to_string_lossy().contains("slack_users_cache.json"));
    }
}

#[test]
fn test_conversations_list_response_deserialize() {
    let json = r#"{
            "channels": [
                {"id": "C12345", "name": "general", "is_private": false, "is_member": true}
            ],
            "response_metadata": {"next_cursor": "abc123"}
        }"#;

    let response: ConversationsListResponse = serde_json::from_str(json).unwrap();
    assert_eq!(response.channels.len(), 1);
    assert_eq!(response.channels[0].id, "C12345");
    assert_eq!(
        response.response_metadata.unwrap().next_cursor,
        Some("abc123".to_string())
    );
}

#[test]
fn test_conversations_list_response_no_cursor() {
    let json = r#"{
            "channels": [
                {"id": "C12345", "name": "general"}
            ]
        }"#;

    let response: ConversationsListResponse = serde_json::from_str(json).unwrap();
    assert_eq!(response.channels.len(), 1);
    assert!(response.response_metadata.is_none());
}

#[test]
fn test_conversations_info_response_deserialize() {
    let json = r#"{
            "channel": {
                "id": "C12345",
                "name": "general",
                "is_private": true,
                "is_member": true,
                "topic": {"value": "Discussion"},
                "purpose": {"value": "General chat"},
                "num_members": 100,
                "created": 1704067200
            }
        }"#;

    let response: ConversationsInfoResponse = serde_json::from_str(json).unwrap();
    assert_eq!(response.channel.id, "C12345");
    assert_eq!(response.channel.name, "general");
}

#[test]
fn test_users_list_response_deserialize() {
    let json = r#"{
            "members": [
                {"id": "U12345", "name": "alice", "real_name": "Alice"},
                {"id": "U67890", "name": "bob"}
            ]
        }"#;

    let response: UsersListResponse = serde_json::from_str(json).unwrap();
    assert_eq!(response.members.len(), 2);
    assert_eq!(response.members[0].id, "U12345");
    assert_eq!(response.members[1].name, "bob");
}

#[test]
fn test_topic_response_deserialize() {
    let json = r#"{"value": "Test topic"}"#;
    let topic: TopicResponse = serde_json::from_str(json).unwrap();
    assert_eq!(topic.value, "Test topic");
}

#[test]
fn test_response_metadata_deserialize() {
    let json = r#"{"next_cursor": "cursor123"}"#;
    let meta: ResponseMetadata = serde_json::from_str(json).unwrap();
    assert_eq!(meta.next_cursor, Some("cursor123".to_string()));
}

#[test]
fn test_response_metadata_empty_cursor() {
    let json = r#"{}"#;
    let meta: ResponseMetadata = serde_json::from_str(json).unwrap();
    assert!(meta.next_cursor.is_none());
}
