//! EKS data types

use serde::{Deserialize, Serialize};

/// Kubernetes pod
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pod {
    /// Pod name
    pub name: String,
    /// Namespace
    pub namespace: String,
    /// Pod status (Running, Pending, etc.)
    pub status: String,
    /// Ready containers (e.g., "1/1")
    pub ready: String,
    /// Restart count
    pub restarts: u32,
    /// Age (e.g., "2d", "5h")
    pub age: String,
    /// Node name
    #[serde(default)]
    pub node: Option<String>,
}

/// Kubectl configuration
#[derive(Debug, Clone, Default)]
pub struct KubectlConfig {
    /// Kubeconfig context to use
    pub context: Option<String>,
    /// Namespace to use
    pub namespace: Option<String>,
}

/// Output format
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OutputFormat {
    /// Table format
    #[default]
    Table,
    /// JSON format
    Json,
}

/// Kubectl JSON output for pods
#[derive(Debug, Deserialize)]
pub struct PodList {
    /// List of items
    pub items: Vec<PodItem>,
}

/// Single pod item from kubectl JSON
#[derive(Debug, Deserialize)]
pub struct PodItem {
    /// Metadata
    pub metadata: PodMetadata,
    /// Spec
    #[serde(default)]
    pub spec: Option<PodSpec>,
    /// Status
    pub status: PodStatus,
}

/// Pod metadata
#[derive(Debug, Deserialize)]
pub struct PodMetadata {
    /// Pod name
    pub name: String,
    /// Namespace
    pub namespace: String,
    /// Creation timestamp
    #[serde(rename = "creationTimestamp")]
    pub creation_timestamp: Option<String>,
}

/// Pod spec
#[derive(Debug, Deserialize, Default)]
pub struct PodSpec {
    /// Node name
    #[serde(rename = "nodeName")]
    pub node_name: Option<String>,
    /// Containers
    #[serde(default)]
    #[allow(dead_code)]
    pub containers: Vec<Container>,
}

/// Container spec
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Container {
    /// Container name
    pub name: String,
}

/// Pod status
#[derive(Debug, Deserialize)]
pub struct PodStatus {
    /// Phase (Running, Pending, Succeeded, Failed, Unknown)
    pub phase: String,
    /// Container statuses
    #[serde(rename = "containerStatuses", default)]
    pub container_statuses: Vec<ContainerStatus>,
}

/// Container status
#[derive(Debug, Deserialize)]
pub struct ContainerStatus {
    /// Container name
    #[allow(dead_code)]
    pub name: String,
    /// Ready state
    pub ready: bool,
    /// Restart count
    #[serde(rename = "restartCount")]
    pub restart_count: u32,
}

impl PodItem {
    /// Convert to simplified Pod struct
    pub fn to_pod(&self) -> Pod {
        let ready = self.ready_string();
        let restarts = self.total_restarts();
        let age = self.age_string();
        let node = self.spec.as_ref().and_then(|s| s.node_name.clone());

        Pod {
            name: self.metadata.name.clone(),
            namespace: self.metadata.namespace.clone(),
            status: self.status.phase.clone(),
            ready,
            restarts,
            age,
            node,
        }
    }

    /// Get ready string (e.g., "1/2")
    fn ready_string(&self) -> String {
        let total = self.status.container_statuses.len();
        let ready = self
            .status
            .container_statuses
            .iter()
            .filter(|c| c.ready)
            .count();
        format!("{}/{}", ready, total)
    }

    /// Get total restart count
    fn total_restarts(&self) -> u32 {
        self.status
            .container_statuses
            .iter()
            .map(|c| c.restart_count)
            .sum()
    }

