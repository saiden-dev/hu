mod config;

use anyhow::{bail, Context, Result};
use aws_sdk_eks::types::Cluster;
use clap::{Parser, ValueEnum};
use colored::Colorize;
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, Color, Table};
use config::Settings;
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
    fn cluster<'a>(&self, settings: &'a Settings) -> &'a str {
        match self {
            Environment::Prod => &settings.environments.clusters.prod,
            Environment::Dev => &settings.environments.clusters.dev,
            Environment::Stg => &settings.environments.clusters.stg,
        }
    }

    fn emoji<'a>(&self, settings: &'a Settings) -> &'a str {
        match self {
            Environment::Prod => &settings.display.emojis.prod,
            Environment::Dev => &settings.display.emojis.dev,
            Environment::Stg => &settings.display.emojis.stg,
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
    hu -l /app/log/sidekiq.log             \x1b[2m# Tail custom log\x1b[0m
    hu --whoami                            \x1b[2m# Show AWS identity and permissions\x1b[0m
    hu --aws-profile hu                    \x1b[2m# Use specific AWS profile\x1b[0m
    hu log                                 \x1b[2m# View local log file\x1b[0m
    hu log -f                              \x1b[2m# Tail local log file\x1b[0m")]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Environment (auto-detects if omitted)
    #[arg(short, long, value_enum)]
    env: Option<Environment>,

    /// Pod name pattern to filter
    #[arg(short = 't', long = "type")]
    pod_type: Option<String>,

    /// Pod number to connect to
    #[arg(short, long)]
    pod: Option<usize>,

    /// Kubernetes namespace
    #[arg(short, long)]
    namespace: Option<String>,

    /// Tail log file from all pods (default: /app/log/<env>.log)
    #[arg(short, long)]
    log: Option<Option<String>>,

    /// Show AWS identity and permissions
    #[arg(long)]
    whoami: bool,

    /// AWS profile to use
    #[arg(long = "aws-profile")]
    aws_profile: Option<String>,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// View or tail local log files with pretty colors
    #[command(alias = "logs")]
    Log {
        /// Environment to view logs for
        #[arg(short, long, value_enum)]
        env: Option<Environment>,

        /// Path to log file (overrides env-based path)
        #[arg(short, long)]
        path: Option<String>,

        /// Follow/tail the log file
        #[arg(short, long)]
        follow: bool,

        /// Number of lines to show (default: 50)
        #[arg(short = 'n', long, default_value = "50")]
        lines: usize,

        /// Filter lines containing this pattern
        #[arg(short = 'g', long)]
        grep: Option<String>,

        /// Show timestamps in a different color
        #[arg(long, default_value = "true")]
        colorize: bool,
    },
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

async fn get_aws_config(profile: Option<&str>, region: &str) -> aws_config::SdkConfig {
    let mut builder = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new(region.to_string()));

    if let Some(profile_name) = profile {
        builder = builder.profile_name(profile_name);
    }

    builder.load().await
}

async fn check_aws_session(config: &aws_config::SdkConfig) -> bool {
    let client = aws_sdk_sts::Client::new(config);
    client.get_caller_identity().send().await.is_ok()
}

fn aws_sso_login(profile: Option<&str>) -> Result<()> {
    let mut cmd = Command::new("aws");
    cmd.args(["sso", "login"]);

    if let Some(profile_name) = profile {
        cmd.args(["--profile", profile_name]);
    }

    let status = cmd.status().context("Failed to run aws sso login")?;

    if status.success() {
        Ok(())
    } else {
        bail!("AWS SSO login failed")
    }
}

// ==================== AWS Identity & Permissions ====================

#[derive(Debug)]
enum IdentityType {
    User(String),        // username
    AssumedRole(String), // role name
    FederatedUser(String),
    Unknown,
}

#[derive(Debug)]
struct IdentityInfo {
    account: String,
    arn: String,
    identity_type: IdentityType,
}

#[derive(Debug)]
enum PolicyType {
    AwsManaged,
    CustomerManaged,
    Inline,
}

#[derive(Debug)]
struct PolicyInfo {
    name: String,
    policy_type: PolicyType,
    statements: Vec<PolicyStatement>,
}

#[derive(Debug, Default)]
struct PolicyStatement {
    effect: String,
    actions: Vec<String>,
    resources: Vec<String>,
    conditions: Option<String>,
}

impl IdentityInfo {
    fn from_arn(arn: &str, account: &str) -> Self {
        let identity_type = if arn.contains(":user/") {
            let name = arn.split(":user/").last().unwrap_or("unknown").to_string();
            IdentityType::User(name)
        } else if arn.contains(":assumed-role/") {
            // ARN format: arn:aws:sts::ACCOUNT:assumed-role/ROLE-NAME/SESSION
            let parts: Vec<&str> = arn
                .split(":assumed-role/")
                .last()
                .unwrap_or("")
                .split('/')
                .collect();
            let role_name = parts.first().unwrap_or(&"unknown").to_string();
            IdentityType::AssumedRole(role_name)
        } else if arn.contains(":federated-user/") {
            let name = arn
                .split(":federated-user/")
                .last()
                .unwrap_or("unknown")
                .to_string();
            IdentityType::FederatedUser(name)
        } else {
            IdentityType::Unknown
        };

        Self {
            account: account.to_string(),
            arn: arn.to_string(),
            identity_type,
        }
    }

    fn type_name(&self) -> &str {
        match &self.identity_type {
            IdentityType::User(_) => "IAM User",
            IdentityType::AssumedRole(_) => "Assumed Role",
            IdentityType::FederatedUser(_) => "Federated User",
            IdentityType::Unknown => "Unknown",
        }
    }

    fn name(&self) -> &str {
        match &self.identity_type {
            IdentityType::User(n) => n,
            IdentityType::AssumedRole(n) => n,
            IdentityType::FederatedUser(n) => n,
            IdentityType::Unknown => "unknown",
        }
    }
}

async fn get_identity_info(config: &aws_config::SdkConfig) -> Result<IdentityInfo> {
    let sts = aws_sdk_sts::Client::new(config);
    let identity = sts
        .get_caller_identity()
        .send()
        .await
        .context("Failed to get caller identity")?;

    let arn = identity.arn().context("No ARN in identity response")?;
    let account = identity
        .account()
        .context("No account in identity response")?;

    Ok(IdentityInfo::from_arn(arn, account))
}

fn parse_policy_document(doc: &str) -> Vec<PolicyStatement> {
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(doc);
    let Ok(json) = parsed else {
        return vec![];
    };

    let statements = match json.get("Statement") {
        Some(serde_json::Value::Array(arr)) => arr.clone(),
        Some(stmt) => vec![stmt.clone()],
        None => return vec![],
    };

    statements
        .iter()
        .map(|stmt| {
            let effect = stmt
                .get("Effect")
                .and_then(|v| v.as_str())
                .unwrap_or("Allow")
                .to_string();

            let actions = match stmt.get("Action") {
                Some(serde_json::Value::Array(arr)) => arr
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect(),
                Some(serde_json::Value::String(s)) => vec![s.clone()],
                _ => vec![],
            };

            let resources = match stmt.get("Resource") {
                Some(serde_json::Value::Array(arr)) => arr
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect(),
                Some(serde_json::Value::String(s)) => vec![s.clone()],
                _ => vec![],
            };

            let conditions = stmt
                .get("Condition")
                .map(|c| serde_json::to_string_pretty(c).unwrap_or_else(|_| "{}".to_string()));

            PolicyStatement {
                effect,
                actions,
                resources,
                conditions,
            }
        })
        .collect()
}

async fn get_managed_policy_document(
    iam: &aws_sdk_iam::Client,
    policy_arn: &str,
) -> Result<Vec<PolicyStatement>> {
    let policy = iam
        .get_policy()
        .policy_arn(policy_arn)
        .send()
        .await
        .context("Failed to get policy")?;

    let version_id = policy
        .policy()
        .and_then(|p| p.default_version_id())
        .context("No default version ID")?;

    let version = iam
        .get_policy_version()
        .policy_arn(policy_arn)
        .version_id(version_id)
        .send()
        .await
        .context("Failed to get policy version")?;

    let document = version
        .policy_version()
        .and_then(|v| v.document())
        .context("No policy document")?;

    // Policy documents are URL-encoded
    let decoded = urlencoding::decode(document).unwrap_or_else(|_| document.into());

    Ok(parse_policy_document(&decoded))
}

async fn get_role_policies(iam: &aws_sdk_iam::Client, role_name: &str) -> Result<Vec<PolicyInfo>> {
    let mut policies = Vec::new();

    // Get attached managed policies
    let attached = iam
        .list_attached_role_policies()
        .role_name(role_name)
        .send()
        .await
        .context("Failed to list attached role policies")?;

    for policy in attached.attached_policies() {
        let name = policy.policy_name().unwrap_or("Unknown");
        let arn = policy.policy_arn().unwrap_or("");

        let policy_type = if arn.contains(":aws:policy/") {
            PolicyType::AwsManaged
        } else {
            PolicyType::CustomerManaged
        };

        let statements = get_managed_policy_document(iam, arn)
            .await
            .unwrap_or_default();

        policies.push(PolicyInfo {
            name: name.to_string(),
            policy_type,
            statements,
        });
    }

    // Get inline policies
    let inline = iam
        .list_role_policies()
        .role_name(role_name)
        .send()
        .await
        .context("Failed to list inline role policies")?;

    for policy_name in inline.policy_names() {
        let policy_doc = iam
            .get_role_policy()
            .role_name(role_name)
            .policy_name(policy_name)
            .send()
            .await;

        if let Ok(doc) = policy_doc {
            let document = doc.policy_document();
            let decoded = urlencoding::decode(document).unwrap_or_else(|_| document.into());
            let statements = parse_policy_document(&decoded);

            policies.push(PolicyInfo {
                name: policy_name.to_string(),
                policy_type: PolicyType::Inline,
                statements,
            });
        }
    }

    Ok(policies)
}

async fn get_user_policies(iam: &aws_sdk_iam::Client, user_name: &str) -> Result<Vec<PolicyInfo>> {
    let mut policies = Vec::new();

    // Get attached managed policies
    let attached = iam
        .list_attached_user_policies()
        .user_name(user_name)
        .send()
        .await
        .context("Failed to list attached user policies")?;

    for policy in attached.attached_policies() {
        let name = policy.policy_name().unwrap_or("Unknown");
        let arn = policy.policy_arn().unwrap_or("");

        let policy_type = if arn.contains(":aws:policy/") {
            PolicyType::AwsManaged
        } else {
            PolicyType::CustomerManaged
        };

        let statements = get_managed_policy_document(iam, arn)
            .await
            .unwrap_or_default();

        policies.push(PolicyInfo {
            name: name.to_string(),
            policy_type,
            statements,
        });
    }

    // Get inline policies
    let inline = iam
        .list_user_policies()
        .user_name(user_name)
        .send()
        .await
        .context("Failed to list inline user policies")?;

    for policy_name in inline.policy_names() {
        let policy_doc = iam
            .get_user_policy()
            .user_name(user_name)
            .policy_name(policy_name)
            .send()
            .await;

        if let Ok(doc) = policy_doc {
            let document = doc.policy_document();
            let decoded = urlencoding::decode(document).unwrap_or_else(|_| document.into());
            let statements = parse_policy_document(&decoded);

            policies.push(PolicyInfo {
                name: policy_name.to_string(),
                policy_type: PolicyType::Inline,
                statements,
            });
        }
    }

    // Get group policies
    let groups = iam
        .list_groups_for_user()
        .user_name(user_name)
        .send()
        .await
        .ok();

    if let Some(groups) = groups {
        for group in groups.groups() {
            let group_name = group.group_name();

            // Attached group policies
            if let Ok(attached) = iam
                .list_attached_group_policies()
                .group_name(group_name)
                .send()
                .await
            {
                for policy in attached.attached_policies() {
                    let name = policy.policy_name().unwrap_or("Unknown");
                    let arn = policy.policy_arn().unwrap_or("");

                    let policy_type = if arn.contains(":aws:policy/") {
                        PolicyType::AwsManaged
                    } else {
                        PolicyType::CustomerManaged
                    };

                    let statements = get_managed_policy_document(iam, arn)
                        .await
                        .unwrap_or_default();

                    policies.push(PolicyInfo {
                        name: format!("{} (via group {})", name, group_name),
                        policy_type,
                        statements,
                    });
                }
            }

            // Inline group policies
            if let Ok(inline) = iam
                .list_group_policies()
                .group_name(group_name)
                .send()
                .await
            {
                for policy_name in inline.policy_names() {
                    if let Ok(doc) = iam
                        .get_group_policy()
                        .group_name(group_name)
                        .policy_name(policy_name)
                        .send()
                        .await
                    {
                        let document = doc.policy_document();
                        let decoded =
                            urlencoding::decode(document).unwrap_or_else(|_| document.into());
                        let statements = parse_policy_document(&decoded);

                        policies.push(PolicyInfo {
                            name: format!("{} (via group {})", policy_name, group_name),
                            policy_type: PolicyType::Inline,
                            statements,
                        });
                    }
                }
            }
        }
    }

    Ok(policies)
}

fn display_policy(policy: &PolicyInfo) {
    let type_label = match policy.policy_type {
        PolicyType::AwsManaged => "AWS Managed".dimmed(),
        PolicyType::CustomerManaged => "Customer Managed".dimmed(),
        PolicyType::Inline => "Inline".dimmed(),
    };

    println!(
        "\n{} {} ({})",
        "▸".blue(),
        policy.name.cyan().bold(),
        type_label
    );

    for stmt in &policy.statements {
        let effect_colored = if stmt.effect == "Allow" {
            stmt.effect.green()
        } else {
            stmt.effect.red()
        };

        println!("  {} {}", "Effect:".dimmed(), effect_colored);

        if !stmt.actions.is_empty() {
            println!("  {}", "Actions:".dimmed());
            for action in &stmt.actions {
                println!("    {} {}", "-".dimmed(), action);
            }
        }

        if !stmt.resources.is_empty() {
            println!("  {}", "Resources:".dimmed());
            for resource in &stmt.resources {
                println!("    {} {}", "-".dimmed(), resource);
            }
        }

        if let Some(conditions) = &stmt.conditions {
            println!("  {} {}", "Conditions:".dimmed(), conditions.dimmed());
        }
    }
}

async fn show_aws_identity(config: &aws_config::SdkConfig) -> Result<()> {
    let spinner = show_spinner("Fetching AWS identity...");
    let identity = get_identity_info(config).await?;
    spinner.finish_and_clear();

    print_header("AWS Identity");
    println!("  {} {}", "Account:".dimmed(), identity.account.white());
    println!("  {} {}", "Type:".dimmed(), identity.type_name().cyan());
    println!("  {} {}", "ARN:".dimmed(), identity.arn.white());
    println!("  {} {}", "Name:".dimmed(), identity.name().cyan().bold());

    let iam = aws_sdk_iam::Client::new(config);

    let spinner = show_spinner("Fetching policies...");
    let policies = match &identity.identity_type {
        IdentityType::User(name) => get_user_policies(&iam, name).await?,
        IdentityType::AssumedRole(role) => get_role_policies(&iam, role).await?,
        IdentityType::FederatedUser(_) => {
            spinner.finish_and_clear();
            print_warning("Federated users: permissions come from the assumed role's policies");
            return Ok(());
        }
        IdentityType::Unknown => {
            spinner.finish_and_clear();
            print_warning("Unknown identity type - cannot fetch policies");
            return Ok(());
        }
    };
    spinner.finish_and_clear();

    if policies.is_empty() {
        print_warning("No policies found");
        return Ok(());
    }

    print_header(&format!("Policies ({})", policies.len()));

    for policy in &policies {
        display_policy(policy);
    }

    println!();
    Ok(())
}

// ==================== EKS Cluster Functions ====================

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

async fn update_kubeconfig(
    config: &aws_config::SdkConfig,
    cluster_name: &str,
    profile: Option<&str>,
    region: &str,
) -> Result<()> {
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
    let mut exec_args = vec![
        "--region".to_string(),
        region.to_string(),
        "eks".to_string(),
        "get-token".to_string(),
        "--cluster-name".to_string(),
        cluster_name.to_string(),
        "--output".to_string(),
        "json".to_string(),
    ];

    if let Some(profile_name) = profile {
        exec_args.push("--profile".to_string());
        exec_args.push(profile_name.to_string());
    }

    let user_entry = KubeconfigUser {
        name: arn.to_string(),
        user: UserData {
            exec: ExecConfig {
                api_version: "client.authentication.k8s.io/v1beta1".to_string(),
                command: "aws".to_string(),
                args: exec_args,
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

fn display_pods(pods: &[String], env: Environment, settings: &Settings) {
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
            env.emoji(settings),
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
    settings: &Settings,
) -> Result<()> {
    let prompt_label = format!("{}-{}-{}", env.as_str(), pod_type, pod_num);
    let env_emoji = env.emoji(settings);

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

// ==================== Local Log Viewing ====================

fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = std::env::var("HOME").ok() {
            return path.replacen("~", &home, 1);
        }
    }
    path.to_string()
}

fn colorize_log_line(line: &str) -> String {
    // Colorize common log patterns
    let line = if line.contains("ERROR") || line.contains("error") || line.contains("Error") {
        line.red().to_string()
    } else if line.contains("WARN") || line.contains("warn") || line.contains("Warning") {
        line.yellow().to_string()
    } else if line.contains("INFO") || line.contains("info") {
        line.to_string()
    } else if line.contains("DEBUG") || line.contains("debug") {
        line.dimmed().to_string()
    } else {
        line.to_string()
    };

    // Try to colorize timestamp at the start (common formats)
    // e.g., "2024-01-09 12:34:56" or "[2024-01-09T12:34:56]"
    if let Some(idx) = line.find(|c: char| c.is_ascii_digit()) {
        if idx < 5 {
            // Timestamp likely at start
            let timestamp_end = line
                .char_indices()
                .take(30)
                .find(|(_, c)| *c == ']' || *c == ' ' && line[idx..].contains(':'))
                .map(|(i, _)| i + 1)
                .unwrap_or(0);

            if timestamp_end > idx + 10 {
                let (timestamp, rest) = line.split_at(timestamp_end);
                return format!("{}{}", timestamp.bright_black(), rest);
            }
        }
    }

    line
}

fn view_local_log(
    path: &str,
    follow: bool,
    lines: usize,
    grep: Option<&str>,
    colorize: bool,
) -> Result<()> {
    let path = expand_tilde(path);
    let path = PathBuf::from(&path);

    if !path.exists() {
        bail!("Log file not found: {:?}", path);
    }

    print_header(&format!("Log: {}", path.display().to_string().bright_cyan()));

    if follow {
        println!("  {} to stop", "Press Ctrl+C".yellow());
        println!();

        let running = Arc::new(AtomicBool::new(true));
        let r = running.clone();

        ctrlc::set_handler(move || {
            r.store(false, Ordering::Relaxed);
            println!("\n{}", "Stopped.".yellow());
            std::process::exit(0);
        })
        .context("Failed to set Ctrl+C handler")?;

        // First show last N lines
        let file = std::fs::File::open(&path)?;
        let reader = BufReader::new(file);
        let all_lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();
        let start = all_lines.len().saturating_sub(lines);

        for line in &all_lines[start..] {
            if let Some(pattern) = grep {
                if !line.contains(pattern) {
                    continue;
                }
            }
            if colorize {
                println!("{}", colorize_log_line(line));
            } else {
                println!("{}", line);
            }
        }

        // Now tail
        let mut last_size = std::fs::metadata(&path)?.len();
        let mut last_pos = last_size;

        while running.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(100));

            let current_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(last_size);

            if current_size > last_pos {
                let file = std::fs::File::open(&path)?;
                let mut reader = BufReader::new(file);
                std::io::Seek::seek(&mut reader, std::io::SeekFrom::Start(last_pos))?;

                for line in reader.lines() {
                    if let Ok(line) = line {
                        if let Some(pattern) = grep {
                            if !line.contains(pattern) {
                                continue;
                            }
                        }
                        if colorize {
                            println!("{}", colorize_log_line(&line));
                        } else {
                            println!("{}", line);
                        }
                    }
                }

                last_pos = current_size;
            } else if current_size < last_size {
                // File was truncated/rotated
                last_pos = 0;
            }
            last_size = current_size;
        }
    } else {
        // Just show last N lines
        let file = std::fs::File::open(&path)?;
        let reader = BufReader::new(file);
        let all_lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();
        let start = all_lines.len().saturating_sub(lines);

        for line in &all_lines[start..] {
            if let Some(pattern) = grep {
                if !line.contains(pattern) {
                    continue;
                }
            }
            if colorize {
                println!("{}", colorize_log_line(line));
            } else {
                println!("{}", line);
            }
        }

        println!();
        println!(
            "{} {} lines (use {} to follow)",
            "Showing last".dimmed(),
            (all_lines.len() - start).to_string().cyan(),
            "-f".yellow()
        );
    }

    Ok(())
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

    // Load settings from config file
    let settings = config::load_settings().context("Failed to load settings")?;

    // Handle subcommands first (they don't need AWS)
    if let Some(command) = &args.command {
        match command {
            Commands::Log {
                env,
                path,
                follow,
                lines,
                grep,
                colorize,
            } => {
                let log_path = if let Some(p) = path {
                    p.clone()
                } else {
                    let env_name = env
                        .map(|e| e.long_name())
                        .or_else(|| detect_env().map(|e| e.long_name()))
                        .unwrap_or("development");
                    settings.logging.log_path.replace("{env}", env_name)
                };
                return view_local_log(&log_path, *follow, *lines, grep.as_deref(), *colorize);
            }
        }
    }

    // CLI args override config file, config file overrides defaults
    let profile = args.aws_profile.as_deref().or(settings.aws.profile.as_deref());
    let region = &settings.aws.region;
    let namespace = args
        .namespace
        .as_deref()
        .unwrap_or(&settings.kubernetes.namespace);
    let pod_type = args
        .pod_type
        .as_deref()
        .unwrap_or(&settings.kubernetes.pod_type);

    // Load AWS config (needed for all AWS operations)
    let aws_config = get_aws_config(profile, region).await;

    // Check AWS session
    let spinner = show_spinner("Checking AWS SSO session...");
    if !check_aws_session(&aws_config).await {
        spinner.finish_and_clear();
        print_warning("SSO session expired. Logging in...");
        aws_sso_login(profile)?;
        // Reload config after login
        let aws_config = get_aws_config(profile, region).await;
        if !check_aws_session(&aws_config).await {
            print_error("AWS session still invalid after login");
            std::process::exit(1);
        }
    } else {
        spinner.finish_and_clear();
    }
    print_success("AWS session active");

    // Handle --whoami before EKS-specific logic
    if args.whoami {
        return show_aws_identity(&aws_config).await;
    }

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

    let cluster = env.cluster(&settings);

    // Resolve log file path
    let log_file = match &args.log {
        Some(Some(path)) => Some(path.clone()),
        Some(None) => Some(settings.logging.log_path.replace("{env}", env.long_name())),
        None => None,
    };

    // Update kubeconfig
    let spinner = show_spinner(&format!("Updating kubeconfig for {}...", cluster));
    update_kubeconfig(&aws_config, cluster, profile, region).await?;
    spinner.finish_and_clear();
    print_success(&format!("Connected to {}", cluster.bold()));

    // Get pods
    let spinner = show_spinner(&format!(
        "Fetching pods matching '{}' in namespace '{}'...",
        pod_type, namespace
    ));
    let pods = get_pods(namespace, pod_type);
    spinner.finish_and_clear();

    if pods.is_empty() {
        print_error(&format!("No pods found matching '{}'", pod_type));
        std::process::exit(1);
    }

    print_success(&format!("Found {} pods", pods.len()));
    display_pods(&pods, env, &settings);

    // Log mode - tail from all pods
    if let Some(log_path) = log_file {
        return tail_logs(&pods, namespace, &log_path);
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
    exec_into_pod(pod, namespace, env, pod_type, pod_num, &settings)
}
