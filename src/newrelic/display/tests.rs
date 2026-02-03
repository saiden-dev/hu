use super::*;

#[test]
fn test_format_time_none() {
    assert_eq!(format_time(None), "-");
}

#[test]
fn test_format_time_days_ago() {
    let now = chrono::Utc::now();
    let two_days_ago = now - chrono::Duration::days(2);
    let ms = two_days_ago.timestamp() * 1000;
    assert_eq!(format_time(Some(ms)), "2d ago");
}

#[test]
fn test_format_time_hours_ago() {
    let now = chrono::Utc::now();
    let two_hours_ago = now - chrono::Duration::hours(2);
    let ms = two_hours_ago.timestamp() * 1000;
    assert_eq!(format_time(Some(ms)), "2h ago");
}

#[test]
fn test_format_time_minutes_ago() {
    let now = chrono::Utc::now();
    let five_mins_ago = now - chrono::Duration::minutes(5);
    let ms = five_mins_ago.timestamp() * 1000;
    assert_eq!(format_time(Some(ms)), "5m ago");
}

#[test]
fn test_format_time_just_now() {
    let now = chrono::Utc::now();
    let ms = now.timestamp() * 1000;
    assert_eq!(format_time(Some(ms)), "just now");
}

#[test]
fn test_truncate_short() {
    assert_eq!(truncate("hello", 10), "hello");
}

#[test]
fn test_truncate_exact() {
    assert_eq!(truncate("hello", 5), "hello");
}

#[test]
fn test_truncate_long() {
    assert_eq!(truncate("hello world", 8), "hello...");
}

#[test]
fn test_priority_color_critical() {
    assert_eq!(priority_color("CRITICAL"), Color::Red);
    assert_eq!(priority_color("critical"), Color::Red);
}

#[test]
fn test_priority_color_high() {
    assert_eq!(priority_color("HIGH"), Color::Yellow);
    assert_eq!(priority_color("high"), Color::Yellow);
}

#[test]
fn test_priority_color_medium() {
    assert_eq!(priority_color("MEDIUM"), Color::Blue);
}

#[test]
fn test_priority_color_other() {
    assert_eq!(priority_color("LOW"), Color::White);
    assert_eq!(priority_color("unknown"), Color::White);
}

#[test]
fn test_state_color_closed() {
    assert_eq!(state_color("CLOSED"), Color::Green);
    assert_eq!(state_color("closed"), Color::Green);
}

#[test]
fn test_state_color_active() {
    assert_eq!(state_color("ACTIVATED"), Color::Red);
    assert_eq!(state_color("ACTIVE"), Color::Red);
    assert_eq!(state_color("active"), Color::Red);
}

#[test]
fn test_state_color_other() {
    assert_eq!(state_color("PENDING"), Color::White);
}

#[test]
fn test_format_json_value_null() {
    assert_eq!(format_json_value(&serde_json::Value::Null), "-");
}

#[test]
fn test_format_json_value_string() {
    assert_eq!(
        format_json_value(&serde_json::Value::String("hello".to_string())),
        "hello"
    );
}

#[test]
fn test_format_json_value_number() {
    assert_eq!(format_json_value(&serde_json::json!(42)), "42");
    assert_eq!(format_json_value(&serde_json::json!(3.14)), "3.14");
}

#[test]
fn test_format_json_value_bool() {
    assert_eq!(format_json_value(&serde_json::json!(true)), "true");
    assert_eq!(format_json_value(&serde_json::json!(false)), "false");
}

#[test]
fn test_format_json_value_array() {
    let arr = serde_json::json!([1, 2, 3]);
    assert_eq!(format_json_value(&arr), "[1,2,3]");
}

#[test]
fn test_output_issues_empty() {
    let issues: Vec<Issue> = vec![];
    let result = output_issues(&issues, OutputFormat::Table);
    assert!(result.is_ok());
}

#[test]
fn test_output_issues_json() {
    let issues = vec![Issue {
        issue_id: "12345678901234567890".to_string(),
        priority: "HIGH".to_string(),
        state: "ACTIVATED".to_string(),
        title: vec!["Test Issue".to_string()],
        entity_names: vec!["my-service".to_string()],
        created_at: Some(chrono::Utc::now().timestamp() * 1000),
        closed_at: None,
        activated_at: Some(chrono::Utc::now().timestamp() * 1000),
    }];
    let result = output_issues(&issues, OutputFormat::Json);
    assert!(result.is_ok());
}

#[test]
fn test_output_incidents_empty() {
    let incidents: Vec<Incident> = vec![];
    let result = output_incidents(&incidents, OutputFormat::Table);
    assert!(result.is_ok());
}

