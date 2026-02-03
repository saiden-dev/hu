use super::*;
use crate::sentry::types::{EventUser, IssueMetadata, ProjectInfo};

#[test]
fn test_time_ago_days() {
    let now = chrono::Utc::now();
    let two_days_ago = now - chrono::Duration::days(2);
    let ts = two_days_ago.to_rfc3339();
    assert_eq!(time_ago(&ts), "2d ago");
}

#[test]
fn test_time_ago_hours() {
    let now = chrono::Utc::now();
    let two_hours_ago = now - chrono::Duration::hours(2);
    let ts = two_hours_ago.to_rfc3339();
    assert_eq!(time_ago(&ts), "2h ago");
}

#[test]
fn test_time_ago_minutes() {
    let now = chrono::Utc::now();
    let five_mins_ago = now - chrono::Duration::minutes(5);
    let ts = five_mins_ago.to_rfc3339();
    assert_eq!(time_ago(&ts), "5m ago");
}

#[test]
fn test_time_ago_just_now() {
    let now = chrono::Utc::now();
    let ts = now.to_rfc3339();
    assert_eq!(time_ago(&ts), "just now");
}

#[test]
fn test_time_ago_invalid() {
    assert_eq!(time_ago("invalid"), "invalid");
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
fn test_level_color() {
    assert_eq!(level_color("error"), Color::Red);
    assert_eq!(level_color("warning"), Color::Yellow);
    assert_eq!(level_color("info"), Color::Blue);
    assert_eq!(level_color("debug"), Color::White);
}

#[test]
fn test_status_color() {
    assert_eq!(status_color("resolved"), Color::Green);
    assert_eq!(status_color("ignored"), Color::DarkGrey);
    assert_eq!(status_color("unresolved"), Color::White);
}

fn make_test_issue() -> Issue {
    Issue {
        id: "12345".to_string(),
        short_id: "PROJ-123".to_string(),
        title: "Test error".to_string(),
        culprit: "src/main.rs".to_string(),
        level: "error".to_string(),
        status: "unresolved".to_string(),
        platform: "rust".to_string(),
        count: "42".to_string(),
        user_count: 10,
        first_seen: chrono::Utc::now().to_rfc3339(),
        last_seen: chrono::Utc::now().to_rfc3339(),
        permalink: "https://sentry.io/issue/123".to_string(),
        is_subscribed: false,
        is_bookmarked: false,
        project: ProjectInfo {
            id: "1".to_string(),
            name: "Test Project".to_string(),
            slug: "test-project".to_string(),
        },
        metadata: IssueMetadata {
            error_type: "RuntimeError".to_string(),
            value: "Something went wrong".to_string(),
            filename: "main.rs".to_string(),
            function: "main".to_string(),
        },
    }
}

#[test]
fn test_output_issues_empty() {
    let issues: Vec<Issue> = vec![];
    let result = output_issues(&issues, OutputFormat::Table);
    assert!(result.is_ok());
}

#[test]
fn test_output_issues_table() {
    let issues = vec![make_test_issue()];
    let result = output_issues(&issues, OutputFormat::Table);
    assert!(result.is_ok());
}

#[test]
fn test_output_issues_json() {
    let issues = vec![make_test_issue()];
    let result = output_issues(&issues, OutputFormat::Json);
    assert!(result.is_ok());
}

#[test]
fn test_output_issue_detail_table() {
    let issue = make_test_issue();
    let result = output_issue_detail(&issue, OutputFormat::Table);
    assert!(result.is_ok());
}

#[test]
fn test_output_issue_detail_json() {
    let issue = make_test_issue();
    let result = output_issue_detail(&issue, OutputFormat::Json);
    assert!(result.is_ok());
}

#[test]
fn test_output_events_empty() {
    let events: Vec<Event> = vec![];
    let result = output_events(&events, OutputFormat::Table);
    assert!(result.is_ok());
}

#[test]
fn test_output_events_table() {
    let events = vec![Event {
        id: "abcdef123456".to_string(),
        title: "Test event".to_string(),
        message: "Error message".to_string(),
        platform: "rust".to_string(),
        date_created: Some(chrono::Utc::now().to_rfc3339()),
        user: Some(EventUser {
            id: Some("user123".to_string()),
            email: Some("test@example.com".to_string()),
            username: None,
            ip_address: None,
        }),
        tags: vec![],
    }];
    let result = output_events(&events, OutputFormat::Table);
    assert!(result.is_ok());
}

#[test]
fn test_output_events_json() {
    let events = vec![Event {
        id: "abcdef123456".to_string(),
        title: "Test event".to_string(),
        message: "".to_string(),
        platform: "".to_string(),
        date_created: None,
        user: None,
        tags: vec![],
    }];
    let result = output_events(&events, OutputFormat::Json);
    assert!(result.is_ok());
}

#[test]
fn test_output_issue_detail_empty_metadata() {
    // Test with empty culprit and empty metadata fields
    let issue = Issue {
        id: "12345".to_string(),
        short_id: "PROJ-456".to_string(),
        title: "Test error".to_string(),
        culprit: "".to_string(), // empty culprit
        level: "warning".to_string(),
        status: "resolved".to_string(),
        platform: "python".to_string(),
        count: "1".to_string(),
        user_count: 1,
        first_seen: chrono::Utc::now().to_rfc3339(),
        last_seen: chrono::Utc::now().to_rfc3339(),
        permalink: "https://sentry.io/issue/456".to_string(),
        is_subscribed: false,
        is_bookmarked: false,
        project: ProjectInfo {
            id: "2".to_string(),
            name: "Other Project".to_string(),
            slug: "other-project".to_string(),
        },
        metadata: IssueMetadata {
            error_type: "".to_string(), // empty
            value: "".to_string(),      // empty
            filename: "".to_string(),   // empty
            function: "".to_string(),   // empty
        },
    };
    let result = output_issue_detail(&issue, OutputFormat::Table);
    assert!(result.is_ok());
}

#[test]
fn test_output_issue_detail_partial_metadata() {
    // Test with only some metadata fields populated
    let issue = Issue {
        id: "12345".to_string(),
        short_id: "PROJ-789".to_string(),
        title: "Partial metadata".to_string(),
        culprit: "some/path.py".to_string(),
        level: "error".to_string(),
        status: "unresolved".to_string(),
        platform: "python".to_string(),
        count: "5".to_string(),
        user_count: 3,
        first_seen: chrono::Utc::now().to_rfc3339(),
        last_seen: chrono::Utc::now().to_rfc3339(),
        permalink: "https://sentry.io/issue/789".to_string(),
        is_subscribed: false,
        is_bookmarked: false,
        project: ProjectInfo {
            id: "3".to_string(),
            name: "Third Project".to_string(),
            slug: "third-project".to_string(),
        },
        metadata: IssueMetadata {
            error_type: "ValueError".to_string(),
            value: "".to_string(), // empty value
            filename: "".to_string(),
            function: "process_data".to_string(),
        },
    };
    let result = output_issue_detail(&issue, OutputFormat::Table);
    assert!(result.is_ok());
}

#[test]
fn test_output_events_user_variants() {
    // Test event with username instead of email
    let events = vec![
        Event {
            id: "event1234567890".to_string(),
            title: "Event with username".to_string(),
            message: "Has message".to_string(),
            platform: "rust".to_string(),
            date_created: Some(chrono::Utc::now().to_rfc3339()),
            user: Some(EventUser {
                id: None,
                email: None,
                username: Some("testuser".to_string()),
                ip_address: None,
            }),
            tags: vec![],
        },
        Event {
            id: "event2".to_string(), // short ID
            title: "Event with only id".to_string(),
            message: "".to_string(), // empty message - should use title
            platform: "rust".to_string(),
            date_created: Some(chrono::Utc::now().to_rfc3339()),
            user: Some(EventUser {
                id: Some("user-id-only".to_string()),
                email: None,
                username: None,
                ip_address: None,
            }),
            tags: vec![],
        },
    ];
    let result = output_events(&events, OutputFormat::Table);
    assert!(result.is_ok());
}

#[test]
fn test_output_config_status() {
    use crate::sentry::config::SentryConfig;

    // Test with all fields set
    let config = SentryConfig {
        auth_token: Some("test-token".to_string()),
        organization: Some("my-org".to_string()),
        project: Some("my-project".to_string()),
    };
    output_config_status(&config);

    // Test with no fields set
    let empty_config = SentryConfig {
        auth_token: None,
        organization: None,
        project: None,
    };
    output_config_status(&empty_config);
}
