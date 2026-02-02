//! PagerDuty output formatting

use anyhow::{Context, Result};
use comfy_table::{presets::UTF8_FULL_CONDENSED, Cell, Color, ContentArrangement, Table};

use super::config::PagerDutyConfig;
use super::types::{Incident, IncidentStatus, Oncall, OutputFormat};

/// Color for incident status
fn status_color(status: IncidentStatus) -> Color {
    match status {
        IncidentStatus::Triggered => Color::Red,
        IncidentStatus::Acknowledged => Color::Yellow,
        IncidentStatus::Resolved => Color::Green,
    }
}

/// Status icon
fn status_icon(status: IncidentStatus) -> &'static str {
    match status {
        IncidentStatus::Triggered => "!",
        IncidentStatus::Acknowledged => "~",
        IncidentStatus::Resolved => "✓",
    }
}

/// Format relative time from ISO8601 timestamp
fn time_ago(timestamp: &str) -> String {
    let Ok(dt) = chrono::DateTime::parse_from_rfc3339(timestamp) else {
        return timestamp.to_string();
    };

    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(dt);

    if duration.num_days() > 0 {
        format!("{}d ago", duration.num_days())
    } else if duration.num_hours() > 0 {
        format!("{}h ago", duration.num_hours())
    } else if duration.num_minutes() > 0 {
        format!("{}m ago", duration.num_minutes())
    } else {
        "just now".to_string()
    }
}

/// Truncate string to max length
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Output oncalls list
pub fn output_oncalls(oncalls: &[Oncall], format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Table => {
            if oncalls.is_empty() {
                println!("No one is currently on call.");
                return Ok(());
            }

            let mut table = Table::new();
            table.load_preset(UTF8_FULL_CONDENSED);
            table.set_content_arrangement(ContentArrangement::Dynamic);
            table.set_header(vec!["User", "Email", "Policy", "Level", "Schedule"]);

            for oncall in oncalls {
                let schedule_name = oncall
                    .schedule
                    .as_ref()
                    .map(|s| s.name.as_str())
                    .unwrap_or("-");

                table.add_row(vec![
                    Cell::new(oncall.user.display_name()).fg(Color::Cyan),
                    Cell::new(&oncall.user.email),
                    Cell::new(truncate(&oncall.escalation_policy.name, 25)),
                    Cell::new(oncall.escalation_level.to_string()),
                    Cell::new(truncate(schedule_name, 20)),
                ]);
            }

            println!("{table}");
            println!("\n{} on-call", oncalls.len());
        }
        OutputFormat::Json => {
            let json =
                serde_json::to_string_pretty(oncalls).context("Failed to serialize oncalls")?;
            println!("{json}");
        }
    }
    Ok(())
}

/// Output incidents list
pub fn output_incidents(incidents: &[Incident], format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Table => {
            if incidents.is_empty() {
                println!("No incidents found.");
                return Ok(());
            }

            let mut table = Table::new();
            table.load_preset(UTF8_FULL_CONDENSED);
            table.set_content_arrangement(ContentArrangement::Dynamic);
            table.set_header(vec![
                "#", "Status", "Urgency", "Service", "Title", "Created",
            ]);

            for incident in incidents {
                let status_text = format!("{} {:?}", status_icon(incident.status), incident.status);

                table.add_row(vec![
                    Cell::new(incident.incident_number.to_string()).fg(Color::Cyan),
                    Cell::new(&status_text).fg(status_color(incident.status)),
                    Cell::new(format!("{:?}", incident.urgency)),
                    Cell::new(truncate(&incident.service.name, 20)),
                    Cell::new(truncate(&incident.title, 40)),
                    Cell::new(time_ago(&incident.created_at)),
                ]);
            }

            println!("{table}");
            println!("\n{} incidents", incidents.len());
        }
        OutputFormat::Json => {
            let json =
                serde_json::to_string_pretty(incidents).context("Failed to serialize incidents")?;
            println!("{json}");
        }
    }
    Ok(())
}

