//! CodePipeline data types

use serde::{Deserialize, Serialize};

pub use crate::util::OutputFormat;

/// Pipeline summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    /// Pipeline name
    pub name: String,
    /// Creation time
    #[serde(default)]
    pub created: Option<String>,
    /// Last update time
    #[serde(default)]
    pub updated: Option<String>,
}

/// Pipeline state (current status)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineState {
    /// Pipeline name
    #[serde(rename = "pipelineName")]
    pub name: String,
    /// Stage states
    #[serde(rename = "stageStates", default)]
    pub stages: Vec<StageState>,
}

/// Stage state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageState {
    /// Stage name
    #[serde(rename = "stageName")]
    pub name: String,
    /// Latest execution
    #[serde(rename = "latestExecution", default)]
    pub latest_execution: Option<StageExecution>,
    /// Action states
    #[serde(rename = "actionStates", default)]
    pub actions: Vec<ActionState>,
}

/// Stage execution info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageExecution {
    /// Status
    pub status: String,
}

/// Action state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionState {
    /// Action name
    #[serde(rename = "actionName")]
    pub name: String,
    /// Latest execution
    #[serde(rename = "latestExecution", default)]
    pub latest_execution: Option<ActionExecution>,
}

/// Action execution info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionExecution {
    /// Status
    pub status: String,
    /// Last status change
    #[serde(rename = "lastStatusChange", default)]
    pub last_status_change: Option<String>,
}

/// Pipeline execution summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineExecution {
    /// Execution ID
    #[serde(rename = "pipelineExecutionId")]
    pub id: String,
    /// Status
    pub status: String,
    /// Start time
    #[serde(rename = "startTime", default)]
    pub started: Option<String>,
    /// Last update time
    #[serde(rename = "lastUpdateTime", default)]
    pub updated: Option<String>,
    /// Trigger info
    #[serde(default)]
    pub trigger: Option<ExecutionTrigger>,
}

/// Execution trigger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionTrigger {
    /// Trigger type
    #[serde(rename = "triggerType")]
    pub trigger_type: String,
}

/// Stage status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageStatus {
    /// In progress
    InProgress,
    /// Succeeded
    Succeeded,
    /// Failed
    Failed,
    /// Stopped
    Stopped,
    /// Unknown
    Unknown,
}

impl StageStatus {
    /// Parse from string
    pub fn from_str(s: &str) -> Self {
        match s {
            "InProgress" => Self::InProgress,
            "Succeeded" => Self::Succeeded,
            "Failed" => Self::Failed,
            "Stopped" => Self::Stopped,
            _ => Self::Unknown,
        }
    }
}

/// AWS CLI configuration
#[derive(Debug, Clone, Default)]
pub struct AwsConfig {
    /// AWS region
    pub region: Option<String>,
}

/// List pipelines response
#[derive(Debug, Deserialize)]
pub struct ListPipelinesResponse {
    /// Pipelines
    pub pipelines: Vec<PipelineSummary>,
}

/// Pipeline summary from list
#[derive(Debug, Deserialize)]
pub struct PipelineSummary {
    /// Name
    pub name: String,
    /// Created
    pub created: Option<String>,
    /// Updated
    pub updated: Option<String>,
}

impl PipelineSummary {
    /// Convert to Pipeline
    pub fn to_pipeline(&self) -> Pipeline {
        Pipeline {
            name: self.name.clone(),
            created: self.created.clone(),
            updated: self.updated.clone(),
        }
    }
}

/// List executions response
#[derive(Debug, Deserialize)]
pub struct ListExecutionsResponse {
    /// Executions
    #[serde(rename = "pipelineExecutionSummaries")]
    pub executions: Vec<PipelineExecution>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_debug() {
        let p = Pipeline {
            name: "test".to_string(),
            created: None,
            updated: None,
        };
        let debug = format!("{:?}", p);
        assert!(debug.contains("test"));
    }

    #[test]
    fn pipeline_clone() {
        let p = Pipeline {
            name: "test".to_string(),
            created: Some("2026-01-01".to_string()),
            updated: None,
        };
        let cloned = p.clone();
        assert_eq!(cloned.name, p.name);
    }

    #[test]
    fn stage_status_from_str() {
        assert_eq!(StageStatus::from_str("InProgress"), StageStatus::InProgress);
        assert_eq!(StageStatus::from_str("Succeeded"), StageStatus::Succeeded);
        assert_eq!(StageStatus::from_str("Failed"), StageStatus::Failed);
        assert_eq!(StageStatus::from_str("Stopped"), StageStatus::Stopped);
        assert_eq!(StageStatus::from_str("Other"), StageStatus::Unknown);
    }

    #[test]
    fn aws_config_default() {
        let config = AwsConfig::default();
        assert!(config.region.is_none());
    }

    #[test]
    fn parse_list_pipelines_response() {
        let json = r#"{
            "pipelines": [
                {"name": "pipeline-1", "created": "2026-01-01", "updated": "2026-01-02"},
                {"name": "pipeline-2"}
            ]
        }"#;
        let resp: ListPipelinesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.pipelines.len(), 2);
        assert_eq!(resp.pipelines[0].name, "pipeline-1");
    }

    #[test]
    fn parse_pipeline_state() {
        let json = r#"{
            "pipelineName": "my-pipeline",
            "stageStates": [
                {
                    "stageName": "Source",
                    "latestExecution": {"status": "Succeeded"},
                    "actionStates": []
                }
            ]
        }"#;
        let state: PipelineState = serde_json::from_str(json).unwrap();
        assert_eq!(state.name, "my-pipeline");
        assert_eq!(state.stages.len(), 1);
        assert_eq!(state.stages[0].name, "Source");
    }

    #[test]
    fn parse_list_executions_response() {
        let json = r#"{
            "pipelineExecutionSummaries": [
                {
                    "pipelineExecutionId": "exec-1",
                    "status": "Succeeded",
                    "startTime": "2026-01-01T00:00:00Z",
                    "trigger": {"triggerType": "Webhook"}
                }
            ]
        }"#;
        let resp: ListExecutionsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.executions.len(), 1);
        assert_eq!(resp.executions[0].id, "exec-1");
        assert_eq!(resp.executions[0].status, "Succeeded");
    }

    #[test]
    fn pipeline_summary_to_pipeline() {
        let summary = PipelineSummary {
            name: "test".to_string(),
            created: Some("2026-01-01".to_string()),
            updated: None,
        };
        let pipeline = summary.to_pipeline();
        assert_eq!(pipeline.name, "test");
        assert_eq!(pipeline.created, Some("2026-01-01".to_string()));
    }
}
