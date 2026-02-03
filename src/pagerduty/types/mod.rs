//! PagerDuty data types

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

/// PagerDuty user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// User ID
    pub id: String,
    /// User name (full response)
    #[serde(default)]
    pub name: Option<String>,
    /// Summary (reference response)
    #[serde(default)]
    pub summary: Option<String>,
    /// Email address
    #[serde(default)]
    pub email: String,
    /// URL to user in PagerDuty
    #[serde(default)]
    pub html_url: String,
}

impl User {
    /// Get display name (prefers name over summary)
    pub fn display_name(&self) -> &str {
        self.name
            .as_deref()
            .or(self.summary.as_deref())
            .unwrap_or(&self.id)
    }
}

/// Escalation policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationPolicy {
    /// Policy ID
    pub id: String,
    /// Policy name (API returns "summary" for references)
    #[serde(alias = "summary")]
    pub name: String,
    /// URL to policy in PagerDuty
    #[serde(default)]
    pub html_url: String,
}

/// On-call schedule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    /// Schedule ID
    pub id: String,
    /// Schedule name (API returns "summary" for references)
    #[serde(alias = "summary")]
    pub name: String,
    /// URL to schedule in PagerDuty
    #[serde(default)]
    pub html_url: String,
}

/// On-call entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Oncall {
    /// User on call
    pub user: User,
    /// Schedule (if any)
    pub schedule: Option<Schedule>,
    /// Escalation policy
    pub escalation_policy: EscalationPolicy,
    /// Escalation level (1 = primary, 2 = secondary, etc.)
    pub escalation_level: u32,
    /// Start time of on-call shift
    pub start: Option<String>,
    /// End time of on-call shift
    pub end: Option<String>,
}

/// Service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    /// Service ID
    pub id: String,
    /// Service name (API returns "summary" for references)
    #[serde(alias = "summary")]
    pub name: String,
    /// Service status
    #[serde(default)]
    pub status: String,
    /// URL to service in PagerDuty
    #[serde(default)]
    pub html_url: String,
}

/// Incident urgency
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Urgency {
    /// High urgency
    High,
    /// Low urgency
    Low,
}

/// Incident status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IncidentStatus {
    /// Triggered - not yet acknowledged
    Triggered,
    /// Acknowledged - someone is working on it
    Acknowledged,
    /// Resolved - incident is closed
    Resolved,
}

impl IncidentStatus {
    /// Convert to API query string value
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Triggered => "triggered",
            Self::Acknowledged => "acknowledged",
            Self::Resolved => "resolved",
        }
    }
}

/// Assignment (user assigned to incident)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assignment {
    /// Assigned user
    pub assignee: User,
}

/// Incident
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Incident {
    /// Incident ID
    pub id: String,
    /// Incident number
    pub incident_number: u64,
    /// Title/summary
    pub title: String,
    /// Current status
    pub status: IncidentStatus,
    /// Urgency level
    pub urgency: Urgency,
    /// Creation timestamp
    pub created_at: String,
    /// URL to incident in PagerDuty
    #[serde(default)]
    pub html_url: String,
    /// Service this incident belongs to
    pub service: Service,
    /// Users assigned to this incident
    #[serde(default)]
    pub assignments: Vec<Assignment>,
}

/// Output format
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OutputFormat {
    /// Table format
    #[default]
    Table,
    /// JSON format
    Json,
}

/// API response wrapper for oncalls
#[derive(Debug, Deserialize)]
pub struct OncallsResponse {
    /// List of oncalls
    pub oncalls: Vec<Oncall>,
}

/// API response wrapper for incidents
#[derive(Debug, Deserialize)]
pub struct IncidentsResponse {
    /// List of incidents
    pub incidents: Vec<Incident>,
}

/// API response wrapper for single incident
#[derive(Debug, Deserialize)]
pub struct IncidentResponse {
    /// The incident
    pub incident: Incident,
}

/// API response wrapper for services
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct ServicesResponse {
    /// List of services
    pub services: Vec<Service>,
}

/// Current user response
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct CurrentUserResponse {
    /// The user
    pub user: User,
}
