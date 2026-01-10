mod aws;
mod commands;
mod config;
mod github;
mod jira;
mod utils;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use colored::Colorize;

use config::Settings;
use utils::{print_error, print_info, print_success, print_warning, run_cmd, spinner};

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq)]
pub enum Environment {
    Prod,
    Dev,
    Stg,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
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
    hu eks                                 \x1b[2m# List web pods\x1b[0m
    hu eks -p 1                            \x1b[2m# Connect to pod #1\x1b[0m
    hu eks -e prod -t api                  \x1b[2m# List api pods on prod\x1b[0m
    hu eks --log                           \x1b[2m# Tail logs from all pods\x1b[0m
    hu aws whoami                          \x1b[2m# Show AWS identity\x1b[0m
    hu log                                 \x1b[2m# View local log file\x1b[0m
    hu log -f                              \x1b[2m# Tail local log file\x1b[0m
    hu jira show PROJ-123                  \x1b[2m# Show issue details\x1b[0m
    hu jira search \"bug login\"             \x1b[2m# Search issues\x1b[0m
    hu jira mine                           \x1b[2m# My assigned issues\x1b[0m")]
struct Args {
    #[command(subcommand)]
    command: Commands,

    /// AWS profile to use
    #[arg(long = "aws-profile", global = true)]
    aws_profile: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// EKS cluster and pod operations
    #[command(alias = "k8s", alias = "kube")]
    Eks {
        #[command(subcommand)]
        action: Option<EksCommands>,

        /// Environment
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

        /// Tail log file from all pods
        #[arg(short, long)]
        log: Option<Option<String>>,
    },

    /// AWS operations
    Aws {
        #[command(subcommand)]
        action: AwsCommands,
    },

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

        /// Number of lines to show
        #[arg(short = 'n', long, default_value = "50")]
        lines: usize,

        /// Filter lines containing this pattern
        #[arg(short = 'g', long)]
        grep: Option<String>,

        /// Colorize output
        #[arg(long, default_value = "true")]
        colorize: bool,
    },

    /// Jira ticket operations
    Jira {
        #[command(subcommand)]
        action: Option<JiraCommands>,
    },

    /// GitHub operations
    #[command(name = "gh")]
    GitHub {
        #[command(subcommand)]
        action: Option<GitHubCommands>,
    },

    /// EC2 instance operations (read-only)
    Ec2 {
        /// Filter by Environment tag (prod, dev, stg)
        #[arg(short, long, value_enum)]
        env: Option<Environment>,

        /// Filter by Name tag pattern
        #[arg(short = 't', long = "tag")]
        name_filter: Option<String>,

        /// Show all instances (including unnamed/terminated)
        #[arg(long)]
        all: bool,

        /// Show only stopped instances
        #[arg(long)]
        stopped: bool,
    },
}

#[derive(Subcommand, Debug)]
enum EksCommands {
    /// List pods (default action)
    Pods {
        /// Environment
        #[arg(short, long, value_enum)]
        env: Option<Environment>,

        /// Pod name pattern to filter
        #[arg(short = 't', long = "type")]
        pod_type: Option<String>,

        /// Kubernetes namespace
        #[arg(short, long)]
        namespace: Option<String>,
    },

    /// Execute into a pod
    Exec {
        /// Pod number to connect to
        #[arg(short, long)]
        pod: usize,

        /// Environment
        #[arg(short, long, value_enum)]
        env: Option<Environment>,

        /// Pod name pattern to filter
        #[arg(short = 't', long = "type")]
        pod_type: Option<String>,

        /// Kubernetes namespace
        #[arg(short, long)]
        namespace: Option<String>,
    },

    /// Tail logs from pods
    Logs {
        /// Environment
        #[arg(short, long, value_enum)]
        env: Option<Environment>,

        /// Log file path on pods
        #[arg(short, long)]
        path: Option<String>,

        /// Pod name pattern to filter
        #[arg(short = 't', long = "type")]
        pod_type: Option<String>,

        /// Kubernetes namespace
        #[arg(short, long)]
        namespace: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum AwsCommands {
    /// Show AWS identity and permissions
    Whoami,

    /// Login to AWS SSO
    Login,

    /// Discover all AWS profiles and their capabilities (read-only)
    Discover {
        /// Include expired/invalid profiles in output
        #[arg(long)]
        all: bool,

        /// Output as JSON for scripting
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum JiraCommands {
    /// Configure Jira OAuth credentials
    Setup,

    /// Login to Jira via browser OAuth
    Login,

    /// Show a specific issue
    Show {
        /// Issue key (e.g., PROJ-123)
        key: String,
    },

    /// Search issues with JQL
    Search {
        /// JQL query or text search
        query: String,

        /// Maximum results to return
        #[arg(short = 'n', long, default_value = "20")]
        max: u32,
    },

    /// Search issues assigned to me
    Mine {
        /// Maximum results to return
        #[arg(short = 'n', long, default_value = "20")]
        max: u32,
    },

    /// My issues in current sprint (default)
    Sprint {
        /// Maximum results to return
        #[arg(short = 'n', long, default_value = "20")]
        max: u32,
    },

    /// Show project details
    Project {
        /// Project key (e.g., PROJ)
        key: String,
    },

    /// List all projects
    Projects,
}

#[derive(Subcommand, Debug)]
enum GitHubCommands {
    /// Configure GitHub token
    Setup,

    /// List workflow runs (default)
    Runs {
        /// Repository (owner/repo or full git URL), defaults to current git repo
        #[arg(short, long)]
        repo: Option<String>,

        /// Filter by actor (GitHub username)
        #[arg(short, long)]
        actor: Option<String>,

        /// Filter by workflow name (partial match)
        #[arg(short, long)]
        workflow: Option<String>,

        /// Jira project to filter by active sprint tasks (e.g., BFR)
        #[arg(short, long)]
        project: Option<String>,

        /// Show all branches (default: only active sprint task branches)
        #[arg(long)]
        all_branches: bool,

        /// Show only running + successful (default: all states)
        #[arg(long)]
        ok: bool,

        /// Maximum results
        #[arg(short = 'n', long, default_value = "15")]
        max: u32,
    },
}

fn detect_env() -> Option<Environment> {
    let context = run_cmd(&["kubectl", "config", "current-context"])?;
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

fn resolve_env(env: Option<Environment>, settings: &Settings) -> String {
    env.map(|e| e.as_str().to_string())
        .or_else(|| detect_env().map(|e| e.as_str().to_string()))
        .unwrap_or_else(|| settings.default_env_name().to_string())
}

/// Resolve the effective profile: CLI flag > config profile > None
fn resolve_profile<'a>(
    cli_profile: Option<&'a str>,
    config_profile: Option<&'a str>,
) -> Option<&'a str> {
    cli_profile.or(config_profile)
}

async fn ensure_aws_session(profile: Option<&str>, region: &str) -> Result<aws_config::SdkConfig> {
    let aws_config = aws::get_config(profile, region).await;

    let spin = spinner("Checking AWS SSO session...");
    if !aws::check_session(&aws_config).await {
        spin.finish_and_clear();
        print_warning("SSO session expired. Logging in...");
        aws::sso_login(profile)?;
        let aws_config = aws::get_config(profile, region).await;
        if !aws::check_session(&aws_config).await {
            print_error("AWS session still invalid after login");
            std::process::exit(1);
        }
        print_success("AWS session active");
        return Ok(aws_config);
    }
    spin.finish_and_clear();
    print_success("AWS session active");
    Ok(aws_config)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let settings = config::load_settings().context("Failed to load settings")?;

    // CLI --aws-profile flag takes precedence over all config profiles
    let cli_profile = args.aws_profile.as_deref();

    match args.command {
        Commands::Log {
            env,
            path,
            follow,
            lines,
            grep,
            colorize,
        } => {
            let log_path = if let Some(p) = path {
                p
            } else {
                let env_name = resolve_env(env, &settings);
                let env_config = settings.get_env(&env_name);
                let log_name = env_config.log_name.unwrap_or(env_name);
                settings.logging.log_path.replace("{env}", &log_name)
            };
            commands::log::view(&log_path, follow, lines, grep.as_deref(), colorize)
        }

        Commands::Aws { action } => {
            match action {
                AwsCommands::Discover { all, json } => {
                    // Discovery checks all profiles individually, no pre-login needed
                    aws::discover(&settings.aws.region, all, json).await
                }
                _ => {
                    // Other AWS commands use general profile
                    let profile =
                        resolve_profile(cli_profile, settings.aws.profiles.general_profile());
                    let aws_config = ensure_aws_session(profile, &settings.aws.region).await?;
                    match action {
                        AwsCommands::Whoami => aws::whoami(&aws_config).await,
                        AwsCommands::Login => {
                            aws::sso_login(profile)?;
                            print_success("Logged in successfully");
                            Ok(())
                        }
                        AwsCommands::Discover { .. } => unreachable!(),
                    }
                }
            }
        }

        Commands::Eks {
            action,
            env,
            pod_type,
            pod,
            namespace,
            log,
        } => {
            // Use eks profile for EKS/Kubernetes operations
            let profile = resolve_profile(cli_profile, settings.aws.profiles.eks_profile());
            let aws_config = ensure_aws_session(profile, &settings.aws.region).await?;

            // Handle subcommands or default behavior
            match action {
                Some(EksCommands::Pods {
                    env: sub_env,
                    pod_type: sub_type,
                    namespace: sub_ns,
                }) => {
                    let env_name = resolve_env(sub_env.or(env), &settings);
                    let ns = sub_ns
                        .or(namespace)
                        .unwrap_or_else(|| settings.kubernetes.namespace.clone());
                    let pt = sub_type
                        .or(pod_type)
                        .unwrap_or_else(|| settings.kubernetes.pod_type.clone());

                    if let Some(detected) = detect_env() {
                        if detected.as_str() != env_name {
                            print_info(&format!("Using environment: {}", env_name.bold()));
                        }
                    }

                    commands::eks::run(
                        &aws_config,
                        &settings,
                        &env_name,
                        profile,
                        &ns,
                        &pt,
                        None,
                        None,
                    )
                    .await
                }

                Some(EksCommands::Exec {
                    pod: pod_num,
                    env: sub_env,
                    pod_type: sub_type,
                    namespace: sub_ns,
                }) => {
                    let env_name = resolve_env(sub_env.or(env), &settings);
                    let ns = sub_ns
                        .or(namespace)
                        .unwrap_or_else(|| settings.kubernetes.namespace.clone());
                    let pt = sub_type
                        .or(pod_type)
                        .unwrap_or_else(|| settings.kubernetes.pod_type.clone());

                    commands::eks::run(
                        &aws_config,
                        &settings,
                        &env_name,
                        profile,
                        &ns,
                        &pt,
                        Some(pod_num),
                        None,
                    )
                    .await
                }

                Some(EksCommands::Logs {
                    env: sub_env,
                    path,
                    pod_type: sub_type,
                    namespace: sub_ns,
                }) => {
                    let env_name = resolve_env(sub_env.or(env), &settings);
                    let ns = sub_ns
                        .or(namespace)
                        .unwrap_or_else(|| settings.kubernetes.namespace.clone());
                    let pt = sub_type
                        .or(pod_type)
                        .unwrap_or_else(|| settings.kubernetes.pod_type.clone());

                    let env_config = settings.get_env(&env_name);
                    let log_name = env_config
                        .log_name
                        .clone()
                        .unwrap_or_else(|| env_name.clone());
                    let log_path = path
                        .unwrap_or_else(|| settings.logging.log_path.replace("{env}", &log_name));

                    commands::eks::run(
                        &aws_config,
                        &settings,
                        &env_name,
                        profile,
                        &ns,
                        &pt,
                        None,
                        Some(log_path),
                    )
                    .await
                }

                None => {
                    // Default: list pods, optionally connect or tail logs
                    let env_name = resolve_env(env, &settings);
                    let ns = namespace.unwrap_or_else(|| settings.kubernetes.namespace.clone());
                    let pt = pod_type.unwrap_or_else(|| settings.kubernetes.pod_type.clone());

                    if let Some(detected) = detect_env() {
                        print_info(&format!(
                            "Detected environment: {} (from current context)",
                            detected.as_str().bold()
                        ));
                    }

                    let log_file = match log {
                        Some(Some(path)) => Some(path),
                        Some(None) => {
                            let env_config = settings.get_env(&env_name);
                            let log_name = env_config.log_name.unwrap_or_else(|| env_name.clone());
                            Some(settings.logging.log_path.replace("{env}", &log_name))
                        }
                        None => None,
                    };

                    commands::eks::run(
                        &aws_config,
                        &settings,
                        &env_name,
                        profile,
                        &ns,
                        &pt,
                        pod,
                        log_file,
                    )
                    .await
                }
            }
        }

        Commands::Jira { action } => {
            let action = action.unwrap_or(JiraCommands::Sprint { max: 20 });
            match action {
                JiraCommands::Setup => jira::setup(),

                JiraCommands::Login => {
                    let mut config = jira::load_jira_config()?;
                    jira::login(&mut config).await
                }

                JiraCommands::Show { key } => {
                    let config = jira::load_jira_config()?;
                    let issue = jira::get_issue(&config, &key).await?;
                    jira::display_issue(&issue);
                    Ok(())
                }

                JiraCommands::Search { query, max } => {
                    let config = jira::load_jira_config()?;
                    // If query doesn't look like JQL, wrap it in a text search
                    let jql =
                        if query.contains('=') || query.contains(" AND ") || query.contains(" OR ")
                        {
                            query
                        } else {
                            format!("text ~ \"{}\"", query)
                        };
                    let result = jira::search_issues(&config, &jql, max).await?;
                    jira::display_search_results(&result);
                    Ok(())
                }

                JiraCommands::Mine { max } => {
                    let config = jira::load_jira_config()?;
                    let jql = "assignee = currentUser() ORDER BY updated DESC";
                    let result = jira::search_issues(&config, jql, max).await?;
                    jira::display_search_results(&result);
                    Ok(())
                }

                JiraCommands::Sprint { max } => {
                    let config = jira::load_jira_config()?;
                    let jql = "assignee = currentUser() AND sprint in openSprints() ORDER BY status ASC, priority DESC";
                    let result = jira::search_issues(&config, jql, max).await?;
                    jira::display_search_results(&result);
                    Ok(())
                }

                JiraCommands::Project { key } => {
                    let config = jira::load_jira_config()?;
                    let project = jira::get_project(&config, &key).await?;
                    jira::display_project(&project);
                    Ok(())
                }

                JiraCommands::Projects => {
                    let config = jira::load_jira_config()?;
                    let projects = jira::list_projects(&config).await?;
                    jira::display_projects(&projects);
                    Ok(())
                }
            }
        }

        Commands::GitHub { action } => {
            let action = action.unwrap_or(GitHubCommands::Runs {
                repo: None,
                actor: None,
                workflow: None,
                project: None,
                all_branches: false,
                ok: false,
                max: 15,
            });
            match action {
                GitHubCommands::Setup => github::setup(),

                GitHubCommands::Runs {
                    repo,
                    actor,
                    workflow,
                    project,
                    all_branches,
                    ok,
                    max,
                } => {
                    let gh_config = github::load_github_config()?;

                    // Get project key from CLI, github.json, or settings.toml
                    let project_key = project
                        .or_else(|| gh_config.default_project.clone())
                        .or_else(|| settings.github.default_project.clone());

                    // Filter by project key prefix unless --all-branches
                    let filter_project_key = if all_branches {
                        None
                    } else {
                        project_key.clone()
                    };

                    // Check if we should use project repos (no explicit repo and project has repos)
                    let project_config = project_key
                        .as_ref()
                        .and_then(|key| settings.project.projects.get(key));

                    if repo.is_none()
                        && project_config.map(|p| !p.repos.is_empty()).unwrap_or(false)
                    {
                        // Multi-repo mode: fetch from all project repos
                        let project_config = project_config.unwrap();

                        // Build repo list from project config
                        let repos: Vec<github::RepoInfo> = project_config
                            .repos
                            .iter()
                            .map(|(label, repo_cfg)| github::RepoInfo {
                                github: repo_cfg.github.clone(),
                                label: label.clone(),
                            })
                            .collect();

                        // Use project-level defaults for actor/workflow, fall back to github.json
                        let actor = actor
                            .or_else(|| project_config.github_actor.clone())
                            .or_else(|| gh_config.default_actor.clone());
                        let workflow = workflow
                            .or_else(|| project_config.github_workflow.clone())
                            .or_else(|| gh_config.default_workflow.clone());

                        let filter = github::RunsFilter {
                            actor: actor.as_deref(),
                            workflow: workflow.as_deref(),
                            success_only: ok,
                            project_key: filter_project_key.as_deref(),
                        };

                        let runs =
                            github::get_project_workflow_runs(&gh_config, &repos, &filter, max)
                                .await?;
                        github::display_project_workflow_runs(&runs, &project_config.name);
                    } else {
                        // Single repo mode
                        let repo = repo
                            .map(|r| github::normalize_repo(&r))
                            .or_else(|| gh_config.default_repo.clone())
                            .or_else(github::detect_repo)
                            .context(
                                "No repository specified. Use -r owner/repo or run from a git directory",
                            )?;

                        let actor = actor.or_else(|| gh_config.default_actor.clone());
                        let workflow = workflow.or_else(|| gh_config.default_workflow.clone());

                        let filter = github::RunsFilter {
                            actor: actor.as_deref(),
                            workflow: workflow.as_deref(),
                            success_only: ok,
                            project_key: filter_project_key.as_deref(),
                        };
                        let runs =
                            github::get_workflow_runs(&gh_config, &repo, &filter, max).await?;
                        github::display_workflow_runs(&runs, &repo);
                    }
                    Ok(())
                }
            }
        }

        Commands::Ec2 {
            env,
            name_filter,
            all,
            stopped,
        } => {
            let filter = aws::Ec2Filter {
                env: env.map(|e| e.as_str().to_string()),
                name_filter,
                show_all: all,
                stopped_only: stopped,
            };
            let instances = aws::list_instances(&settings.aws.region, &filter).await?;
            aws::display_instances(&instances);
            Ok(())
        }
    }
}
