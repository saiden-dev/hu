use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::utils::{print_error, print_header, print_success};

const GITHUB_API_URL: &str = "https://api.github.com";

// ==================== Config ====================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitHubConfig {
    pub token: Option<String>,
    pub default_repo: Option<String>,
    pub default_actor: Option<String>,
    pub default_workflow: Option<String>,
    pub default_project: Option<String>,
}

fn get_github_config_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir().context("Could not determine config directory")?;
    Ok(config_dir.join("hu").join("github.json"))
}

pub fn load_github_config() -> Result<GitHubConfig> {
    let path = get_github_config_path()?;
    if path.exists() {
        let content = std::fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&content)?)
    } else {
        Ok(GitHubConfig::default())
    }
}

pub fn save_github_config(config: &GitHubConfig) -> Result<()> {
    let path = get_github_config_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(config)?;
    std::fs::write(&path, content)?;
    Ok(())
}

// ==================== API Types ====================

#[derive(Debug, Deserialize)]
pub struct WorkflowRun {
    pub name: String,
    pub head_branch: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub display_title: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WorkflowRunsResponse {
    pub workflow_runs: Vec<WorkflowRun>,
}

// ==================== API Functions ====================

fn get_client(config: &GitHubConfig) -> Result<(reqwest::Client, String)> {
    let token = config.token.as_ref().context(
        "GitHub token not configured. Run: hu gh setup\n\
         Create a token at: https://github.com/settings/tokens",
    )?;

    let client = reqwest::Client::builder().user_agent("hu-cli").build()?;

    Ok((client, token.clone()))
}

pub struct RunsFilter<'a> {
    pub actor: Option<&'a str>,
    pub workflow: Option<&'a str>,
    pub success_only: bool,
    pub project_key: Option<&'a str>,
}

pub async fn get_workflow_runs(
    config: &GitHubConfig,
    repo: &str,
    filter: &RunsFilter<'_>,
    limit: u32,
) -> Result<WorkflowRunsResponse> {
    let (client, token) = get_client(config)?;

    let url = format!("{}/repos/{}/actions/runs", GITHUB_API_URL, repo);

    // Fetch more than requested to allow for client-side filtering
    let fetch_limit =
        if filter.workflow.is_some() || filter.success_only || filter.project_key.is_some() {
            (limit * 5).min(100)
        } else {
            limit
        };

    let mut query = vec![("per_page", fetch_limit.to_string())];
    if let Some(actor) = filter.actor {
        query.push(("actor", actor.to_string()));
    }

    let response = client
        .get(&url)
        .bearer_auth(&token)
        .query(&query)
        .send()
        .await?;

    if response.status() == 401 {
        bail!("Unauthorized. Check your GitHub token: hu gh setup");
    }

    if response.status() == 404 {
        bail!("Repository not found: {}", repo);
    }

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        bail!("GitHub API error ({}): {}", status, text);
    }

    let mut result: WorkflowRunsResponse = response.json().await?;

    // Client-side filtering
    result.workflow_runs = result
        .workflow_runs
        .into_iter()
        .filter(|run| {
            // Filter by workflow name (case-insensitive partial match)
            if let Some(wf) = filter.workflow {
                let wf_lower = wf.to_lowercase();
                if !run.name.to_lowercase().contains(&wf_lower) {
                    return false;
                }
            }

            // Filter by project key prefix in branch (e.g., BFR-)
            if let Some(key) = filter.project_key {
                let branch_upper = run.head_branch.to_uppercase();
                let key_prefix = format!("{}-", key.to_uppercase());
                if !branch_upper.contains(&key_prefix) {
                    return false;
                }
            }

            // Filter by status if --ok flag is set
            if filter.success_only && !is_running_or_successful(run) {
                return false;
            }

            true
        })
        .take(limit as usize)
        .collect();

    Ok(result)
}

fn is_running_or_successful(run: &WorkflowRun) -> bool {
    match (run.status.as_str(), run.conclusion.as_deref()) {
        // Running states
        ("in_progress", _) | ("queued", _) | ("waiting", _) | ("pending", _) => true,
        // Successful
        ("completed", Some("success")) => true,
        // Everything else (failure, cancelled, etc.)
        _ => false,
    }
}

// ==================== Display ====================

