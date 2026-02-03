//! New Relic output formatting

use anyhow::{Context, Result};
use comfy_table::{presets::UTF8_FULL_CONDENSED, Cell, Color, ContentArrangement, Table};

use super::types::{Incident, Issue, OutputFormat};

#[cfg(test)]
mod tests;

/// Format timestamp from epoch millis
fn format_time(ts: Option<i64>) -> String {
    let Some(ms) = ts else {
        return "-".to_string();
    };

    let secs = ms / 1000;
    let Some(dt) = chrono::DateTime::from_timestamp(secs, 0) else {
        return "-".to_string();
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

/// Color for priority
fn priority_color(priority: &str) -> Color {
    match priority.to_uppercase().as_str() {
        "CRITICAL" => Color::Red,
        "HIGH" => Color::Yellow,
        "MEDIUM" => Color::Blue,
        _ => Color::White,
    }
}

/// Color for state
fn state_color(state: &str) -> Color {
    match state.to_uppercase().as_str() {
        "CLOSED" => Color::Green,
        "ACTIVATED" | "ACTIVE" => Color::Red,
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
            table.set_header(vec![
                "ID", "Priority", "State", "Title", "Entities", "Created",
            ]);

            for issue in issues {
                let title = issue.title.join(", ");
                let entities = issue.entity_names.join(", ");

                table.add_row(vec![
                    Cell::new(&issue.issue_id[..issue.issue_id.len().min(12)]).fg(Color::Cyan),
                    Cell::new(&issue.priority).fg(priority_color(&issue.priority)),
                    Cell::new(&issue.state).fg(state_color(&issue.state)),
                    Cell::new(truncate(&title, 40)),
                    Cell::new(truncate(&entities, 20)),
                    Cell::new(format_time(issue.created_at)),
                ]);
            }

            println!("{table}");
            println!("\n{} issues", issues.len());
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(issues).context("Failed to serialize")?;
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
            table.set_header(vec!["ID", "Priority", "State", "Title", "Created"]);

            for incident in incidents {
                table.add_row(vec![
                    Cell::new(&incident.incident_id[..incident.incident_id.len().min(12)])
                        .fg(Color::Cyan),
                    Cell::new(&incident.priority).fg(priority_color(&incident.priority)),
                    Cell::new(&incident.state).fg(state_color(&incident.state)),
                    Cell::new(truncate(&incident.title, 50)),
                    Cell::new(format_time(incident.created_at)),
                ]);
            }

            println!("{table}");
            println!("\n{} incidents", incidents.len());
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(incidents).context("Failed to serialize")?;
            println!("{json}");
        }
    }
    Ok(())
}

/// Output NRQL results
pub fn output_nrql(results: &[serde_json::Value], format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Table => {
            if results.is_empty() {
                println!("No results.");
                return Ok(());
            }

            // Try to create table from results
            if let Some(first) = results.first() {
                if let Some(obj) = first.as_object() {
                    let mut table = Table::new();
                    table.load_preset(UTF8_FULL_CONDENSED);
                    table.set_content_arrangement(ContentArrangement::Dynamic);

                    // Headers from first object
                    let headers: Vec<_> = obj.keys().collect();
                    table.set_header(headers.iter().map(|h| h.as_str()).collect::<Vec<_>>());

                    // Rows
                    for result in results {
                        if let Some(obj) = result.as_object() {
                            let row: Vec<_> = headers
                                .iter()
                                .map(|h| {
                                    obj.get(*h)
                                        .map(format_json_value)
                                        .unwrap_or_else(|| "-".to_string())
                                })
                                .collect();
                            table.add_row(row);
                        }
                    }

                    println!("{table}");
                    println!("\n{} results", results.len());
                    return Ok(());
                }
            }

            // Fallback to JSON
            let json = serde_json::to_string_pretty(results)?;
            println!("{json}");
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(results).context("Failed to serialize")?;
            println!("{json}");
        }
    }
    Ok(())
}

/// Format JSON value for table display
fn format_json_value(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => "-".to_string(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        _ => v.to_string(),
    }
}

/// Output config status
pub fn output_config_status(config: &super::config::NewRelicConfig) {
    println!("New Relic Configuration");
    println!("{}", "-".repeat(40));
    println!(
        "API key:    {}",
        if config.api_key.is_some() {
            "Yes"
        } else {
            "No"
        }
    );
    println!(
        "Account ID: {}",
        config
            .account_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "Not set".to_string())
    );
}