/// Output single incident detail
pub fn output_incident_detail(incident: &Incident, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Table => {
            println!("{}", "-".repeat(60));
            println!(
                "#{} - {}",
                incident.incident_number,
                truncate(&incident.title, 50)
            );
            println!("{}", "-".repeat(60));
            println!(
                "Status:   {} {:?}",
                status_icon(incident.status),
                incident.status
            );
            println!("Urgency:  {:?}", incident.urgency);
            println!("Service:  {}", incident.service.name);
            println!("Created:  {}", time_ago(&incident.created_at));

            if !incident.assignments.is_empty() {
                println!("\nAssigned to:");
                for assignment in &incident.assignments {
                    println!(
                        "  - {} ({})",
                        assignment.assignee.display_name(),
                        assignment.assignee.email
                    );
                }
            }

            if !incident.html_url.is_empty() {
                println!("\nLink: {}", incident.html_url);
            }
        }
        OutputFormat::Json => {
            let json =
                serde_json::to_string_pretty(incident).context("Failed to serialize incident")?;
            println!("{json}");
        }
    }
    Ok(())
}

/// Output config status
pub fn output_config_status(config: &PagerDutyConfig) {
    println!("PagerDuty Configuration");
    println!("{}", "-".repeat(40));
    println!(
        "API token:  {}",
        if config.api_token.is_some() {
            "Configured"
        } else {
            "Not set"
        }
    );

    if !config.escalation_policy_ids.is_empty() {
        println!(
            "Default escalation policies: {}",
            config.escalation_policy_ids.join(", ")
        );
    }

    if !config.schedule_ids.is_empty() {
        println!("Default schedules: {}", config.schedule_ids.join(", "));
    }
}

