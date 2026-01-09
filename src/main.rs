use anyhow::{bail, Context, Result};
use aws_sdk_eks::types::Cluster;
use clap::{Parser, ValueEnum};
use colored::Colorize;
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, Color, Table};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq)]
enum Environment {
    Prod,
    Dev,
    Stg,
}

impl Environment {
    fn cluster(&self) -> &'static str {
        match self {
            Environment::Prod => "prod-eks",
            Environment::Dev => "eks-dev",
            Environment::Stg => "eks-stg",
        }
    }

    fn emoji(&self) -> &'static str {
        match self {
            Environment::Prod => "🔴",
            Environment::Dev => "🟢",
            Environment::Stg => "🟡",
        }
    }

    fn long_name(&self) -> &'static str {
        match self {
            Environment::Prod => "production",
            Environment::Dev => "development",
            Environment::Stg => "staging",
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Environment::Prod => "prod",
            Environment::Dev => "dev",
            Environment::Stg => "stg",
        }
    }
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// hu - Dev workflow CLI for EKS pods, Jira, GitHub, and AWS
#[derive(Parser, Debug)]
#[command(name = "hu")]
#[command(author, version, about, long_about = None)]
#[command(after_help = "\x1b[2mExamples:\x1b[0m
    hu                                     \x1b[2m# List web pods\x1b[0m
    hu --pod 1                             \x1b[2m# Connect to pod #1\x1b[0m
    hu -e prod -t api                      \x1b[2m# List api pods on prod\x1b[0m
    hu --log                               \x1b[2m# Tail default log\x1b[0m
    hu -l /app/log/sidekiq.log             \x1b[2m# Tail custom log\x1b[0m")]
struct Args {
    /// Environment (auto-detects if omitted)
    #[arg(short, long, value_enum)]
    env: Option<Environment>,

    /// Pod name pattern to filter
    #[arg(short = 't', long = "type", default_value = "web")]
    pod_type: String,

    /// Pod number to connect to
    #[arg(short, long)]
    pod: Option<usize>,

    /// Kubernetes namespace
    #[arg(short, long, default_value = "cms")]
    namespace: String,

    /// Tail log file from all pods (default: /app/log/<env>.log)
    #[arg(short, long)]
    log: Option<Option<String>>,
}

const ANSI_COLORS: [&str; 6] = ["red", "green", "yellow", "blue", "magenta", "cyan"];

fn run_cmd_no_check(cmd: &[&str]) -> Option<String> {
    Command::new(cmd[0])
        .args(&cmd[1..])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

fn detect_env() -> Option<Environment> {
    let context = run_cmd_no_check(&["kubectl", "config", "current-context"])?;
    if context.contains("prod") {
        Some(Environment::Prod)
    } else if context.contains("dev") {
        Some(Environment::Dev)
    } else if context.contains("stg") {
        Some(Environment::Stg)
    } else {
        None
    }
}

const AWS_REGION: &str = "us-east-1";

async fn get_aws_config() -> aws_config::SdkConfig {
    aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new(AWS_REGION))
        .load()
        .await
}

async fn check_aws_session(config: &aws_config::SdkConfig) -> bool {
    let client = aws_sdk_sts::Client::new(config);
    client.get_caller_identity().send().await.is_ok()
}

fn aws_sso_login() -> Result<()> {
    let status = Command::new("aws")
        .args(["sso", "login"])
        .status()
        .context("Failed to run aws sso login")?;

    if status.success() {
        Ok(())
    } else {
        bail!("AWS SSO login failed")
    }
}

async fn get_cluster_info(config: &aws_config::SdkConfig, cluster: &str) -> Result<Cluster> {
    let client = aws_sdk_eks::Client::new(config);
    let response = client
        .describe_cluster()
        .name(cluster)
        .send()
        .await
        .context("Failed to describe EKS cluster")?;

    response
        .cluster()
        .cloned()
        .context("No cluster info returned")
}

// Kubeconfig structures for serde serialization
#[derive(Debug, Serialize, Deserialize, Default)]
struct Kubeconfig {
    #[serde(rename = "apiVersion")]
    api_version: String,
    kind: String,
    clusters: Vec<KubeconfigCluster>,
    contexts: Vec<KubeconfigContext>,
    #[serde(rename = "current-context")]
    current_context: String,
    users: Vec<KubeconfigUser>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    preferences: Option<HashMap<String, serde_yaml::Value>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct KubeconfigCluster {
    name: String,
    cluster: ClusterData,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ClusterData {
    #[serde(rename = "certificate-authority-data")]
    certificate_authority_data: String,
    server: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct KubeconfigContext {
    name: String,
    context: ContextData,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ContextData {
    cluster: String,
    user: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct KubeconfigUser {
    name: String,
    user: UserData,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct UserData {
    exec: ExecConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ExecConfig {
    #[serde(rename = "apiVersion")]
    api_version: String,
    command: String,
    args: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    env: Option<Vec<HashMap<String, String>>>,
    #[serde(
        rename = "interactiveMode",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    interactive_mode: Option<String>,
    #[serde(
        rename = "provideClusterInfo",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    provide_cluster_info: Option<bool>,
}

fn get_kubeconfig_path() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME environment variable not set")?;
    Ok(PathBuf::from(home).join(".kube").join("config"))
}

fn load_kubeconfig() -> Result<Kubeconfig> {
    let path = get_kubeconfig_path()?;
    if path.exists() {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read kubeconfig at {:?}", path))?;
        serde_yaml::from_str(&content).context("Failed to parse kubeconfig YAML")
    } else {
        Ok(Kubeconfig {
            api_version: "v1".to_string(),
            kind: "Config".to_string(),
            clusters: vec![],
            contexts: vec![],
            current_context: String::new(),
            users: vec![],
            preferences: None,
        })
    }
}

fn save_kubeconfig(config: &Kubeconfig) -> Result<()> {
    let path = get_kubeconfig_path()?;

    // Ensure .kube directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {:?}", parent))?;
    }

    let content = serde_yaml::to_string(config).context("Failed to serialize kubeconfig")?;
    std::fs::write(&path, content)
        .with_context(|| format!("Failed to write kubeconfig to {:?}", path))?;
    Ok(())
}

async fn update_kubeconfig(config: &aws_config::SdkConfig, cluster_name: &str) -> Result<()> {
    let cluster = get_cluster_info(config, cluster_name).await?;

    let endpoint = cluster.endpoint().context("Cluster has no endpoint")?;
    let ca_data = cluster
        .certificate_authority()
        .and_then(|ca| ca.data())
        .context("Cluster has no CA data")?;
    let arn = cluster.arn().context("Cluster has no ARN")?;

    let mut kubeconfig = load_kubeconfig()?;

    // Update or add cluster
    let cluster_entry = KubeconfigCluster {
        name: arn.to_string(),
        cluster: ClusterData {
            certificate_authority_data: ca_data.to_string(),
            server: endpoint.to_string(),
        },
    };

    if let Some(existing) = kubeconfig.clusters.iter_mut().find(|c| c.name == arn) {
        *existing = cluster_entry;
    } else {
        kubeconfig.clusters.push(cluster_entry);
    }

    // Update or add user with exec-based auth
    let user_entry = KubeconfigUser {
        name: arn.to_string(),
        user: UserData {
            exec: ExecConfig {
                api_version: "client.authentication.k8s.io/v1beta1".to_string(),
                command: "aws".to_string(),
                args: vec![
                    "--region".to_string(),
                    AWS_REGION.to_string(),
                    "eks".to_string(),
                    "get-token".to_string(),
                    "--cluster-name".to_string(),
                    cluster_name.to_string(),
                    "--output".to_string(),
                    "json".to_string(),
                ],
                env: None,
                interactive_mode: Some("Never".to_string()),
                provide_cluster_info: None,
            },
        },
    };

    if let Some(existing) = kubeconfig.users.iter_mut().find(|u| u.name == arn) {
        *existing = user_entry;
    } else {
        kubeconfig.users.push(user_entry);
    }

    // Update or add context
    let context_entry = KubeconfigContext {
        name: arn.to_string(),
        context: ContextData {
            cluster: arn.to_string(),
            user: arn.to_string(),
        },
    };

    if let Some(existing) = kubeconfig.contexts.iter_mut().find(|c| c.name == arn) {
        *existing = context_entry;
    } else {
        kubeconfig.contexts.push(context_entry);
    }

    // Set current context
    kubeconfig.current_context = arn.to_string();

    save_kubeconfig(&kubeconfig)?;
    Ok(())
}

fn get_pods(namespace: &str, pattern: &str) -> Vec<String> {
    let output = run_cmd_no_check(&["kubectl", "get", "pod", "-n", namespace, "--no-headers"]);

    output
        .map(|s| {
            s.lines()
                .filter(|line| line.contains(pattern))
                .filter_map(|line| line.split_whitespace().next())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default()
}

fn print_header(text: &str) {
    println!();
    println!("{}", format!("━━━ {} ━━━", text).bright_blue().bold());
    println!();
}

fn print_info(text: &str) {
    println!("{} {}", "ℹ".blue(), text);
}

fn print_success(text: &str) {
    println!("{} {}", "✓".green(), text);
}

fn print_warning(text: &str) {
    println!("{} {}", "⚠".yellow(), text);
}

fn print_error(text: &str) {
    eprintln!("{} {}", "✗".red(), text);
}

fn display_pods(pods: &[String], env: Environment) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("#").fg(Color::Cyan),
            Cell::new("Pod Name").fg(Color::Magenta),
            Cell::new("Short ID").fg(Color::DarkGrey),
        ]);

    for (i, pod) in pods.iter().enumerate() {
        let short_id = &pod[pod.len().saturating_sub(5)..];
        table.add_row(vec![
            Cell::new(i + 1).fg(Color::Cyan),
            Cell::new(pod).fg(Color::White),
            Cell::new(short_id).fg(Color::DarkGrey),
        ]);
    }

    println!();
    println!(
        "{}",
        format!(
            "{} Matching Pods ({})",
            env.emoji(),
            env.as_str().to_uppercase()
        )
        .bold()
    );
    println!("{table}");
    println!();
}

fn exec_into_pod(
    pod: &str,
    namespace: &str,
    env: Environment,
    pod_type: &str,
    pod_num: usize,
) -> Result<()> {
    let prompt_label = format!("{}-{}-{}", env.as_str(), pod_type, pod_num);
    let env_emoji = env.emoji();

    print_header(&format!("Connecting to {}", pod.bright_cyan()));
    println!(
        "  {} {} {}",
        "Prompt:".dimmed(),
        env_emoji,
        prompt_label.cyan().bold()
    );
    println!();

    let ps1_cmd = format!(
        r#"export PS1="{} \[\033[1;36m\]{}\[\033[0m\] $ "; exec bash --norc --noprofile"#,
        env_emoji, prompt_label
    );

    let status = Command::new("kubectl")
        .args([
            "exec",
            "-it",
            pod,
            "-n",
            namespace,
            "--",
            "env",
            &format!("EKS_ENV={}", env.as_str()),
            &format!("EKS_TYPE={}", pod_type),
            &format!("EKS_POD_NUM={}", pod_num),
            &format!("EKS_LABEL={}", prompt_label),
            &format!("EKS_EMOJI={}", env_emoji),
            "bash",
            "-c",
            &ps1_cmd,
        ])
        .status()
        .context("Failed to exec into pod")?;

    if !status.success() {
        bail!("kubectl exec failed");
    }

    Ok(())
}

fn tail_pod_log(
    pod: String,
    namespace: String,
    log_file: String,
    color: &'static str,
    running: Arc<AtomicBool>,
) {
    let short_id = &pod[pod.len().saturating_sub(5)..];
    let short_id = short_id.to_string();

    let child = Command::new("kubectl")
        .args([
            "exec", &pod, "-n", &namespace, "--", "tail", "-f", &log_file,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();

    let mut child = match child {
        Ok(c) => c,
        Err(_) => return,
    };

    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if !running.load(Ordering::Relaxed) {
                break;
            }
            if let Ok(line) = line {
                let prefix = format!("[{}]", short_id);
                let colored_prefix = match color {
                    "red" => prefix.red(),
                    "green" => prefix.green(),
                    "yellow" => prefix.yellow(),
                    "blue" => prefix.blue(),
                    "magenta" => prefix.magenta(),
                    "cyan" => prefix.cyan(),
                    _ => prefix.white(),
                };
                println!("{} {}", colored_prefix, line);
            }
        }
    }

    let _ = child.kill();
}

fn tail_logs(pods: &[String], namespace: &str, log_file: &str) -> Result<()> {
    print_header(&format!("Tailing Logs: {}", log_file.bright_cyan()));
    println!(
        "  {} from {} pods",
        "Streaming".dimmed(),
        pods.len().to_string().green()
    );
    println!("  {} to stop", "Press Ctrl+C".yellow());
    println!();

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::Relaxed);
        println!("\n{}", "Stopping log tail...".yellow());
    })
    .context("Failed to set Ctrl+C handler")?;

    let mut handles = vec![];

    for (i, pod) in pods.iter().enumerate() {
        let pod = pod.clone();
        let namespace = namespace.to_string();
        let log_file = log_file.to_string();
        let color = ANSI_COLORS[i % ANSI_COLORS.len()];
        let running = running.clone();

        let handle = thread::spawn(move || {
            tail_pod_log(pod, namespace, log_file, color, running);
        });
        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.join();
    }

    Ok(())
}

fn show_spinner(message: &str) -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner:.blue} {msg}")
            .unwrap(),
    );
    spinner.set_message(message.to_string());
    spinner.enable_steady_tick(Duration::from_millis(80));
    spinner
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Detect environment if not specified
    let env = match args.env {
        Some(e) => e,
        None => {
            if let Some(detected) = detect_env() {
                print_info(&format!(
                    "Detected environment: {} (from current context)",
                    detected.as_str().bold()
                ));
                detected
            } else {
                print_warning("No --env specified and couldn't detect from current context");
                print_error("Please specify --env (prod, dev, or stg)");
                std::process::exit(1);
            }
        }
    };

    let cluster = env.cluster();

    // Resolve log file path
    let log_file = match &args.log {
        Some(Some(path)) => Some(path.clone()),
        Some(None) => Some(format!("/app/log/{}.log", env.long_name())),
        None => None,
    };

    // Load AWS config
    let aws_config = get_aws_config().await;

    // Check AWS session
    let spinner = show_spinner("Checking AWS SSO session...");
    if !check_aws_session(&aws_config).await {
        spinner.finish_and_clear();
        print_warning("SSO session expired. Logging in...");
        aws_sso_login()?;
    } else {
        spinner.finish_and_clear();
    }
    print_success("AWS session active");

    // Update kubeconfig
    let spinner = show_spinner(&format!("Updating kubeconfig for {}...", cluster));
    update_kubeconfig(&aws_config, cluster).await?;
    spinner.finish_and_clear();
    print_success(&format!("Connected to {}", cluster.bold()));

    // Get pods
    let spinner = show_spinner(&format!(
        "Fetching pods matching '{}' in namespace '{}'...",
        args.pod_type, args.namespace
    ));
    let pods = get_pods(&args.namespace, &args.pod_type);
    spinner.finish_and_clear();

    if pods.is_empty() {
        print_error(&format!("No pods found matching '{}'", args.pod_type));
        std::process::exit(1);
    }

    print_success(&format!("Found {} pods", pods.len()));
    display_pods(&pods, env);

    // Log mode - tail from all pods
    if let Some(log_path) = log_file {
        return tail_logs(&pods, &args.namespace, &log_path);
    }

    // No pod specified - show hint and exit
    if args.pod.is_none() {
        println!("{}", "Next steps:".yellow().bold());
        println!(
            "  {} {}  Connect to a specific pod",
            "▸".blue(),
            "--pod <number>".cyan()
        );
        println!(
            "  {} {}           Tail logs from all pods",
            "▸".blue(),
            "--log".cyan()
        );
        println!();
        return Ok(());
    }

    // Validate pod number
    let pod_num = args.pod.unwrap();
    if pod_num < 1 || pod_num > pods.len() {
        print_error(&format!(
            "Invalid pod number '{}'. Choose 1-{}",
            pod_num,
            pods.len()
        ));
        std::process::exit(1);
    }

    // Connect to pod
    let pod = &pods[pod_num - 1];
    exec_into_pod(pod, &args.namespace, env, &args.pod_type, pod_num)
}
