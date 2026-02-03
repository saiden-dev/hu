//! Pipeline output formatting

use anyhow::{Context, Result};
use comfy_table::{presets::UTF8_FULL_CONDENSED, Cell, Color, ContentArrangement, Table};

use super::types::{OutputFormat, Pipeline, PipelineExecution, PipelineState, StageStatus};

#[cfg(test)]
mod tests;

/// Get color for stage/execution status
fn status_color(status: &str) -> Color {
    match status {
        "Succeeded" => Color::Green,
        "InProgress" => Color::Yellow,
        "Failed" => Color::Red,
        "Stopped" | "Cancelled" | "Superseded" => Color::DarkGrey,
        _ => Color::White,
    }
}

/// Get icon for status
fn status_icon(status: &str) -> &'static str {
    match status {
        "Succeeded" => "✓",
        "InProgress" => "◐",
        "Failed" => "✗",
        "Stopped" | "Cancelled" => "○",
        _ => " ",
    }
}

/// Output pipelines list
pub fn output_pipelines(pipelines: &[Pipeline], format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Table => {
            if pipelines.is_empty() {
                println!("No pipelines found.");
                return Ok(());
            }

            let mut table = Table::new();
            table.load_preset(UTF8_FULL_CONDENSED);
            table.set_content_arrangement(ContentArrangement::Dynamic);
            table.set_header(vec!["NAME", "CREATED", "UPDATED"]);

            for pipeline in pipelines {
                table.add_row(vec![
                    Cell::new(&pipeline.name).fg(Color::Cyan),
                    Cell::new(pipeline.created.as_deref().unwrap_or("-")),
                    Cell::new(pipeline.updated.as_deref().unwrap_or("-")),
                ]);
            }

            println!("{table}");
            println!("\n{} pipelines", pipelines.len());
        }
        OutputFormat::Json => {
            let json =
                serde_json::to_string_pretty(pipelines).context("Failed to serialize pipelines")?;
            println!("{json}");
        }
    }
    Ok(())
}

/// Output pipeline state (stages with status)
pub fn output_pipeline_state(state: &PipelineState, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Table => {
            println!("Pipeline: {}", state.name);
            println!();

            if state.stages.is_empty() {
                println!("No stages found.");
                return Ok(());
            }

            let mut table = Table::new();
            table.load_preset(UTF8_FULL_CONDENSED);
            table.set_content_arrangement(ContentArrangement::Dynamic);
            table.set_header(vec!["STAGE", "STATUS", "ACTIONS"]);

            for stage in &state.stages {
                let status = stage
                    .latest_execution
                    .as_ref()
                    .map(|e| e.status.as_str())
                    .unwrap_or("-");

                let status_enum = StageStatus::from_str(status);
                let icon = status_icon(status);
                let display_status = format!("{} {}", icon, status);

                // Show action count
                let action_count = stage.actions.len();
                let action_summary = if action_count > 0 {
                    let succeeded = stage
                        .actions
                        .iter()
                        .filter(|a| {
                            a.latest_execution
                                .as_ref()
                                .map(|e| e.status == "Succeeded")
                                .unwrap_or(false)
                        })
                        .count();
                    format!("{}/{} succeeded", succeeded, action_count)
                } else {
                    "-".to_string()
                };

                table.add_row(vec![
                    Cell::new(&stage.name).fg(Color::Cyan),
                    Cell::new(&display_status).fg(match status_enum {
                        StageStatus::Succeeded => Color::Green,
                        StageStatus::InProgress => Color::Yellow,
                        StageStatus::Failed => Color::Red,
                        _ => Color::White,
                    }),
                    Cell::new(&action_summary),
                ]);
            }

            println!("{table}");
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(state)
                .context("Failed to serialize pipeline state")?;
            println!("{json}");
        }
    }
    Ok(())
}

/// Output pipeline execution history
pub fn output_executions(executions: &[PipelineExecution], format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Table => {
            if executions.is_empty() {
                println!("No executions found.");
                return Ok(());
            }

            let mut table = Table::new();
            table.load_preset(UTF8_FULL_CONDENSED);
            table.set_content_arrangement(ContentArrangement::Dynamic);
            table.set_header(vec!["ID", "STATUS", "STARTED", "TRIGGER"]);

            for exec in executions {
                let icon = status_icon(&exec.status);
                let display_status = format!("{} {}", icon, exec.status);

                let trigger = exec
                    .trigger
                    .as_ref()
                    .map(|t| t.trigger_type.as_str())
                    .unwrap_or("-");

                table.add_row(vec![
                    Cell::new(&exec.id).fg(Color::Cyan),
                    Cell::new(&display_status).fg(status_color(&exec.status)),
                    Cell::new(exec.started.as_deref().unwrap_or("-")),
                    Cell::new(trigger),
                ]);
            }

            println!("{table}");
            println!("\n{} executions", executions.len());
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(executions)
                .context("Failed to serialize executions")?;
            println!("{json}");
        }
    }
    Ok(())
}