    /// Get age string from creation timestamp
    fn age_string(&self) -> String {
        let Some(ts) = &self.metadata.creation_timestamp else {
            return "-".to_string();
        };

        let Ok(created) = chrono::DateTime::parse_from_rfc3339(ts) else {
            return "-".to_string();
        };

        let now = chrono::Utc::now();
        let duration = now.signed_duration_since(created);

        if duration.num_days() > 0 {
            format!("{}d", duration.num_days())
        } else if duration.num_hours() > 0 {
            format!("{}h", duration.num_hours())
        } else if duration.num_minutes() > 0 {
            format!("{}m", duration.num_minutes())
        } else {
            format!("{}s", duration.num_seconds())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pod_debug() {
        let pod = Pod {
            name: "test-pod".to_string(),
            namespace: "default".to_string(),
            status: "Running".to_string(),
            ready: "1/1".to_string(),
            restarts: 0,
            age: "1d".to_string(),
            node: Some("node-1".to_string()),
        };
        let debug = format!("{:?}", pod);
        assert!(debug.contains("test-pod"));
    }

    #[test]
    fn pod_clone() {
        let pod = Pod {
            name: "test-pod".to_string(),
            namespace: "default".to_string(),
            status: "Running".to_string(),
            ready: "1/1".to_string(),
            restarts: 0,
            age: "1d".to_string(),
            node: None,
        };
        let cloned = pod.clone();
        assert_eq!(cloned.name, pod.name);
    }

    #[test]
    fn kubectl_config_default() {
        let config = KubectlConfig::default();
        assert!(config.context.is_none());
        assert!(config.namespace.is_none());
    }

    #[test]
    fn output_format_default() {
        let format = OutputFormat::default();
        assert_eq!(format, OutputFormat::Table);
    }

    #[test]
    fn parse_pod_list() {
        let json = r#"{
            "items": [
                {
                    "metadata": {
                        "name": "my-pod",
                        "namespace": "default",
                        "creationTimestamp": "2026-01-01T00:00:00Z"
                    },
                    "status": {
                        "phase": "Running",
                        "containerStatuses": [
                            {"name": "main", "ready": true, "restartCount": 2}
                        ]
                    }
                }
            ]
        }"#;

        let pod_list: PodList = serde_json::from_str(json).unwrap();
        assert_eq!(pod_list.items.len(), 1);

        let pod = pod_list.items[0].to_pod();
        assert_eq!(pod.name, "my-pod");
        assert_eq!(pod.namespace, "default");
        assert_eq!(pod.status, "Running");
        assert_eq!(pod.ready, "1/1");
        assert_eq!(pod.restarts, 2);
    }

    #[test]
    fn parse_pod_list_multiple_containers() {
        let json = r#"{
            "items": [
                {
                    "metadata": {
                        "name": "multi-container",
                        "namespace": "prod"
                    },
                    "status": {
                        "phase": "Running",
                        "containerStatuses": [
                            {"name": "app", "ready": true, "restartCount": 1},
                            {"name": "sidecar", "ready": false, "restartCount": 3}
                        ]
                    }
                }
            ]
        }"#;

        let pod_list: PodList = serde_json::from_str(json).unwrap();
        let pod = pod_list.items[0].to_pod();
        assert_eq!(pod.ready, "1/2");
        assert_eq!(pod.restarts, 4);
    }

    #[test]
    fn parse_pod_list_with_node() {
        let json = r#"{
            "items": [
                {
                    "metadata": {
                        "name": "my-pod",
                        "namespace": "default"
                    },
                    "spec": {
                        "nodeName": "node-abc123"
                    },
                    "status": {
                        "phase": "Running",
                        "containerStatuses": []
                    }
                }
            ]
        }"#;

