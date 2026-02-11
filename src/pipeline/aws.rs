//! AWS CLI wrapper functions

use anyhow::{Context, Result};
use std::process::Command;

use super::types::{
    AwsConfig, ListExecutionsResponse, ListPipelinesResponse, Pipeline, PipelineExecution,
    PipelineState,
};

/// Build AWS CLI base command with region
#[cfg(not(tarpaulin_include))]
fn build_aws_cmd(config: &AwsConfig) -> Command {
    let mut cmd = Command::new("aws");
    cmd.arg("codepipeline");

    if let Some(region) = &config.region {
        cmd.arg("--region").arg(region);
    }

    cmd
}

/// List all pipelines
#[cfg(not(tarpaulin_include))]
pub fn list_pipelines(config: &AwsConfig) -> Result<Vec<Pipeline>> {
    let mut cmd = build_aws_cmd(config);
    cmd.arg("list-pipelines");

    let output = cmd
        .output()
        .context("Failed to execute aws cli. Is AWS CLI installed and configured?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("aws cli failed: {}", stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_list_pipelines(&stdout)
}

/// Parse list-pipelines output
pub fn parse_list_pipelines(json: &str) -> Result<Vec<Pipeline>> {
    let resp: ListPipelinesResponse =
        serde_json::from_str(json).context("Failed to parse aws cli output")?;

    Ok(resp.pipelines.iter().map(|s| s.to_pipeline()).collect())
}

/// Get pipeline state
#[cfg(not(tarpaulin_include))]
pub fn get_pipeline_state(config: &AwsConfig, name: &str) -> Result<PipelineState> {
    let mut cmd = build_aws_cmd(config);
    cmd.arg("get-pipeline-state").arg("--name").arg(name);

    let output = cmd.output().context("Failed to execute aws cli")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("aws cli failed: {}", stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_pipeline_state(&stdout)
}

/// Parse get-pipeline-state output
pub fn parse_pipeline_state(json: &str) -> Result<PipelineState> {
    serde_json::from_str(json).context("Failed to parse pipeline state")
}

/// List pipeline executions
#[cfg(not(tarpaulin_include))]
pub fn list_executions(
    config: &AwsConfig,
    name: &str,
    limit: usize,
) -> Result<Vec<PipelineExecution>> {
    let mut cmd = build_aws_cmd(config);
    cmd.arg("list-pipeline-executions")
        .arg("--pipeline-name")
        .arg(name)
        .arg("--max-results")
        .arg(limit.to_string());

    let output = cmd.output().context("Failed to execute aws cli")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("aws cli failed: {}", stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_list_executions(&stdout)
}

/// Parse list-pipeline-executions output
pub fn parse_list_executions(json: &str) -> Result<Vec<PipelineExecution>> {
    let resp: ListExecutionsResponse =
        serde_json::from_str(json).context("Failed to parse executions")?;

    Ok(resp.executions)
}

/// Build list-pipelines args (for testing)
#[cfg(test)]
pub fn build_list_args(config: &AwsConfig) -> Vec<String> {
    let mut args = vec!["codepipeline".to_string()];

    if let Some(region) = &config.region {
        args.push("--region".to_string());
        args.push(region.clone());
    }

    args.push("list-pipelines".to_string());
    args
}

/// Build get-pipeline-state args (for testing)
#[cfg(test)]
pub fn build_state_args(config: &AwsConfig, name: &str) -> Vec<String> {
    let mut args = vec!["codepipeline".to_string()];

    if let Some(region) = &config.region {
        args.push("--region".to_string());
        args.push(region.clone());
    }

    args.push("get-pipeline-state".to_string());
    args.push("--name".to_string());
    args.push(name.to_string());
    args
}

/// Build list-pipeline-executions args (for testing)
#[cfg(test)]
pub fn build_executions_args(config: &AwsConfig, name: &str, limit: usize) -> Vec<String> {
    let mut args = vec!["codepipeline".to_string()];

    if let Some(region) = &config.region {
        args.push("--region".to_string());
        args.push(region.clone());
    }

    args.push("list-pipeline-executions".to_string());
    args.push("--pipeline-name".to_string());
    args.push(name.to_string());
    args.push("--max-results".to_string());
    args.push(limit.to_string());
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_list_args_basic() {
        let config = AwsConfig::default();
        let args = build_list_args(&config);
        assert_eq!(args, vec!["codepipeline", "list-pipelines"]);
    }

    #[test]
    fn build_list_args_with_region() {
        let config = AwsConfig {
            region: Some("us-west-2".to_string()),
        };
        let args = build_list_args(&config);
        assert_eq!(
            args,
            vec!["codepipeline", "--region", "us-west-2", "list-pipelines"]
        );
    }

    #[test]
    fn build_state_args_basic() {
        let config = AwsConfig::default();
        let args = build_state_args(&config, "my-pipeline");
        assert_eq!(
            args,
            vec![
                "codepipeline",
                "get-pipeline-state",
                "--name",
                "my-pipeline"
            ]
        );
    }

    #[test]
    fn build_state_args_with_region() {
        let config = AwsConfig {
            region: Some("eu-west-1".to_string()),
        };
        let args = build_state_args(&config, "my-pipeline");
        assert_eq!(
            args,
            vec![
                "codepipeline",
                "--region",
                "eu-west-1",
                "get-pipeline-state",
                "--name",
                "my-pipeline"
            ]
        );
    }

    #[test]
    fn build_executions_args_basic() {
        let config = AwsConfig::default();
        let args = build_executions_args(&config, "my-pipeline", 10);
        assert_eq!(
            args,
            vec![
                "codepipeline",
                "list-pipeline-executions",
                "--pipeline-name",
                "my-pipeline",
                "--max-results",
                "10"
            ]
        );
    }

    #[test]
    fn parse_list_pipelines_empty() {
        let json = r#"{"pipelines": []}"#;
        let pipelines = parse_list_pipelines(json).unwrap();
        assert!(pipelines.is_empty());
    }

    #[test]
    fn parse_list_pipelines_single() {
        let json = r#"{"pipelines": [{"name": "test"}]}"#;
        let pipelines = parse_list_pipelines(json).unwrap();
        assert_eq!(pipelines.len(), 1);
        assert_eq!(pipelines[0].name, "test");
    }

    #[test]
    fn parse_list_pipelines_invalid() {
        let result = parse_list_pipelines("not json");
        assert!(result.is_err());
    }

    #[test]
    fn parse_pipeline_state_basic() {
        let json = r#"{
            "pipelineName": "test",
            "stageStates": []
        }"#;
        let state = parse_pipeline_state(json).unwrap();
        assert_eq!(state.name, "test");
    }

    #[test]
    fn parse_list_executions_empty() {
        let json = r#"{"pipelineExecutionSummaries": []}"#;
        let executions = parse_list_executions(json).unwrap();
        assert!(executions.is_empty());
    }

    #[test]
    fn parse_list_executions_single() {
        let json = r#"{
            "pipelineExecutionSummaries": [
                {"pipelineExecutionId": "exec-1", "status": "Succeeded"}
            ]
        }"#;
        let executions = parse_list_executions(json).unwrap();
        assert_eq!(executions.len(), 1);
        assert_eq!(executions[0].id, "exec-1");
    }

    #[test]
    fn build_executions_args_with_region() {
        let config = AwsConfig {
            region: Some("ap-northeast-1".to_string()),
        };
        let args = build_executions_args(&config, "prod-pipeline", 5);
        assert_eq!(
            args,
            vec![
                "codepipeline",
                "--region",
                "ap-northeast-1",
                "list-pipeline-executions",
                "--pipeline-name",
                "prod-pipeline",
                "--max-results",
                "5"
            ]
        );
    }

    #[test]
    fn parse_list_pipelines_multiple() {
        let json = r#"{
            "pipelines": [
                {"name": "pipeline-1", "created": "2026-01-01", "updated": "2026-01-02"},
                {"name": "pipeline-2"},
                {"name": "pipeline-3", "created": "2026-01-03"}
            ]
        }"#;
        let pipelines = parse_list_pipelines(json).unwrap();
        assert_eq!(pipelines.len(), 3);
        assert_eq!(pipelines[0].name, "pipeline-1");
        assert_eq!(pipelines[0].created, Some("2026-01-01".to_string()));
        assert_eq!(pipelines[0].updated, Some("2026-01-02".to_string()));
        assert_eq!(pipelines[1].name, "pipeline-2");
        assert!(pipelines[1].created.is_none());
        assert_eq!(pipelines[2].name, "pipeline-3");
    }

    #[test]
    fn parse_pipeline_state_with_stages() {
        let json = r#"{
            "pipelineName": "complex-pipeline",
            "stageStates": [
                {
                    "stageName": "Source",
                    "latestExecution": {"status": "Succeeded"},
                    "actionStates": [
                        {"actionName": "GitCheckout", "latestExecution": {"status": "Succeeded"}}
                    ]
                },
                {
                    "stageName": "Build",
                    "latestExecution": {"status": "InProgress"},
                    "actionStates": []
                }
            ]
        }"#;
        let state = parse_pipeline_state(json).unwrap();
        assert_eq!(state.name, "complex-pipeline");
        assert_eq!(state.stages.len(), 2);
        assert_eq!(state.stages[0].name, "Source");
        assert_eq!(
            state.stages[0].latest_execution.as_ref().unwrap().status,
            "Succeeded"
        );
        assert_eq!(state.stages[0].actions.len(), 1);
        assert_eq!(state.stages[0].actions[0].name, "GitCheckout");
        assert_eq!(state.stages[1].name, "Build");
    }

    #[test]
    fn parse_pipeline_state_invalid_json() {
        let result = parse_pipeline_state("invalid json");
        assert!(result.is_err());
    }

    #[test]
    fn parse_list_executions_invalid() {
        let result = parse_list_executions("not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn parse_list_executions_multiple() {
        let json = r#"{
            "pipelineExecutionSummaries": [
                {
                    "pipelineExecutionId": "exec-1",
                    "status": "Succeeded",
                    "startTime": "2026-01-01T10:00:00Z",
                    "lastUpdateTime": "2026-01-01T10:30:00Z",
                    "trigger": {"triggerType": "Webhook"}
                },
                {
                    "pipelineExecutionId": "exec-2",
                    "status": "Failed",
                    "startTime": "2026-01-01T08:00:00Z"
                },
                {
                    "pipelineExecutionId": "exec-3",
                    "status": "InProgress"
                }
            ]
        }"#;
        let executions = parse_list_executions(json).unwrap();
        assert_eq!(executions.len(), 3);
        assert_eq!(executions[0].id, "exec-1");
        assert_eq!(executions[0].status, "Succeeded");
        assert!(executions[0].trigger.is_some());
        assert_eq!(
            executions[0].trigger.as_ref().unwrap().trigger_type,
            "Webhook"
        );
        assert_eq!(executions[1].id, "exec-2");
        assert_eq!(executions[1].status, "Failed");
        assert!(executions[1].trigger.is_none());
        assert_eq!(executions[2].id, "exec-3");
        assert_eq!(executions[2].status, "InProgress");
    }
}
