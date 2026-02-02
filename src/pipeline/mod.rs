//! AWS CodePipeline status (read-only)
//!
//! List pipelines, view status, and check execution history.

mod aws;
mod cli;
mod display;
mod types;

use anyhow::Result;

pub use cli::PipelineCommand;
use types::{AwsConfig, OutputFormat};

/// Run a pipeline command
pub async fn run(cmd: PipelineCommand) -> Result<()> {
    match cmd {
        PipelineCommand::List { region, json } => cmd_list(region, json),
        PipelineCommand::Status { name, region, json } => cmd_status(&name, region, json),
        PipelineCommand::History {
            name,
            region,
            limit,
            json,
        } => cmd_history(&name, region, limit, json),
    }
}

/// List pipelines
fn cmd_list(region: Option<String>, json: bool) -> Result<()> {
    let config = AwsConfig { region };
    let pipelines = aws::list_pipelines(&config)?;

    let format = if json {
        OutputFormat::Json
    } else {
        OutputFormat::Table
    };

    display::output_pipelines(&pipelines, format)?;
    Ok(())
}

/// Show pipeline status
fn cmd_status(name: &str, region: Option<String>, json: bool) -> Result<()> {
    let config = AwsConfig { region };
    let state = aws::get_pipeline_state(&config, name)?;

    let format = if json {
        OutputFormat::Json
    } else {
        OutputFormat::Table
    };

    display::output_pipeline_state(&state, format)?;
    Ok(())
}

/// Show pipeline execution history
fn cmd_history(name: &str, region: Option<String>, limit: usize, json: bool) -> Result<()> {
    let config = AwsConfig { region };
    let executions = aws::list_executions(&config, name, limit)?;

    let format = if json {
        OutputFormat::Json
    } else {
        OutputFormat::Table
    };

    display::output_executions(&executions, format)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aws_config_from_region() {
        let config = AwsConfig {
            region: Some("us-east-1".to_string()),
        };
        assert_eq!(config.region, Some("us-east-1".to_string()));
    }

    #[test]
    fn aws_config_default() {
        let config = AwsConfig { region: None };
        assert!(config.region.is_none());
    }

    #[test]
    fn output_format_from_json_flag_true() {
        let json = true;
        let format = if json {
            OutputFormat::Json
        } else {
            OutputFormat::Table
        };
        assert_eq!(format, OutputFormat::Json);
    }

    #[test]
    fn output_format_from_json_flag_false() {
        let json = false;
        let format = if json {
            OutputFormat::Json
        } else {
            OutputFormat::Table
        };
        assert_eq!(format, OutputFormat::Table);
    }

    #[test]
    fn pipeline_command_list_matches() {
        let cmd = PipelineCommand::List {
            region: Some("us-west-2".to_string()),
            json: true,
        };
        match cmd {
            PipelineCommand::List { region, json } => {
                assert_eq!(region, Some("us-west-2".to_string()));
                assert!(json);
            }
            _ => panic!("Expected List command"),
        }
    }

    #[test]
    fn pipeline_command_status_matches() {
        let cmd = PipelineCommand::Status {
            name: "my-pipeline".to_string(),
            region: None,
            json: false,
        };
        match cmd {
            PipelineCommand::Status { name, region, json } => {
                assert_eq!(name, "my-pipeline");
                assert!(region.is_none());
                assert!(!json);
            }
            _ => panic!("Expected Status command"),
        }
    }

    #[test]
    fn pipeline_command_history_matches() {
        let cmd = PipelineCommand::History {
            name: "prod-pipeline".to_string(),
            region: Some("eu-central-1".to_string()),
            limit: 25,
            json: true,
        };
        match cmd {
            PipelineCommand::History {
                name,
                region,
                limit,
                json,
            } => {
                assert_eq!(name, "prod-pipeline");
                assert_eq!(region, Some("eu-central-1".to_string()));
                assert_eq!(limit, 25);
                assert!(json);
            }
            _ => panic!("Expected History command"),
        }
    }
}
