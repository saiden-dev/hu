use super::*;

#[test]
fn build_list_args_basic() {
    let config = KubectlConfig::default();
    let args = build_list_args(&config, false);
    assert_eq!(args, vec!["get", "pods", "-o", "json"]);
}

#[test]
fn build_list_args_with_context() {
    let config = KubectlConfig {
        context: Some("prod".to_string()),
        namespace: None,
    };
    let args = build_list_args(&config, false);
    assert_eq!(args, vec!["--context", "prod", "get", "pods", "-o", "json"]);
}

#[test]
fn build_list_args_with_namespace() {
    let config = KubectlConfig {
        context: None,
        namespace: Some("kube-system".to_string()),
    };
    let args = build_list_args(&config, false);
    assert_eq!(args, vec!["-n", "kube-system", "get", "pods", "-o", "json"]);
}

#[test]
fn build_list_args_all_namespaces() {
    let config = KubectlConfig::default();
    let args = build_list_args(&config, true);
    assert_eq!(args, vec!["get", "pods", "-o", "json", "--all-namespaces"]);
}

#[test]
fn build_list_args_full() {
    let config = KubectlConfig {
        context: Some("prod".to_string()),
        namespace: Some("default".to_string()),
    };
    let args = build_list_args(&config, true);
    assert_eq!(
        args,
        vec![
            "--context",
            "prod",
            "-n",
            "default",
            "get",
            "pods",
            "-o",
            "json",
            "--all-namespaces"
        ]
    );
}

#[test]
fn build_exec_args_basic() {
    let config = KubectlConfig::default();
    let args = build_exec_args(&config, "my-pod", None, &[]);
    assert_eq!(args, vec!["exec", "-it", "my-pod", "--", "/bin/sh"]);
}

#[test]
fn build_exec_args_with_container() {
    let config = KubectlConfig::default();
    let args = build_exec_args(&config, "my-pod", Some("app"), &[]);
    assert_eq!(
        args,
        vec!["exec", "-it", "my-pod", "-c", "app", "--", "/bin/sh"]
    );
}

#[test]
fn build_exec_args_with_command() {
    let config = KubectlConfig::default();
    let cmd = vec!["bash".to_string(), "-c".to_string(), "ls -la".to_string()];
    let args = build_exec_args(&config, "my-pod", None, &cmd);
    assert_eq!(
        args,
        vec!["exec", "-it", "my-pod", "--", "bash", "-c", "ls -la"]
    );
}

#[test]
fn build_exec_args_full() {
    let config = KubectlConfig {
        context: Some("prod".to_string()),
        namespace: Some("app".to_string()),
    };
    let args = build_exec_args(&config, "my-pod", Some("main"), &[]);
    assert_eq!(
        args,
        vec![
            "--context",
            "prod",
            "-n",
            "app",
            "exec",
            "-it",
            "my-pod",
            "-c",
            "main",
            "--",
            "/bin/sh"
        ]
    );
}

#[test]
fn build_logs_args_basic() {
    let config = KubectlConfig::default();
    let args = build_logs_args(&config, "my-pod", None, false, false, None);
    assert_eq!(args, vec!["logs", "my-pod"]);
}

#[test]
fn build_logs_args_follow() {
    let config = KubectlConfig::default();
    let args = build_logs_args(&config, "my-pod", None, true, false, None);
    assert_eq!(args, vec!["logs", "my-pod", "-f"]);
}

#[test]
fn build_logs_args_previous() {
    let config = KubectlConfig::default();
    let args = build_logs_args(&config, "my-pod", None, false, true, None);
    assert_eq!(args, vec!["logs", "my-pod", "--previous"]);
}

#[test]
fn build_logs_args_tail() {
    let config = KubectlConfig::default();
    let args = build_logs_args(&config, "my-pod", None, false, false, Some(100));
    assert_eq!(args, vec!["logs", "my-pod", "--tail", "100"]);
}

