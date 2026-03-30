//! Sentry data types

use serde::{Deserialize, Serialize};

pub use crate::util::OutputFormat;

/// Sentry issue
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    /// Issue ID
    pub id: String,
    /// Short ID (e.g., "PROJECT-123")
    pub short_id: String,
    /// Issue title
    pub title: String,
    /// Culprit (location in code)
    #[serde(default)]
    pub culprit: String,
    /// Issue level (error, warning, info)
    pub level: String,
    /// Issue status (unresolved, resolved, ignored)
    pub status: String,
    /// Platform (python, javascript, etc.)
    #[serde(default)]
    pub platform: String,
    /// Project info
    pub project: ProjectInfo,
    /// Number of events
    pub count: String,
    /// Number of affected users
    pub user_count: u32,
    /// First seen timestamp
    pub first_seen: String,
    /// Last seen timestamp
    pub last_seen: String,
    /// Permalink to Sentry UI
    pub permalink: String,
    /// Is subscribed
    #[serde(default)]
    pub is_subscribed: bool,
    /// Is bookmarked
    #[serde(default)]
    pub is_bookmarked: bool,
    /// Metadata
    #[serde(default)]
    pub metadata: IssueMetadata,
}

/// Project info embedded in issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    /// Project ID
    pub id: String,
    /// Project name
    pub name: String,
    /// Project slug
    pub slug: String,
}

/// Issue metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IssueMetadata {
    /// Error type
    #[serde(rename = "type", default)]
    pub error_type: String,
    /// Error value/message
    #[serde(default)]
    pub value: String,
    /// Filename
    #[serde(default)]
    pub filename: String,
    /// Function name
    #[serde(default)]
    pub function: String,
}

/// Sentry event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Event ID
    #[serde(rename = "eventID")]
    pub id: String,
    /// Event title
    #[serde(default)]
    pub title: String,
    /// Event message
    #[serde(default)]
    pub message: String,
    /// Platform
    #[serde(default)]
    pub platform: String,
    /// Timestamp
    #[serde(rename = "dateCreated")]
    pub date_created: Option<String>,
    /// User info
    pub user: Option<EventUser>,
    /// Tags
    #[serde(default)]
    pub tags: Vec<EventTag>,
}

/// User info in event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventUser {
    /// User ID
    pub id: Option<String>,
    /// Email
    pub email: Option<String>,
    /// Username
    pub username: Option<String>,
    /// IP address
    pub ip_address: Option<String>,
}

