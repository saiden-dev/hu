//! Sentry output formatting

use anyhow::{Context, Result};
use comfy_table::{presets::UTF8_FULL_CONDENSED, Cell, Color, ContentArrangement, Table};

use super::types::{Event, Issue, OutputFormat};

#[cfg(test)]
mod tests;

/// Format relative time
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

/// Truncate string
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Color for issue level
fn level_color(level: &str) -> Color {
    match level {
        "error" => Color::Red,
        "warning" => Color::Yellow,
        "info" => Color::Blue,
        _ => Color::White,
    }
}

/// Color for issue status
#[allow(dead_code)]
fn status_color(status: &str) -> Color {
    match status {
        "resolved" => Color::Green,
        "ignored" => Color::DarkGrey,
        _ => Color::White,
    }
}

/// Output issues list
pub fn output_issues(issues: &[Issue], format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Table => {
            if issues.is_empty() {
                println!("No issues found.");
                return Ok(());
            }

            let mut table = Table::new();
            table.load_preset(UTF8_FULL_CONDENSED);
            table.set_content_arrangement(ContentArrangement::Dynamic);
            table.set_header(vec!["ID", "Level", "Title", "Events", "Users", "Last Seen"]);

            for issue in issues {
                table.add_row(vec![
                    Cell::new(&issue.short_id).fg(Color::Cyan),
                    Cell::new(&issue.level).fg(level_color(&issue.level)),
                    Cell::new(truncate(&issue.title, 50)),
                    Cell::new(&issue.count),
                    Cell::new(issue.user_count.to_string()),
                    Cell::new(time_ago(&issue.last_seen)),
                ]);
            }

            println!("{table}");
            println!("\n{} issues", issues.len());
        }
        OutputFormat::Json => {
            let json =
                serde_json::to_string_pretty(issues).context("Failed to serialize issues")?;
            println!("{json}");
        }
    }
    Ok(())
}

/// Output single issue detail
pub fn output_issue_detail(issue: &Issue, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Table => {
            println!("{}", "-".repeat(60));
            println!("{} - {}", issue.short_id, issue.title);
            println!("{}", "-".repeat(60));
            println!(
                "Project:    {} ({})",
                issue.project.name, issue.project.slug
            );
            println!("Level:      {}", issue.level);
            println!("Status:     {}", issue.status);
            println!("Platform:   {}", issue.platform);
            println!("Events:     {}", issue.count);
            println!("Users:      {}", issue.user_count);
            println!("First seen: {}", time_ago(&issue.first_seen));
            println!("Last seen:  {}", time_ago(&issue.last_seen));

            if !issue.culprit.is_empty() {
                println!("\nCulprit: {}", issue.culprit);
            }

            if !issue.metadata.error_type.is_empty() || !issue.metadata.value.is_empty() {
                println!("\nError:");
                if !issue.metadata.error_type.is_empty() {
                    println!("  Type: {}", issue.metadata.error_type);
                }
                if !issue.metadata.value.is_empty() {
                    println!("  Message: {}", issue.metadata.value);
                }
                if !issue.metadata.filename.is_empty() {
                    println!("  File: {}", issue.metadata.filename);
                }
                if !issue.metadata.function.is_empty() {
                    println!("  Function: {}", issue.metadata.function);
                }
            }

            println!("\nLink: {}", issue.permalink);
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(issue).context("Failed to serialize issue")?;
            println!("{json}");
        }
    }
    Ok(())
}

/// Output events list
pub fn output_events(events: &[Event], format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Table => {
            if events.is_empty() {
                println!("No events found.");
                return Ok(());
            }

            let mut table = Table::new();
            table.load_preset(UTF8_FULL_CONDENSED);
            table.set_content_arrangement(ContentArrangement::Dynamic);
            table.set_header(vec!["Event ID", "Time", "User", "Message"]);

            for event in events {
                let user = event
                    .user
                    .as_ref()
                    .and_then(|u| u.email.as_ref().or(u.username.as_ref()).or(u.id.as_ref()))
                    .map(|s| s.as_str())
                    .unwrap_or("-");

                let message = if event.message.is_empty() {
                    &event.title
                } else {
                    &event.message
                };

                let event_id_short = if event.id.len() > 12 {
                    &event.id[..12]
                } else {
                    &event.id
                };
                let date = event.date_created.as_deref().unwrap_or("-");

                table.add_row(vec![
                    Cell::new(event_id_short).fg(Color::Cyan),
                    Cell::new(time_ago(date)),
                    Cell::new(truncate(user, 20)),
                    Cell::new(truncate(message, 40)),
                ]);
            }

            println!("{table}");
            println!("\n{} events", events.len());
        }
        OutputFormat::Json => {
            let json =
                serde_json::to_string_pretty(events).context("Failed to serialize events")?;
            println!("{json}");
        }
    }
    Ok(())
}

/// Output config status
pub fn output_config_status(config: &super::config::SentryConfig) {
    println!("Sentry Configuration");
    println!("{}", "-".repeat(40));
    println!(
        "Auth token:   {}",
        if config.auth_token.is_some() {
            "Yes"
        } else {
            "No"
        }
    );
    println!(
        "Organization: {}",
        config.organization.as_deref().unwrap_or("Not set")
    );
    println!(
        "Project:      {}",
        config.project.as_deref().unwrap_or("Not set")
    );
}