        let pod_list: PodList = serde_json::from_str(json).unwrap();
        let pod = pod_list.items[0].to_pod();
        assert_eq!(pod.node, Some("node-abc123".to_string()));
    }

    #[test]
    fn parse_pod_list_no_node() {
        let json = r#"{
            "items": [
                {
                    "metadata": {
                        "name": "pending-pod",
                        "namespace": "default"
                    },
                    "status": {
                        "phase": "Pending",
                        "containerStatuses": []
                    }
                }
            ]
        }"#;

        let pod_list: PodList = serde_json::from_str(json).unwrap();
        let pod = pod_list.items[0].to_pod();
        assert!(pod.node.is_none());
    }

    #[test]
    fn age_string_no_timestamp() {
        let item = PodItem {
            metadata: PodMetadata {
                name: "test".to_string(),
                namespace: "default".to_string(),
                creation_timestamp: None,
            },
            spec: None,
            status: PodStatus {
                phase: "Running".to_string(),
                container_statuses: vec![],
            },
        };
        let pod = item.to_pod();
        assert_eq!(pod.age, "-");
    }

    #[test]
    fn age_string_invalid_timestamp() {
        let item = PodItem {
            metadata: PodMetadata {
                name: "test".to_string(),
                namespace: "default".to_string(),
                creation_timestamp: Some("not-a-date".to_string()),
            },
            spec: None,
            status: PodStatus {
                phase: "Running".to_string(),
                container_statuses: vec![],
            },
        };
        let pod = item.to_pod();
        assert_eq!(pod.age, "-");
    }

    #[test]
    fn pod_serialize() {
        let pod = Pod {
            name: "test".to_string(),
            namespace: "default".to_string(),
            status: "Running".to_string(),
            ready: "1/1".to_string(),
            restarts: 0,
            age: "1h".to_string(),
            node: None,
        };
        let json = serde_json::to_string(&pod).unwrap();
        assert!(json.contains("test"));
    }

    #[test]
    fn pod_deserialize() {
        let json = r#"{
            "name": "test-pod",
            "namespace": "default",
            "status": "Running",
            "ready": "1/1",
            "restarts": 5,
            "age": "2d",
            "node": "worker-1"
        }"#;
        let pod: Pod = serde_json::from_str(json).unwrap();
        assert_eq!(pod.name, "test-pod");
        assert_eq!(pod.restarts, 5);
        assert_eq!(pod.node, Some("worker-1".to_string()));
    }

    #[test]
    fn pod_deserialize_no_node() {
        let json = r#"{
            "name": "test-pod",
            "namespace": "default",
            "status": "Running",
            "ready": "1/1",
            "restarts": 0,
            "age": "1h"
        }"#;
        let pod: Pod = serde_json::from_str(json).unwrap();
        assert!(pod.node.is_none());
    }

    #[test]
    fn output_format_debug() {
        let format = OutputFormat::Json;
        let debug = format!("{:?}", format);
        assert!(debug.contains("Json"));
    }

    #[test]
    fn output_format_clone() {
        let format = OutputFormat::Table;
        let cloned = format;
        assert_eq!(cloned, OutputFormat::Table);
    }

    #[test]
    fn output_format_eq() {
        assert_eq!(OutputFormat::Table, OutputFormat::Table);
        assert_eq!(OutputFormat::Json, OutputFormat::Json);
        assert_ne!(OutputFormat::Table, OutputFormat::Json);
    }

    #[test]
    fn kubectl_config_debug() {
        let config = KubectlConfig {
            context: Some("test".to_string()),
            namespace: Some("default".to_string()),
        };
        let debug = format!("{:?}", config);
        assert!(debug.contains("test"));
    }

    #[test]
    fn kubectl_config_clone() {
        let config = KubectlConfig {
            context: Some("prod".to_string()),
            namespace: None,
        };
        let cloned = config.clone();
        assert_eq!(cloned.context, Some("prod".to_string()));
    }

    #[test]
    fn pod_list_debug() {
        let pod_list = PodList { items: vec![] };
        let debug = format!("{:?}", pod_list);
        assert!(debug.contains("PodList"));
    }

    #[test]
    fn pod_item_debug() {
        let item = PodItem {
            metadata: PodMetadata {
                name: "debug-test".to_string(),
                namespace: "default".to_string(),
                creation_timestamp: None,
            },
            spec: None,
            status: PodStatus {
                phase: "Running".to_string(),
                container_statuses: vec![],
            },
        };
        let debug = format!("{:?}", item);
        assert!(debug.contains("debug-test"));
    }

    #[test]
    fn pod_metadata_debug() {
        let meta = PodMetadata {
            name: "test".to_string(),
            namespace: "ns".to_string(),
            creation_timestamp: Some("2026-01-01T00:00:00Z".to_string()),
        };
        let debug = format!("{:?}", meta);
        assert!(debug.contains("test"));
    }

    #[test]
    fn pod_spec_debug() {
        let spec = PodSpec {
            node_name: Some("node-1".to_string()),
            containers: vec![Container {
                name: "main".to_string(),
            }],
        };
        let debug = format!("{:?}", spec);
        assert!(debug.contains("node-1"));
    }

    #[test]
    fn pod_spec_default() {
        let spec = PodSpec::default();
        assert!(spec.node_name.is_none());
        assert!(spec.containers.is_empty());
    }

    #[test]
    fn container_debug() {
        let container = Container {
            name: "sidecar".to_string(),
        };
        let debug = format!("{:?}", container);
        assert!(debug.contains("sidecar"));
    }

    #[test]
    fn pod_status_debug() {
        let status = PodStatus {
            phase: "Pending".to_string(),
            container_statuses: vec![],
        };
        let debug = format!("{:?}", status);
        assert!(debug.contains("Pending"));
    }

    #[test]
    fn container_status_debug() {
        let status = ContainerStatus {
            name: "app".to_string(),
            ready: true,
            restart_count: 3,
        };
        let debug = format!("{:?}", status);
        assert!(debug.contains("app"));
        assert!(debug.contains("true"));
    }

    #[test]
    fn ready_string_all_ready() {
        let item = PodItem {
            metadata: PodMetadata {
                name: "test".to_string(),
                namespace: "default".to_string(),
                creation_timestamp: None,
            },
            spec: None,
            status: PodStatus {
                phase: "Running".to_string(),
                container_statuses: vec![
                    ContainerStatus {
                        name: "a".to_string(),
                        ready: true,
                        restart_count: 0,
                    },
                    ContainerStatus {
                        name: "b".to_string(),
                        ready: true,
                        restart_count: 0,
                    },
                ],
            },
        };
        let pod = item.to_pod();
        assert_eq!(pod.ready, "2/2");
    }

    #[test]
    fn ready_string_none_ready() {
        let item = PodItem {
            metadata: PodMetadata {
                name: "test".to_string(),
                namespace: "default".to_string(),
                creation_timestamp: None,
            },
            spec: None,
            status: PodStatus {
                phase: "Pending".to_string(),
                container_statuses: vec![
                    ContainerStatus {
                        name: "a".to_string(),
                        ready: false,
                        restart_count: 0,
                    },
                    ContainerStatus {
                        name: "b".to_string(),
                        ready: false,
                        restart_count: 0,
                    },
                ],
            },
        };
        let pod = item.to_pod();
        assert_eq!(pod.ready, "0/2");
    }

    #[test]
    fn total_restarts_sum() {
        let item = PodItem {
            metadata: PodMetadata {
                name: "test".to_string(),
                namespace: "default".to_string(),
                creation_timestamp: None,
            },
            spec: None,
            status: PodStatus {
                phase: "Running".to_string(),
                container_statuses: vec![
                    ContainerStatus {
                        name: "a".to_string(),
                        ready: true,
                        restart_count: 5,
                    },
                    ContainerStatus {
                        name: "b".to_string(),
                        ready: true,
                        restart_count: 3,
                    },
                    ContainerStatus {
                        name: "c".to_string(),
                        ready: true,
                        restart_count: 2,
                    },
                ],
            },
        };
        let pod = item.to_pod();
        assert_eq!(pod.restarts, 10);
    }

    #[test]
    fn age_string_hours() {
        // Use a timestamp from a few hours ago
        let now = chrono::Utc::now();
        let hours_ago = now - chrono::Duration::hours(5);
        let ts = hours_ago.to_rfc3339();

        let item = PodItem {
            metadata: PodMetadata {
                name: "test".to_string(),
                namespace: "default".to_string(),
                creation_timestamp: Some(ts),
            },
            spec: None,
            status: PodStatus {
                phase: "Running".to_string(),
                container_statuses: vec![],
            },
        };
        let pod = item.to_pod();
        assert!(pod.age.ends_with('h'), "Expected hours, got: {}", pod.age);
    }

    #[test]
    fn age_string_minutes() {
        let now = chrono::Utc::now();
        let mins_ago = now - chrono::Duration::minutes(30);
        let ts = mins_ago.to_rfc3339();

        let item = PodItem {
            metadata: PodMetadata {
                name: "test".to_string(),
                namespace: "default".to_string(),
                creation_timestamp: Some(ts),
            },
            spec: None,
            status: PodStatus {
                phase: "Running".to_string(),
                container_statuses: vec![],
            },
        };
        let pod = item.to_pod();
        assert!(pod.age.ends_with('m'), "Expected minutes, got: {}", pod.age);
    }

    #[test]
    fn age_string_seconds() {
        let now = chrono::Utc::now();
        let secs_ago = now - chrono::Duration::seconds(45);
        let ts = secs_ago.to_rfc3339();

        let item = PodItem {
            metadata: PodMetadata {
                name: "test".to_string(),
                namespace: "default".to_string(),
                creation_timestamp: Some(ts),
            },
            spec: None,
            status: PodStatus {
                phase: "Running".to_string(),
                container_statuses: vec![],
            },
        };
        let pod = item.to_pod();
        assert!(pod.age.ends_with('s'), "Expected seconds, got: {}", pod.age);
    }

    #[test]
    fn age_string_days() {
        let now = chrono::Utc::now();
        let days_ago = now - chrono::Duration::days(7);
        let ts = days_ago.to_rfc3339();

        let item = PodItem {
            metadata: PodMetadata {
                name: "test".to_string(),
                namespace: "default".to_string(),
                creation_timestamp: Some(ts),
            },
            spec: None,
            status: PodStatus {
                phase: "Running".to_string(),
                container_statuses: vec![],
            },
        };
        let pod = item.to_pod();
        assert!(pod.age.ends_with('d'), "Expected days, got: {}", pod.age);
    }

    #[test]
    fn pod_with_spec_node() {
        let item = PodItem {
            metadata: PodMetadata {
                name: "test".to_string(),
                namespace: "default".to_string(),
                creation_timestamp: None,
            },
            spec: Some(PodSpec {
                node_name: Some("worker-abc".to_string()),
                containers: vec![],
            }),
            status: PodStatus {
                phase: "Running".to_string(),
                container_statuses: vec![],
            },
        };
        let pod = item.to_pod();
        assert_eq!(pod.node, Some("worker-abc".to_string()));
    }

    #[test]
    fn pod_with_spec_no_node() {
        let item = PodItem {
            metadata: PodMetadata {
                name: "test".to_string(),
                namespace: "default".to_string(),
                creation_timestamp: None,
            },
            spec: Some(PodSpec {
                node_name: None,
                containers: vec![],
            }),
            status: PodStatus {
                phase: "Pending".to_string(),
                container_statuses: vec![],
            },
        };
        let pod = item.to_pod();
        assert!(pod.node.is_none());
    }
}
