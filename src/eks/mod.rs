//! EKS pod management
//!
//! List pods, exec into pods, and tail logs.

mod cli;
mod display;
mod kubectl;
mod types;

use anyhow::Result;

pub use cli::EksCommand;
use types::{KubectlConfig, OutputFormat};

/// Run an EKS command
pub async fn run(cmd: EksCommand) -> Result<()> {
    match cmd {
        EksCommand::List {
            namespace,
            all_namespaces,
            context,
            json,
        } => cmd_list(namespace, all_namespaces, context, json),
        EksCommand::Exec {
            pod,
            namespace,
            container,
            context,
            command,
        } => cmd_exec(&pod, namespace, container, context, command),
        EksCommand::Logs {
            pod,
            namespace,
            container,
            follow,
            previous,
            tail,
            context,
        } => cmd_logs(&pod, namespace, container, follow, previous, tail, context),
    }
}

/// List pods
fn cmd_list(
    namespace: Option<String>,
    all_namespaces: bool,
    context: Option<String>,
    json: bool,
) -> Result<()> {
    let config = KubectlConfig {
        context,
        namespace: namespace.clone(),
    };

    let pods = kubectl::list_pods(&config, all_namespaces)?;

    let format = if json {
        OutputFormat::Json
    } else {
        OutputFormat::Table
    };

    // Show namespace column if listing all namespaces or no specific namespace
    let show_namespace = all_namespaces || namespace.is_none();
    display::output_pods(&pods, format, show_namespace)?;

    Ok(())
}

/// Exec into a pod
fn cmd_exec(
    pod: &str,
    namespace: Option<String>,
    container: Option<String>,
    context: Option<String>,
    command: Vec<String>,
) -> Result<()> {
    let config = KubectlConfig { context, namespace };

    kubectl::exec_pod(&config, pod, container.as_deref(), &command)
}

/// Tail logs from a pod
#[allow(clippy::too_many_arguments)]
fn cmd_logs(
    pod: &str,
    namespace: Option<String>,
    container: Option<String>,
    follow: bool,
    previous: bool,
    tail: Option<usize>,
    context: Option<String>,
) -> Result<()> {
    let config = KubectlConfig { context, namespace };

    kubectl::tail_logs(&config, pod, container.as_deref(), follow, previous, tail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kubectl_config_from_options() {
        let config = KubectlConfig {
            context: Some("prod".to_string()),
            namespace: Some("default".to_string()),
        };
        assert_eq!(config.context, Some("prod".to_string()));
        assert_eq!(config.namespace, Some("default".to_string()));
    }

    #[test]
    fn kubectl_config_none_options() {
        let config = KubectlConfig {
            context: None,
            namespace: None,
        };
        assert!(config.context.is_none());
        assert!(config.namespace.is_none());
    }

    #[test]
    fn output_format_table() {
        let format = OutputFormat::Table;
        assert_eq!(format, OutputFormat::Table);
    }

    #[test]
    fn output_format_json() {
        let format = OutputFormat::Json;
        assert_eq!(format, OutputFormat::Json);
    }

    #[test]
    fn output_format_from_bool_false() {
        let json = false;
        let format = if json {
            OutputFormat::Json
        } else {
            OutputFormat::Table
        };
        assert_eq!(format, OutputFormat::Table);
    }

    #[test]
    fn output_format_from_bool_true() {
        let json = true;
        let format = if json {
            OutputFormat::Json
        } else {
            OutputFormat::Table
        };
        assert_eq!(format, OutputFormat::Json);
    }

    // Test show_namespace logic - matches cmd_list behavior
    #[test]
    fn show_namespace_all_namespaces() {
        let all_namespaces = true;
        let namespace: Option<String> = None;
        let show_namespace = all_namespaces || namespace.is_none();
        assert!(show_namespace);
    }

    #[test]
    fn show_namespace_specific_namespace() {
        let all_namespaces = false;
        let namespace = Some("kube-system".to_string());
        let show_namespace = all_namespaces || namespace.is_none();
        assert!(!show_namespace);
    }

    #[test]
    fn show_namespace_no_namespace() {
        let all_namespaces = false;
        let namespace: Option<String> = None;
        let show_namespace = all_namespaces || namespace.is_none();
        assert!(show_namespace);
    }

    #[test]
    fn show_namespace_both_set() {
        // When both all_namespaces and specific namespace set,
        // show_namespace should be true (all_namespaces takes precedence)
        let all_namespaces = true;
        let namespace = Some("default".to_string());
        let show_namespace = all_namespaces || namespace.is_none();
        assert!(show_namespace);
    }

    // Test EksCommand variants exist and can be constructed
    #[test]
    fn eks_command_list_variant() {
        let cmd = EksCommand::List {
            namespace: None,
            all_namespaces: false,
            context: None,
            json: false,
        };
        // Just verify it constructs
        match cmd {
            EksCommand::List { .. } => {}
            _ => panic!("Expected List variant"),
        }
    }

    #[test]
    fn eks_command_exec_variant() {
        let cmd = EksCommand::Exec {
            pod: "my-pod".to_string(),
            namespace: None,
            container: None,
            context: None,
            command: vec![],
        };
        match cmd {
            EksCommand::Exec { pod, .. } => {
                assert_eq!(pod, "my-pod");
            }
            _ => panic!("Expected Exec variant"),
        }
    }

    #[test]
    fn eks_command_logs_variant() {
        let cmd = EksCommand::Logs {
            pod: "log-pod".to_string(),
            namespace: Some("prod".to_string()),
            container: None,
            follow: true,
            previous: false,
            tail: Some(100),
            context: None,
        };
        match cmd {
            EksCommand::Logs {
                pod,
                namespace,
                follow,
                tail,
                ..
            } => {
                assert_eq!(pod, "log-pod");
                assert_eq!(namespace, Some("prod".to_string()));
                assert!(follow);
                assert_eq!(tail, Some(100));
            }
            _ => panic!("Expected Logs variant"),
        }
    }
}
