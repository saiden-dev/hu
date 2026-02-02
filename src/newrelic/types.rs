//! New Relic data types

use serde::{Deserialize, Serialize};

/// New Relic incident
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Incident {
    /// Incident ID
    pub incident_id: String,
    /// Title
    pub title: String,
    /// Priority (CRITICAL, HIGH, MEDIUM, LOW)
    #[serde(default)]
    pub priority: String,
    /// State (CREATED, ACTIVATED, CLOSED)
    #[serde(default)]
    pub state: String,
    /// Account ID
    pub account_ids: Vec<i64>,
    /// Created at timestamp
    #[serde(default)]
    pub created_at: Option<i64>,
    /// Closed at timestamp
    #[serde(default)]
    pub closed_at: Option<i64>,
}

/// New Relic issue (groups incidents)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    /// Issue ID
    pub issue_id: String,
    /// Title
    pub title: Vec<String>,
    /// Priority
    #[serde(default)]
    pub priority: String,
    /// State
    #[serde(default)]
    pub state: String,
    /// Entity names
    #[serde(default)]
    pub entity_names: Vec<String>,
    /// Created at
    pub created_at: Option<i64>,
    /// Closed at
    pub closed_at: Option<i64>,
    /// Activated at
    pub activated_at: Option<i64>,
}

/// Output format
#[derive(Debug, Clone, Copy, Default)]
pub enum OutputFormat {
    #[default]
    Table,
    Json,
}