#[test]
fn build_logs_args_full() {
    let config = KubectlConfig {
        context: Some("prod".to_string()),
        namespace: Some("app".to_string()),
    };
    let args = build_logs_args(&config, "my-pod", Some("main"), true, true, Some(50));
    assert_eq!(
        args,
        vec![
            "--context",
            "prod",
            "-n",
            "app",
            "logs",
            "my-pod",
            "-c",
            "main",
            "-f",
            "--previous",
            "--tail",
            "50"
        ]
    );
}

#[test]
fn parse_pod_list_empty() {
    let json = r#"{"items": []}"#;
    let pods = parse_pod_list(json).unwrap();
    assert!(pods.is_empty());
}

#[test]
fn parse_pod_list_single() {
    let json = r#"{
            "items": [{
                "metadata": {"name": "test", "namespace": "default"},
                "status": {"phase": "Running", "containerStatuses": []}
            }]
        }"#;
    let pods = parse_pod_list(json).unwrap();
    assert_eq!(pods.len(), 1);
    assert_eq!(pods[0].name, "test");
}

#[test]
fn parse_pod_list_invalid_json() {
    let result = parse_pod_list("not json");
    assert!(result.is_err());
}

#[test]
fn parse_pod_list_multiple_pods() {
    let json = r#"{
            "items": [
                {
                    "metadata": {"name": "pod1", "namespace": "default"},
                    "status": {"phase": "Running", "containerStatuses": []}
                },
                {
                    "metadata": {"name": "pod2", "namespace": "kube-system"},
                    "status": {"phase": "Pending", "containerStatuses": []}
                }
            ]
        }"#;
    let pods = parse_pod_list(json).unwrap();
    assert_eq!(pods.len(), 2);
    assert_eq!(pods[0].name, "pod1");
    assert_eq!(pods[1].name, "pod2");
    assert_eq!(pods[1].namespace, "kube-system");
}

#[test]
fn parse_pod_list_with_full_metadata() {
    let json = r#"{
            "items": [{
                "metadata": {
                    "name": "full-pod",
                    "namespace": "production",
                    "creationTimestamp": "2026-01-15T10:30:00Z"
                },
                "spec": {
                    "nodeName": "worker-node-1",
                    "containers": [{"name": "main"}]
                },
                "status": {
                    "phase": "Running",
                    "containerStatuses": [
                        {"name": "main", "ready": true, "restartCount": 5}
                    ]
                }
            }]
        }"#;
    let pods = parse_pod_list(json).unwrap();
    assert_eq!(pods.len(), 1);
    assert_eq!(pods[0].name, "full-pod");
    assert_eq!(pods[0].namespace, "production");
    assert_eq!(pods[0].node, Some("worker-node-1".to_string()));
    assert_eq!(pods[0].restarts, 5);
    assert_eq!(pods[0].ready, "1/1");
}

#[test]
fn build_logs_args_with_container_only() {
    let config = KubectlConfig::default();
    let args = build_logs_args(&config, "my-pod", Some("sidecar"), false, false, None);
    assert_eq!(args, vec!["logs", "my-pod", "-c", "sidecar"]);
}

#[test]
fn build_exec_args_with_context_only() {
    let config = KubectlConfig {
        context: Some("staging".to_string()),
        namespace: None,
    };
    let args = build_exec_args(&config, "test-pod", None, &[]);
    assert_eq!(
        args,
        vec![
            "--context",
            "staging",
            "exec",
            "-it",
            "test-pod",
            "--",
            "/bin/sh"
        ]
    );
}

#[test]
fn build_exec_args_with_namespace_only() {
    let config = KubectlConfig {
        context: None,
        namespace: Some("monitoring".to_string()),
    };
    let args = build_exec_args(&config, "test-pod", None, &[]);
    assert_eq!(
        args,
        vec![
            "-n",
            "monitoring",
            "exec",
            "-it",
            "test-pod",
            "--",
            "/bin/sh"
        ]
    );
}

