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
