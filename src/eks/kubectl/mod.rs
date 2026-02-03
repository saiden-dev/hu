//! kubectl wrapper functions

use anyhow::{Context, Result};
use std::process::{Command, Stdio};

use super::types::{KubectlConfig, Pod, PodList};

#[cfg(test)]
mod tests;

/// Build kubectl base command with context/namespace
fn build_kubectl_cmd(config: &KubectlConfig) -> Command {
    let mut cmd = Command::new("kubectl");

    if let Some(ctx) = &config.context {
        cmd.arg("--context").arg(ctx);
    }

    if let Some(ns) = &config.namespace {
        cmd.arg("-n").arg(ns);
    }

    cmd
}

/// List pods using kubectl
pub fn list_pods(config: &KubectlConfig, all_namespaces: bool) -> Result<Vec<Pod>> {
    let mut cmd = build_kubectl_cmd(config);
    cmd.arg("get").arg("pods").arg("-o").arg("json");

    if all_namespaces {
        cmd.arg("--all-namespaces");
    }

    let output = cmd
        .output()
        .context("Failed to execute kubectl. Is kubectl installed and configured?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("kubectl failed: {}", stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_pod_list(&stdout)
}

/// Parse kubectl JSON output into Pod list
pub fn parse_pod_list(json: &str) -> Result<Vec<Pod>> {
    let pod_list: PodList = serde_json::from_str(json).context("Failed to parse kubectl output")?;

    Ok(pod_list.items.iter().map(|item| item.to_pod()).collect())
}

/// Execute into a pod (interactive)
pub fn exec_pod(
    config: &KubectlConfig,
    pod: &str,
    container: Option<&str>,
    command: &[String],
) -> Result<()> {
    let mut cmd = build_kubectl_cmd(config);
    cmd.arg("exec").arg("-it").arg(pod);

    if let Some(c) = container {
        cmd.arg("-c").arg(c);
    }

    cmd.arg("--");

    if command.is_empty() {
        cmd.arg("/bin/sh");
    } else {
        for arg in command {
            cmd.arg(arg);
        }
    }

    // Run interactively
    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let status = cmd.status().context("Failed to execute kubectl exec")?;

    if !status.success() {
        anyhow::bail!("kubectl exec exited with status: {}", status);
    }

    Ok(())
}

/// Tail logs from a pod
#[allow(clippy::too_many_arguments)]
pub fn tail_logs(
    config: &KubectlConfig,
    pod: &str,
    container: Option<&str>,
    follow: bool,
    previous: bool,
    tail_lines: Option<usize>,
) -> Result<()> {
    let mut cmd = build_kubectl_cmd(config);
    cmd.arg("logs").arg(pod);

    if let Some(c) = container {
        cmd.arg("-c").arg(c);
    }

    if follow {
        cmd.arg("-f");
    }

    if previous {
        cmd.arg("--previous");
    }

    if let Some(n) = tail_lines {
        cmd.arg("--tail").arg(n.to_string());
    }

    // Stream output
    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let status = cmd.status().context("Failed to execute kubectl logs")?;

    if !status.success() {
        anyhow::bail!("kubectl logs exited with status: {}", status);
    }

    Ok(())
}

/// Get list of containers in a pod
#[allow(dead_code)]
pub fn get_containers(config: &KubectlConfig, pod: &str) -> Result<Vec<String>> {
    let mut cmd = build_kubectl_cmd(config);
    cmd.arg("get")
        .arg("pod")
        .arg(pod)
        .arg("-o")
        .arg("jsonpath={.spec.containers[*].name}");

    let output = cmd.output().context("Failed to execute kubectl")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("kubectl failed: {}", stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.split_whitespace().map(|s| s.to_string()).collect())
}

/// Build kubectl command args (for testing)
#[cfg(test)]
pub fn build_list_args(config: &KubectlConfig, all_namespaces: bool) -> Vec<String> {
    let mut args = Vec::new();

    if let Some(ctx) = &config.context {
        args.push("--context".to_string());
        args.push(ctx.clone());
    }

    if let Some(ns) = &config.namespace {
        args.push("-n".to_string());
        args.push(ns.clone());
    }

    args.push("get".to_string());
    args.push("pods".to_string());
    args.push("-o".to_string());
    args.push("json".to_string());

    if all_namespaces {
        args.push("--all-namespaces".to_string());
    }

    args
}

/// Build kubectl exec args (for testing)
#[cfg(test)]
pub fn build_exec_args(
    config: &KubectlConfig,
    pod: &str,
    container: Option<&str>,
    command: &[String],
) -> Vec<String> {
    let mut args = Vec::new();

    if let Some(ctx) = &config.context {
        args.push("--context".to_string());
        args.push(ctx.clone());
    }

    if let Some(ns) = &config.namespace {
        args.push("-n".to_string());
        args.push(ns.clone());
    }

    args.push("exec".to_string());
    args.push("-it".to_string());
    args.push(pod.to_string());

    if let Some(c) = container {
        args.push("-c".to_string());
        args.push(c.to_string());
    }

    args.push("--".to_string());

    if command.is_empty() {
        args.push("/bin/sh".to_string());
    } else {
        args.extend(command.iter().cloned());
    }

    args
}

/// Build kubectl logs args (for testing)
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub fn build_logs_args(
    config: &KubectlConfig,
    pod: &str,
    container: Option<&str>,
    follow: bool,
    previous: bool,
    tail_lines: Option<usize>,
) -> Vec<String> {
    let mut args = Vec::new();

    if let Some(ctx) = &config.context {
        args.push("--context".to_string());
        args.push(ctx.clone());
    }

    if let Some(ns) = &config.namespace {
        args.push("-n".to_string());
        args.push(ns.clone());
    }

    args.push("logs".to_string());
    args.push(pod.to_string());

    if let Some(c) = container {
        args.push("-c".to_string());
        args.push(c.to_string());
    }

    if follow {
        args.push("-f".to_string());
    }

    if previous {
        args.push("--previous".to_string());
    }

    if let Some(n) = tail_lines {
        args.push("--tail".to_string());
        args.push(n.to_string());
    }

    args
}
