use anyhow::Result;

use super::cli::RunsArgs;
use super::client::{GithubApi, GithubClient};
use super::helpers::{get_current_repo, parse_owner_repo};
use super::types::{RunsQuery, WorkflowRun};

#[cfg(test)]
mod tests;

// ANSI color codes
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";
const GRAY: &str = "\x1b[90m";
const RESET: &str = "\x1b[0m";

/// Handle the `hu gh runs` command
pub async fn run(args: RunsArgs) -> Result<()> {
    let client = GithubClient::new()?;
    let (owner, repo) = match &args.repo {
        Some(r) => parse_owner_repo(r)?,
        None => get_current_repo()?,
    };
    run_with_client(&client, &owner, &repo, &args).await
}

/// Fetch and display workflow runs using the given API client
pub async fn run_with_client(
    client: &impl GithubApi,
    owner: &str,
    repo: &str,
    args: &RunsArgs,
) -> Result<()> {
    let runs = if let Some(ticket) = &args.ticket {
        fetch_runs_for_ticket(client, owner, repo, ticket, args).await?
    } else {
        let query = RunsQuery {
            owner,
            repo,
            branch: args.branch.as_deref(),
            status: args.status.as_deref(),
            limit: args.limit,
        };
        client.list_workflow_runs(&query).await?
    };

    if runs.is_empty() {
        println!("No workflow runs found.");
        return Ok(());
    }

    if args.json {
        print_runs_json(&runs);
    } else {
        print_runs_table(&runs);
    }

    Ok(())
}

/// Find runs associated with a ticket by searching PRs and their branches
async fn fetch_runs_for_ticket(
    client: &impl GithubApi,
    owner: &str,
    repo: &str,
    ticket: &str,
    args: &RunsArgs,
) -> Result<Vec<WorkflowRun>> {
    let prs = client.search_prs_by_title(owner, repo, ticket).await?;

    if prs.is_empty() {
        return Ok(vec![]);
    }

    let mut all_runs = Vec::new();
    let mut seen_branches = std::collections::HashSet::new();

    for pr in &prs {
        // Get the branch for this PR
        let parts: Vec<&str> = pr.repo_full_name.split('/').collect();
        let (pr_owner, pr_repo) = if parts.len() == 2 {
            (parts[0], parts[1])
        } else {
            (owner, repo)
        };

        if let Ok(branch) = client.get_pr_branch(pr_owner, pr_repo, pr.number).await {
            if seen_branches.insert(branch.clone()) {
                let query = RunsQuery {
                    owner: pr_owner,
                    repo: pr_repo,
                    branch: Some(&branch),
                    status: args.status.as_deref(),
                    limit: args.limit,
                };
                let runs = client.list_workflow_runs(&query).await?;
                all_runs.extend(runs);
            }
        }
    }

    // Sort by created_at descending and truncate to limit
    all_runs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    all_runs.truncate(args.limit);

    Ok(all_runs)
}

/// Get status icon with color for a workflow run
fn status_icon(run: &WorkflowRun) -> String {
    match run.conclusion.as_deref() {
        Some("success") => format!("{GREEN}✓{RESET}"),
        Some("failure") => format!("{RED}✗{RESET}"),
        Some("cancelled") => format!("{GRAY}○{RESET}"),
        _ => match run.status.as_str() {
            "in_progress" => format!("{YELLOW}◐{RESET}"),
            "queued" => format!("{GRAY}○{RESET}"),
            _ => format!("{GRAY}○{RESET}"),
        },
    }
}

fn get_terminal_width() -> usize {
    terminal_size::terminal_size()
        .map(|(w, _)| w.0 as usize)
        .unwrap_or(80)
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len.saturating_sub(1)).collect();
        format!("{truncated}…")
    }
}

fn print_runs_table(runs: &[WorkflowRun]) {
    let term_width = get_terminal_width();

    let max_link_len = runs.iter().map(|r| r.html_url.len()).max().unwrap_or(40);
    let max_branch_len = runs
        .iter()
        .map(|r| r.branch.len())
        .max()
        .unwrap_or(10)
        .min(30);

    // Layout: │ S │ Name │ Branch │ Link │
    let status_col = 1;
    let border_overhead = 14; // "│ " + " │ " + " │ " + " │ " + "│"

    let available =
        term_width.saturating_sub(border_overhead + status_col + max_branch_len + max_link_len);
    let name_width = available.max(15);
    let branch_width = max_branch_len;
    let link_width = max_link_len;

    // Top border
    println!(
        "┌───┬{}┬{}┬{}┐",
        "─".repeat(name_width + 2),
        "─".repeat(branch_width + 2),
        "─".repeat(link_width + 2),
    );

    for run in runs {
        let icon = status_icon(run);
        let name = truncate(&run.name, name_width);
        let branch = truncate(&run.branch, branch_width);
        let link = format!("{GRAY}{}{RESET}", &run.html_url);

        println!(
            "│ {} │ {:<nw$} │ {:<bw$} │ {} │",
            icon,
            name,
            branch,
            link,
            nw = name_width,
            bw = branch_width,
        );
    }

    // Bottom border
    println!(
        "└───┴{}┴{}┴{}┘",
        "─".repeat(name_width + 2),
        "─".repeat(branch_width + 2),
        "─".repeat(link_width + 2),
    );
}

fn print_runs_json(runs: &[WorkflowRun]) {
    let json = serde_json::to_string_pretty(runs).unwrap_or_default();
    println!("{json}");
}