/// Output current user info
pub fn output_user(user: &super::types::User, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Table => {
            println!("{}", user.display_name());
            if !user.email.is_empty() {
                println!("{}", user.email);
            }
            if !user.html_url.is_empty() {
                println!("{}", user.html_url);
            }
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(user).context("Failed to serialize user")?;
            println!("{json}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_color_triggered_is_red() {
        assert_eq!(status_color(IncidentStatus::Triggered), Color::Red);
    }

    #[test]
    fn status_color_acknowledged_is_yellow() {
        assert_eq!(status_color(IncidentStatus::Acknowledged), Color::Yellow);
    }

    #[test]
    fn status_color_resolved_is_green() {
        assert_eq!(status_color(IncidentStatus::Resolved), Color::Green);
    }

    #[test]
    fn status_icon_triggered() {
        assert_eq!(status_icon(IncidentStatus::Triggered), "!");
    }

    #[test]
    fn status_icon_acknowledged() {
        assert_eq!(status_icon(IncidentStatus::Acknowledged), "~");
    }

    #[test]
    fn status_icon_resolved() {
        assert_eq!(status_icon(IncidentStatus::Resolved), "✓");
    }

    #[test]
    fn truncate_short_string() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_exact_length() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn truncate_long_string() {
        assert_eq!(truncate("hello world", 8), "hello...");
    }

    #[test]
    fn truncate_very_short_max() {
        // Edge case: max_len less than 3
        assert_eq!(truncate("hello", 2), "...");
    }

    #[test]
    fn time_ago_invalid_timestamp() {
        assert_eq!(time_ago("invalid"), "invalid");
    }

    #[test]
    fn time_ago_days() {
        // 5 days ago
        let dt = chrono::Utc::now() - chrono::Duration::days(5);
        let timestamp = dt.to_rfc3339();
        assert_eq!(time_ago(&timestamp), "5d ago");
    }

    #[test]
    fn time_ago_hours() {
        // 3 hours ago
        let dt = chrono::Utc::now() - chrono::Duration::hours(3);
        let timestamp = dt.to_rfc3339();
        assert_eq!(time_ago(&timestamp), "3h ago");
    }

    #[test]
    fn time_ago_minutes() {
        // 15 minutes ago
        let dt = chrono::Utc::now() - chrono::Duration::minutes(15);
        let timestamp = dt.to_rfc3339();
        assert_eq!(time_ago(&timestamp), "15m ago");
    }

    #[test]
    fn time_ago_just_now() {
        // 30 seconds ago
        let dt = chrono::Utc::now() - chrono::Duration::seconds(30);
        let timestamp = dt.to_rfc3339();
        assert_eq!(time_ago(&timestamp), "just now");
    }

    #[test]
    fn output_config_status_not_configured() {
        let config = PagerDutyConfig::default();
        // Just verify it doesn't panic
        output_config_status(&config);
    }

    #[test]
    fn output_config_status_configured() {
        let config = PagerDutyConfig {
            api_token: Some("token".to_string()),
            escalation_policy_ids: vec!["EP1".to_string()],
            schedule_ids: vec!["S1".to_string(), "S2".to_string()],
        };
        // Just verify it doesn't panic
        output_config_status(&config);
    }

    #[test]
    fn output_oncalls_empty() {
        let result = output_oncalls(&[], OutputFormat::Table);
        assert!(result.is_ok());
    }

    #[test]
    fn output_incidents_empty() {
        let result = output_incidents(&[], OutputFormat::Table);
        assert!(result.is_ok());
    }

    #[test]
    fn output_oncalls_json_empty() {
        let result = output_oncalls(&[], OutputFormat::Json);
        assert!(result.is_ok());
    }

    #[test]
    fn output_incidents_json_empty() {
        let result = output_incidents(&[], OutputFormat::Json);
        assert!(result.is_ok());
    }

    #[test]
    fn output_oncalls_with_data() {
        use super::super::types::{EscalationPolicy, Schedule, User};

        let oncalls = vec![Oncall {
            user: User {
                id: "U1".to_string(),
                name: Some("Alice".to_string()),
                summary: None,
                email: "alice@example.com".to_string(),
                html_url: String::new(),
            },
            schedule: Some(Schedule {
                id: "S1".to_string(),
                name: "Weekly".to_string(),
                html_url: String::new(),
            }),
            escalation_policy: EscalationPolicy {
                id: "EP1".to_string(),
                name: "Primary".to_string(),
                html_url: String::new(),
            },
            escalation_level: 1,
            start: None,
            end: None,
        }];

        let result = output_oncalls(&oncalls, OutputFormat::Table);
        assert!(result.is_ok());
    }

    #[test]
    fn output_incidents_with_data() {
        use super::super::types::{Service, Urgency};

        let incidents = vec![Incident {
            id: "INC1".to_string(),
            incident_number: 42,
            title: "Test incident".to_string(),
            status: IncidentStatus::Triggered,
            urgency: Urgency::High,
            created_at: chrono::Utc::now().to_rfc3339(),
            html_url: String::new(),
            service: Service {
                id: "S1".to_string(),
                name: "Production".to_string(),
                status: "active".to_string(),
                html_url: String::new(),
            },
            assignments: vec![],
        }];

        let result = output_incidents(&incidents, OutputFormat::Table);
        assert!(result.is_ok());
    }

    #[test]
    fn output_incident_detail_table() {
        use super::super::types::{Assignment, Service, Urgency, User};

        let incident = Incident {
            id: "INC1".to_string(),
            incident_number: 42,
            title: "Server down".to_string(),
            status: IncidentStatus::Acknowledged,
            urgency: Urgency::High,
            created_at: chrono::Utc::now().to_rfc3339(),
            html_url: "https://pagerduty.com/incidents/INC1".to_string(),
            service: Service {
                id: "S1".to_string(),
                name: "Production".to_string(),
                status: "active".to_string(),
                html_url: String::new(),
            },
            assignments: vec![Assignment {
                assignee: User {
                    id: "U1".to_string(),
                    name: Some("Alice".to_string()),
                    summary: None,
                    email: "alice@example.com".to_string(),
                    html_url: String::new(),
                },
            }],
        };

        let result = output_incident_detail(&incident, OutputFormat::Table);
        assert!(result.is_ok());
    }

    #[test]
    fn output_incident_detail_json() {
        use super::super::types::{Service, Urgency};

        let incident = Incident {
            id: "INC1".to_string(),
            incident_number: 42,
            title: "Server down".to_string(),
            status: IncidentStatus::Triggered,
            urgency: Urgency::Low,
            created_at: "2026-01-01T12:00:00Z".to_string(),
            html_url: String::new(),
            service: Service {
                id: "S1".to_string(),
                name: "Production".to_string(),
                status: "active".to_string(),
                html_url: String::new(),
            },
            assignments: vec![],
        };

        let result = output_incident_detail(&incident, OutputFormat::Json);
        assert!(result.is_ok());
    }

    #[test]
    fn output_user_table_format() {
        use super::super::types::User;

        let user = User {
            id: "U1".to_string(),
            name: Some("Alice Smith".to_string()),
            summary: None,
            email: "alice@example.com".to_string(),
            html_url: "https://pagerduty.com/users/U1".to_string(),
        };

        let result = output_user(&user, OutputFormat::Table);
        assert!(result.is_ok());
    }

    #[test]
    fn output_user_json_format() {
        use super::super::types::User;

        let user = User {
            id: "U1".to_string(),
            name: Some("Alice Smith".to_string()),
            summary: None,
            email: "alice@example.com".to_string(),
            html_url: "https://pagerduty.com/users/U1".to_string(),
        };

        let result = output_user(&user, OutputFormat::Json);
        assert!(result.is_ok());
    }

    #[test]
    fn output_user_empty_email() {
        use super::super::types::User;

        let user = User {
            id: "U1".to_string(),
            name: Some("Alice".to_string()),
            summary: None,
            email: String::new(),
            html_url: String::new(),
        };

        let result = output_user(&user, OutputFormat::Table);
        assert!(result.is_ok());
    }

    #[test]
    fn output_oncalls_without_schedule() {
        use super::super::types::{EscalationPolicy, User};

        let oncalls = vec![Oncall {
            user: User {
                id: "U1".to_string(),
                name: Some("Alice".to_string()),
                summary: None,
                email: "alice@example.com".to_string(),
                html_url: String::new(),
            },
            schedule: None,
            escalation_policy: EscalationPolicy {
                id: "EP1".to_string(),
                name: "Primary".to_string(),
                html_url: String::new(),
            },
            escalation_level: 1,
            start: None,
            end: None,
        }];

        let result = output_oncalls(&oncalls, OutputFormat::Table);
        assert!(result.is_ok());
    }

    #[test]
    fn output_oncalls_json_with_data() {
        use super::super::types::{EscalationPolicy, Schedule, User};

        let oncalls = vec![Oncall {
            user: User {
                id: "U1".to_string(),
                name: Some("Alice".to_string()),
                summary: None,
                email: "alice@example.com".to_string(),
                html_url: String::new(),
            },
            schedule: Some(Schedule {
                id: "S1".to_string(),
                name: "Weekly".to_string(),
                html_url: String::new(),
            }),
            escalation_policy: EscalationPolicy {
                id: "EP1".to_string(),
                name: "Primary".to_string(),
                html_url: String::new(),
            },
            escalation_level: 1,
            start: None,
            end: None,
        }];

        let result = output_oncalls(&oncalls, OutputFormat::Json);
        assert!(result.is_ok());
    }

    #[test]
    fn output_incidents_json_with_data() {
        use super::super::types::{Service, Urgency};

        let incidents = vec![Incident {
            id: "INC1".to_string(),
            incident_number: 42,
            title: "Test incident".to_string(),
            status: IncidentStatus::Acknowledged,
            urgency: Urgency::Low,
            created_at: chrono::Utc::now().to_rfc3339(),
            html_url: String::new(),
            service: Service {
                id: "S1".to_string(),
                name: "Production".to_string(),
                status: "active".to_string(),
                html_url: String::new(),
            },
            assignments: vec![],
        }];

        let result = output_incidents(&incidents, OutputFormat::Json);
        assert!(result.is_ok());
    }

    #[test]
    fn output_incident_detail_no_url() {
        use super::super::types::{Service, Urgency};

        let incident = Incident {
            id: "INC1".to_string(),
            incident_number: 42,
            title: "Server down".to_string(),
            status: IncidentStatus::Resolved,
            urgency: Urgency::High,
            created_at: chrono::Utc::now().to_rfc3339(),
            html_url: String::new(),
            service: Service {
                id: "S1".to_string(),
                name: "Production".to_string(),
                status: "active".to_string(),
                html_url: String::new(),
            },
            assignments: vec![],
        };

        let result = output_incident_detail(&incident, OutputFormat::Table);
        assert!(result.is_ok());
    }

    #[test]
    fn truncate_zero_max() {
        // Edge case: max_len = 0
        assert_eq!(truncate("hello", 0), "...");
    }

    #[test]
    fn time_ago_boundary_cases() {
        // Exactly 1 day ago
        let dt = chrono::Utc::now() - chrono::Duration::days(1);
        let timestamp = dt.to_rfc3339();
        assert_eq!(time_ago(&timestamp), "1d ago");

        // Exactly 1 hour ago
        let dt = chrono::Utc::now() - chrono::Duration::hours(1);
        let timestamp = dt.to_rfc3339();
        assert_eq!(time_ago(&timestamp), "1h ago");

        // Exactly 1 minute ago
        let dt = chrono::Utc::now() - chrono::Duration::minutes(1);
        let timestamp = dt.to_rfc3339();
        assert_eq!(time_ago(&timestamp), "1m ago");
    }
}
