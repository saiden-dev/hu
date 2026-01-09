use anyhow::{bail, Context, Result};
use clap::{Parser, ValueEnum};
use colored::Colorize;
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, Color, Table};
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{BufRead, BufReader};
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
            Environment::Dev => "dev-eks",
            Environment::Stg => "stg-eks",
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

/// EKS Shell - Connect to Kubernetes pods easily
#[derive(Parser, Debug)]
#[command(name = "eks-shell")]
#[command(author, version, about, long_about = None)]
#[command(after_help = "\x1b[2mExamples:\x1b[0m
    eks-shell                              \x1b[2m# List web pods\x1b[0m
    eks-shell --pod 1                      \x1b[2m# Connect to pod #1\x1b[0m
    eks-shell -e prod -t api               \x1b[2m# List api pods on prod\x1b[0m
    eks-shell --log                        \x1b[2m# Tail default log\x1b[0m
    eks-shell -l /app/log/sidekiq.log      \x1b[2m# Tail custom log\x1b[0m")]
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

fn run_cmd(cmd: &[&str]) -> Result<String> {
    let output = Command::new(cmd[0])
        .args(&cmd[1..])
        .output()
        .with_context(|| format!("Failed to execute: {}", cmd.join(" ")))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        bail!(
            "Command failed: {}\n{}",
            cmd.join(" "),
            String::from_utf8_lossy(&output.stderr)
        )
    }
}

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

fn check_aws_session() -> bool {
    run_cmd_no_check(&["aws", "sts", "get-caller-identity"]).is_some()
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

fn update_kubeconfig(cluster: &str, region: &str) -> Result<()> {
    run_cmd(&[
        "aws",
        "eks",
        "update-kubeconfig",
        "--name",
        cluster,
        "--region",
        region,
    ])?;
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

fn main() -> Result<()> {
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

    // Check AWS session
    let spinner = show_spinner("Checking AWS SSO session...");
    if !check_aws_session() {
        spinner.finish_and_clear();
        print_warning("SSO session expired. Logging in...");
        aws_sso_login()?;
    } else {
        spinner.finish_and_clear();
    }
    print_success("AWS session active");

    // Update kubeconfig
    let spinner = show_spinner(&format!("Updating kubeconfig for {}...", cluster));
    update_kubeconfig(cluster, "us-east-1")?;
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