pub fn display_workflow_runs(runs: &WorkflowRunsResponse, repo: &str) {
    use colored::Colorize;
    use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, Color, Table};

    if runs.workflow_runs.is_empty() {
        print_error("No workflow runs found");
        return;
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("").fg(Color::Yellow),
            Cell::new("Title").fg(Color::White),
            Cell::new("Branch").fg(Color::Magenta),
        ]);

    for run in &runs.workflow_runs {
        let status_icon = match (run.status.as_str(), run.conclusion.as_deref()) {
            ("completed", Some("success")) => "✓".green().to_string(),
            ("completed", Some("failure")) => "✗".red().to_string(),
            ("completed", Some("cancelled")) => "⊘".dimmed().to_string(),
            ("in_progress", _) => "●".yellow().to_string(),
            ("queued", _) | ("waiting", _) => "○".blue().to_string(),
            _ => "?".white().to_string(),
        };

        // Use display_title (PR title) which includes Jira ticket
        let title = run
            .display_title
            .as_ref()
            .map(|t| {
                if t.len() > 55 {
                    format!("{}...", &t[..52])
                } else {
                    t.clone()
                }
            })
            .unwrap_or_else(|| run.name.clone());

        let branch = if run.head_branch.len() > 25 {
            format!("{}...", &run.head_branch[..22])
        } else {
            run.head_branch.clone()
        };

        table.add_row(vec![
            Cell::new(&status_icon),
            Cell::new(&title).fg(Color::White),
            Cell::new(&branch).fg(Color::DarkGrey),
        ]);
    }

    println!();
    println!(
        "{}",
        format!("{} - {} workflow runs", repo, runs.workflow_runs.len()).dimmed()
    );
    println!("{table}");
    println!();
}

// ==================== Setup ====================

pub fn setup() -> Result<()> {
    use std::io::{stdin, stdout, Write};

    print_header("GitHub Setup");
    println!("Create a personal access token at:");
    println!("  https://github.com/settings/tokens");
    println!();
    println!("Required scopes: repo, workflow");
    println!();

    let mut config = load_github_config()?;

    print!("GitHub Token: ");
    stdout().flush()?;
    let mut token = String::new();
    stdin().read_line(&mut token)?;
    config.token = Some(token.trim().to_string());

    print!("Default repo (owner/repo, optional): ");
    stdout().flush()?;
    let mut repo = String::new();
    stdin().read_line(&mut repo)?;
    let repo = repo.trim();
    if !repo.is_empty() {
        config.default_repo = Some(repo.to_string());
    }

    print!("Default actor (GitHub username, optional): ");
    stdout().flush()?;
    let mut actor = String::new();
    stdin().read_line(&mut actor)?;
    let actor = actor.trim();
    if !actor.is_empty() {
        config.default_actor = Some(actor.to_string());
    }

    print!("Default workflow filter (partial name, optional): ");
    stdout().flush()?;
    let mut workflow = String::new();
    stdin().read_line(&mut workflow)?;
    let workflow = workflow.trim();
    if !workflow.is_empty() {
        config.default_workflow = Some(workflow.to_string());
    }

    save_github_config(&config)?;
    print_success("GitHub credentials saved!");

    Ok(())
}

// ==================== Helpers ====================

/// Detect repo from current git directory
pub fn detect_repo() -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let url = String::from_utf8_lossy(&output.stdout);
    parse_repo_from_url(url.trim())
}

/// Normalize a repo string - handles owner/repo, SSH URLs, and HTTPS URLs
pub fn normalize_repo(input: &str) -> String {
    parse_repo_from_url(input).unwrap_or_else(|| input.to_string())
}

fn parse_repo_from_url(url: &str) -> Option<String> {
    // Handle SSH format: git@github.com:owner/repo.git
    if url.starts_with("git@github.com:") {
        let path = url.strip_prefix("git@github.com:")?;
        let repo = path.strip_suffix(".git").unwrap_or(path);
        return Some(repo.to_string());
    }

    // Handle HTTPS format: https://github.com/owner/repo.git
    if url.contains("github.com/") {
        let parts: Vec<&str> = url.split("github.com/").collect();
        if parts.len() >= 2 {
            let repo = parts[1].strip_suffix(".git").unwrap_or(parts[1]);
            return Some(repo.to_string());
        }
    }

    None
}