#[test]
fn build_logs_args_with_context_only() {
    let config = KubectlConfig {
        context: Some("dev".to_string()),
        namespace: None,
    };
    let args = build_logs_args(&config, "app-pod", None, false, false, None);
    assert_eq!(args, vec!["--context", "dev", "logs", "app-pod"]);
}

#[test]
fn build_logs_args_with_namespace_only() {
    let config = KubectlConfig {
        context: None,
        namespace: Some("logging".to_string()),
    };
    let args = build_logs_args(&config, "app-pod", None, false, false, None);
    assert_eq!(args, vec!["-n", "logging", "logs", "app-pod"]);
}

#[test]
fn build_logs_args_follow_and_tail() {
    let config = KubectlConfig::default();
    let args = build_logs_args(&config, "my-pod", None, true, false, Some(200));
    assert_eq!(args, vec!["logs", "my-pod", "-f", "--tail", "200"]);
}

#[test]
fn build_logs_args_previous_and_tail() {
    let config = KubectlConfig::default();
    let args = build_logs_args(&config, "my-pod", None, false, true, Some(50));
    assert_eq!(args, vec!["logs", "my-pod", "--previous", "--tail", "50"]);
}

#[test]
fn build_exec_args_with_multi_word_command() {
    let config = KubectlConfig::default();
    let cmd = vec![
        "python".to_string(),
        "-c".to_string(),
        "print('hello')".to_string(),
    ];
    let args = build_exec_args(&config, "py-pod", None, &cmd);
    assert_eq!(
        args,
        vec![
            "exec",
            "-it",
            "py-pod",
            "--",
            "python",
            "-c",
            "print('hello')"
        ]
    );
}

#[test]
fn build_exec_args_full_with_command() {
    let config = KubectlConfig {
        context: Some("prod".to_string()),
        namespace: Some("api".to_string()),
    };
    let cmd = vec!["cat".to_string(), "/etc/hosts".to_string()];
    let args = build_exec_args(&config, "api-pod", Some("nginx"), &cmd);
    assert_eq!(
        args,
        vec![
            "--context",
            "prod",
            "-n",
            "api",
            "exec",
            "-it",
            "api-pod",
            "-c",
            "nginx",
            "--",
            "cat",
            "/etc/hosts"
        ]
    );
}

#[test]
fn parse_pod_list_mixed_container_states() {
    let json = r#"{
            "items": [{
                "metadata": {"name": "mixed", "namespace": "default"},
                "status": {
                    "phase": "Running",
                    "containerStatuses": [
                        {"name": "a", "ready": true, "restartCount": 0},
                        {"name": "b", "ready": false, "restartCount": 2},
                        {"name": "c", "ready": true, "restartCount": 1}
                    ]
                }
            }]
        }"#;
    let pods = parse_pod_list(json).unwrap();
    assert_eq!(pods[0].ready, "2/3");
    assert_eq!(pods[0].restarts, 3);
}

#[test]
fn parse_pod_list_failed_status() {
    let json = r#"{
            "items": [{
                "metadata": {"name": "failed", "namespace": "default"},
                "status": {"phase": "Failed", "containerStatuses": []}
            }]
        }"#;
    let pods = parse_pod_list(json).unwrap();
    assert_eq!(pods[0].status, "Failed");
}

#[test]
fn parse_pod_list_succeeded_status() {
    let json = r#"{
            "items": [{
                "metadata": {"name": "job-pod", "namespace": "batch"},
                "status": {"phase": "Succeeded", "containerStatuses": []}
            }]
        }"#;
    let pods = parse_pod_list(json).unwrap();
    assert_eq!(pods[0].status, "Succeeded");
}

#[test]
fn parse_pod_list_unknown_status() {
    let json = r#"{
            "items": [{
                "metadata": {"name": "mystery", "namespace": "default"},
                "status": {"phase": "Unknown", "containerStatuses": []}
            }]
        }"#;
    let pods = parse_pod_list(json).unwrap();
    assert_eq!(pods[0].status, "Unknown");
}
