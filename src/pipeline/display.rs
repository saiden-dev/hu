//! Pipeline output formatting

use anyhow::{Context, Result};
use comfy_table::{presets::UTF8_FULL_CONDENSED, Cell, Color, ContentArrangement, Table};

use super::types::{OutputFormat, Pipeline, PipelineExecution, PipelineState, StageStatus};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_color_succeeded() {
        assert_eq!(status_color("Succeeded"), Color::Green);
    }

    #[test]
    fn status_color_in_progress() {
        assert_eq!(status_color("InProgress"), Color::Yellow);
    }

    #[test]
    fn status_color_failed() {
        assert_eq!(status_color("Failed"), Color::Red);
    }

    #[test]
    fn status_color_stopped() {
        assert_eq!(status_color("Stopped"), Color::DarkGrey);
    }

    #[test]
    fn status_color_cancelled() {
        assert_eq!(status_color("Cancelled"), Color::DarkGrey);
    }

    #[test]
    fn status_color_unknown() {
        assert_eq!(status_color("Unknown"), Color::White);
    }

    #[test]
    fn status_icon_succeeded() {
        assert_eq!(status_icon("Succeeded"), "✓");
    }

    #[test]
    fn status_icon_in_progress() {
        assert_eq!(status_icon("InProgress"), "◐");
    }

    #[test]
    fn status_icon_failed() {
        assert_eq!(status_icon("Failed"), "✗");
    }

    #[test]
    fn status_icon_stopped() {
        assert_eq!(status_icon("Stopped"), "○");
    }

    #[test]
    fn status_icon_unknown() {
        assert_eq!(status_icon("Other"), " ");
    }

    #[test]
    fn status_icon_cancelled() {
        assert_eq!(status_icon("Cancelled"), "○");
    }

    #[test]
    fn status_color_superseded() {
        assert_eq!(status_color("Superseded"), Color::DarkGrey);
    }

    #[test]
    fn output_pipelines_empty() {
        let result = output_pipelines(&[], OutputFormat::Table);
        assert!(result.is_ok());
    }

    #[test]
    fn output_pipelines_table() {
        let pipelines = vec![Pipeline {
            name: "test-pipeline".to_string(),
            created: Some("2026-01-01".to_string()),
            updated: None,
        }];
        let result = output_pipelines(&pipelines, OutputFormat::Table);
        assert!(result.is_ok());
    }

    #[test]
    fn output_pipelines_json() {
        let pipelines = vec![Pipeline {
            name: "test-pipeline".to_string(),
            created: None,
            updated: None,
        }];
        let result = output_pipelines(&pipelines, OutputFormat::Json);
        assert!(result.is_ok());
    }

    #[test]
    fn output_pipeline_state_empty() {
        let state = PipelineState {
            name: "test".to_string(),
            stages: vec![],
        };
        let result = output_pipeline_state(&state, OutputFormat::Table);
        assert!(result.is_ok());
    }

    #[test]
    fn output_pipeline_state_table() {
        use super::super::types::{ActionExecution, ActionState, StageExecution, StageState};

        let state = PipelineState {
            name: "test-pipeline".to_string(),
            stages: vec![StageState {
                name: "Source".to_string(),
                latest_execution: Some(StageExecution {
                    status: "Succeeded".to_string(),
                }),
                actions: vec![ActionState {
                    name: "SourceAction".to_string(),
                    latest_execution: Some(ActionExecution {
                        status: "Succeeded".to_string(),
                        last_status_change: None,
                    }),
                }],
            }],
        };
        let result = output_pipeline_state(&state, OutputFormat::Table);
        assert!(result.is_ok());
    }

    #[test]
    fn output_pipeline_state_with_all_statuses() {
        use super::super::types::{ActionExecution, ActionState, StageExecution, StageState};

        // Test with InProgress status
        let state = PipelineState {
            name: "in-progress-pipeline".to_string(),
            stages: vec![StageState {
                name: "Build".to_string(),
                latest_execution: Some(StageExecution {
                    status: "InProgress".to_string(),
                }),
                actions: vec![ActionState {
                    name: "BuildAction".to_string(),
                    latest_execution: Some(ActionExecution {
                        status: "InProgress".to_string(),
                        last_status_change: Some("2026-01-01T00:00:00Z".to_string()),
                    }),
                }],
            }],
        };
        assert!(output_pipeline_state(&state, OutputFormat::Table).is_ok());

        // Test with Failed status
        let state = PipelineState {
            name: "failed-pipeline".to_string(),
            stages: vec![StageState {
                name: "Deploy".to_string(),
                latest_execution: Some(StageExecution {
                    status: "Failed".to_string(),
                }),
                actions: vec![],
            }],
        };
        assert!(output_pipeline_state(&state, OutputFormat::Table).is_ok());

        // Test with Stopped status (triggers Unknown branch)
        let state = PipelineState {
            name: "stopped-pipeline".to_string(),
            stages: vec![StageState {
                name: "Test".to_string(),
                latest_execution: Some(StageExecution {
                    status: "Stopped".to_string(),
                }),
                actions: vec![],
            }],
        };
        assert!(output_pipeline_state(&state, OutputFormat::Table).is_ok());

        // Test stage with no execution
        let state = PipelineState {
            name: "no-execution-pipeline".to_string(),
            stages: vec![StageState {
                name: "Pending".to_string(),
                latest_execution: None,
                actions: vec![],
            }],
        };
        assert!(output_pipeline_state(&state, OutputFormat::Table).is_ok());
    }

    #[test]
    fn output_pipeline_state_json() {
        let state = PipelineState {
            name: "test".to_string(),
            stages: vec![],
        };
        let result = output_pipeline_state(&state, OutputFormat::Json);
        assert!(result.is_ok());
    }

    #[test]
    fn output_executions_empty() {
        let result = output_executions(&[], OutputFormat::Table);
        assert!(result.is_ok());
    }

    #[test]
    fn output_executions_table() {
        use super::super::types::ExecutionTrigger;

        let executions = vec![PipelineExecution {
            id: "exec-1".to_string(),
            status: "Succeeded".to_string(),
            started: Some("2026-01-01T00:00:00Z".to_string()),
            updated: None,
            trigger: Some(ExecutionTrigger {
                trigger_type: "Webhook".to_string(),
            }),
        }];
        let result = output_executions(&executions, OutputFormat::Table);
        assert!(result.is_ok());
    }

    #[test]
    fn output_executions_json() {
        let executions = vec![PipelineExecution {
            id: "exec-1".to_string(),
            status: "Failed".to_string(),
            started: None,
            updated: None,
            trigger: None,
        }];
        let result = output_executions(&executions, OutputFormat::Json);
        assert!(result.is_ok());
    }
}
