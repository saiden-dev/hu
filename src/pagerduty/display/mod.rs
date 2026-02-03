//! PagerDuty output formatting

use anyhow::{Context, Result};
use comfy_table::{presets::UTF8_FULL_CONDENSED, Cell, Color, ContentArrangement, Table};

use super::config::PagerDutyConfig;
use super::types::{Incident, IncidentStatus, Oncall, OutputFormat};

#[cfg(test)]
mod tests;

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
