//! Sentry data types

use serde::{Deserialize, Serialize};

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

/// Output format
#[derive(Debug, Clone, Copy, Default)]
pub enum OutputFormat {
    #[default]
    Table,
    Json,
}