#[test]
fn test_output_incidents_json() {
    let incidents = vec![Incident {
        incident_id: "12345678901234567890".to_string(),
        priority: "CRITICAL".to_string(),
        state: "CLOSED".to_string(),
        title: "Test Incident".to_string(),
        account_ids: vec![12345],
        created_at: Some(chrono::Utc::now().timestamp() * 1000),
        closed_at: None,
    }];
    let result = output_incidents(&incidents, OutputFormat::Json);
    assert!(result.is_ok());
}

#[test]
fn test_output_nrql_empty() {
    let results: Vec<serde_json::Value> = vec![];
    let result = output_nrql(&results, OutputFormat::Table);
    assert!(result.is_ok());
}

#[test]
fn test_output_nrql_json() {
    let results = vec![serde_json::json!({"count": 42, "name": "test"})];
    let result = output_nrql(&results, OutputFormat::Json);
    assert!(result.is_ok());
}

#[test]
fn test_output_nrql_table() {
    let results = vec![
        serde_json::json!({"count": 42, "name": "test1"}),
        serde_json::json!({"count": 10, "name": "test2"}),
    ];
    let result = output_nrql(&results, OutputFormat::Table);
    assert!(result.is_ok());
}

#[test]
fn test_output_nrql_table_non_object() {
    // Test fallback to JSON when results are not objects
    let results = vec![serde_json::json!("string value"), serde_json::json!(123)];
    let result = output_nrql(&results, OutputFormat::Table);
    assert!(result.is_ok());
}

#[test]
fn test_output_nrql_table_missing_keys() {
    // Test when second object is missing keys from first
    let results = vec![
        serde_json::json!({"a": 1, "b": 2}),
        serde_json::json!({"a": 3}), // missing "b"
    ];
    let result = output_nrql(&results, OutputFormat::Table);
    assert!(result.is_ok());
}

#[test]
fn test_output_issues_table_with_data() {
    let issues = vec![Issue {
        issue_id: "123456789012345678901234567890".to_string(), // long ID
        priority: "MEDIUM".to_string(),
        state: "CREATED".to_string(),
        title: vec![
            "This is a very long title that should be truncated for display purposes".to_string(),
        ],
        entity_names: vec!["service-one".to_string(), "service-two".to_string()],
        created_at: Some(chrono::Utc::now().timestamp() * 1000 - 86400000), // 1 day ago
        closed_at: None,
        activated_at: None,
    }];
    let result = output_issues(&issues, OutputFormat::Table);
    assert!(result.is_ok());
}

#[test]
fn test_output_incidents_table_with_data() {
    let incidents = vec![Incident {
        incident_id: "INC-VERYLONGIDTHATWILLBETRUNCATED".to_string(),
        priority: "LOW".to_string(),
        state: "PENDING".to_string(),
        title: "This incident title is also quite long and needs truncation".to_string(),
        account_ids: vec![1, 2, 3],
        created_at: Some(chrono::Utc::now().timestamp() * 1000),
        closed_at: Some(chrono::Utc::now().timestamp() * 1000),
    }];
    let result = output_incidents(&incidents, OutputFormat::Table);
    assert!(result.is_ok());
}

#[test]
fn test_format_time_invalid_timestamp() {
    // Very old timestamp that might be invalid
    let result = format_time(Some(-1000000000000000));
    // Should still return something, not crash
    assert!(!result.is_empty());
}

#[test]
fn test_format_time_out_of_range() {
    // Timestamp so large that DateTime::from_timestamp returns None
    // i64::MAX / 1000 = ~292 billion years, way beyond chrono's range
    let result = format_time(Some(i64::MAX));
    assert_eq!(result, "-");
}

#[test]
fn test_format_json_value_object() {
    let obj = serde_json::json!({"key": "value"});
    let formatted = format_json_value(&obj);
    assert!(formatted.contains("key"));
    assert!(formatted.contains("value"));
}

#[test]
fn test_truncate_empty() {
    assert_eq!(truncate("", 10), "");
}

#[test]
fn test_truncate_very_short_max() {
    // Test edge case where max_len is very small
    assert_eq!(truncate("hello", 3), "...");
}

#[test]
fn test_truncate_zero() {
    // Test edge case where max_len is 0
    let result = truncate("hello", 0);
    // Should handle gracefully
    assert!(result.len() <= 3); // "..." or empty
}

#[test]
fn test_output_config_status_configured() {
    let config = super::super::config::NewRelicConfig {
        api_key: Some("NRAK-test".to_string()),
        account_id: Some(12345),
    };
    // Just verify it doesn't panic
    output_config_status(&config);
}

#[test]
fn test_output_config_status_not_configured() {
    let config = super::super::config::NewRelicConfig {
        api_key: None,
        account_id: None,
    };
    output_config_status(&config);
}

#[test]
fn test_output_config_status_partial() {
    let config = super::super::config::NewRelicConfig {
        api_key: Some("NRAK-partial".to_string()),
        account_id: None,
    };
    output_config_status(&config);
}