/// Event tag
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventTag {
    /// Tag key
    pub key: String,
    /// Tag value
    pub value: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_issue_debug() {
        let issue = Issue {
            id: "12345".to_string(),
            short_id: "PROJ-123".to_string(),
            title: "Test error".to_string(),
            culprit: "src/main.rs".to_string(),
            level: "error".to_string(),
            status: "unresolved".to_string(),
            platform: "rust".to_string(),
            count: "42".to_string(),
            user_count: 10,
            first_seen: "2024-01-01T00:00:00Z".to_string(),
            last_seen: "2024-01-02T00:00:00Z".to_string(),
            permalink: "https://sentry.io/issue/123".to_string(),
            is_subscribed: false,
            is_bookmarked: true,
            project: ProjectInfo {
                id: "1".to_string(),
                name: "Test Project".to_string(),
                slug: "test-project".to_string(),
            },
            metadata: IssueMetadata::default(),
        };
        let debug = format!("{:?}", issue);
        assert!(debug.contains("Issue"));
        assert!(debug.contains("PROJ-123"));
    }

    #[test]
    fn test_issue_clone() {
        let issue = Issue {
            id: "12345".to_string(),
            short_id: "PROJ-123".to_string(),
            title: "Test".to_string(),
            culprit: "".to_string(),
            level: "error".to_string(),
            status: "unresolved".to_string(),
            platform: "".to_string(),
            count: "1".to_string(),
            user_count: 1,
            first_seen: "".to_string(),
            last_seen: "".to_string(),
            permalink: "".to_string(),
            is_subscribed: false,
            is_bookmarked: false,
            project: ProjectInfo {
                id: "1".to_string(),
                name: "Test".to_string(),
                slug: "test".to_string(),
            },
            metadata: IssueMetadata::default(),
        };
        let cloned = issue.clone();
        assert_eq!(cloned.id, issue.id);
        assert_eq!(cloned.short_id, issue.short_id);
    }

    #[test]
    fn test_project_info_debug() {
        let project = ProjectInfo {
            id: "1".to_string(),
            name: "My Project".to_string(),
            slug: "my-project".to_string(),
        };
        let debug = format!("{:?}", project);
        assert!(debug.contains("ProjectInfo"));
    }

    #[test]
    fn test_issue_metadata_default() {
        let metadata = IssueMetadata::default();
        assert!(metadata.error_type.is_empty());
        assert!(metadata.value.is_empty());
        assert!(metadata.filename.is_empty());
        assert!(metadata.function.is_empty());
    }

    #[test]
    fn test_issue_metadata_debug() {
        let metadata = IssueMetadata {
            error_type: "RuntimeError".to_string(),
            value: "Error message".to_string(),
            filename: "main.rs".to_string(),
            function: "main".to_string(),
        };
        let debug = format!("{:?}", metadata);
        assert!(debug.contains("IssueMetadata"));
    }

    #[test]
    fn test_event_debug() {
        let event = Event {
            id: "event123".to_string(),
            title: "Error event".to_string(),
            message: "Something went wrong".to_string(),
            platform: "rust".to_string(),
            date_created: Some("2024-01-01T00:00:00Z".to_string()),
            user: None,
            tags: vec![],
        };
        let debug = format!("{:?}", event);
        assert!(debug.contains("Event"));
    }

    #[test]
    fn test_event_clone() {
        let event = Event {
            id: "event123".to_string(),
            title: "Test".to_string(),
            message: "".to_string(),
            platform: "".to_string(),
            date_created: None,
            user: Some(EventUser {
                id: Some("user1".to_string()),
                email: None,
                username: None,
                ip_address: None,
            }),
            tags: vec![EventTag {
                key: "env".to_string(),
                value: "prod".to_string(),
            }],
        };
        let cloned = event.clone();
        assert_eq!(cloned.id, event.id);
        assert!(cloned.user.is_some());
    }

    #[test]
    fn test_event_user_debug() {
        let user = EventUser {
            id: Some("user123".to_string()),
            email: Some("test@example.com".to_string()),
            username: Some("testuser".to_string()),
            ip_address: Some("192.168.1.1".to_string()),
        };
        let debug = format!("{:?}", user);
        assert!(debug.contains("EventUser"));
    }

    #[test]
    fn test_event_tag_debug() {
        let tag = EventTag {
            key: "environment".to_string(),
            value: "production".to_string(),
        };
        let debug = format!("{:?}", tag);
        assert!(debug.contains("EventTag"));
    }

    #[test]
    fn test_issue_serde_default_fields() {
        // Test that serde default works for optional fields
        let json = r#"{
            "id": "1",
            "shortId": "PROJ-1",
            "title": "Test",
            "level": "error",
            "status": "unresolved",
            "count": "1",
            "userCount": 1,
            "firstSeen": "2024-01-01T00:00:00Z",
            "lastSeen": "2024-01-01T00:00:00Z",
            "permalink": "http://example.com",
            "project": {"id": "1", "name": "Test", "slug": "test"}
        }"#;
        let issue: Issue = serde_json::from_str(json).unwrap();
        assert!(issue.culprit.is_empty());
        assert!(issue.platform.is_empty());
        assert!(!issue.is_subscribed);
        assert!(!issue.is_bookmarked);
    }
}
