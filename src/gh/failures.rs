use anyhow::{Context, Result};

use super::cli::FailuresArgs;
use super::client::GithubClient;

/// Handle the `hu gh failures` command
pub async fn run(args: FailuresArgs) -> Result<()> {
    let client = GithubClient::new()?;

    // Get repo info from args or current directory
    let (owner, repo) = if let Some(repo_arg) = &args.repo {
        parse_owner_repo(repo_arg)?
    } else {
        get_current_repo()?
    };

    // Determine which PR to check
    let pr_number = if let Some(pr) = args.pr {
        pr
    } else {
        get_current_branch_pr(&owner, &repo).await?
    };

    eprintln!(
        "Fetching failures for PR #{} in {}/{}...",
        pr_number, owner, repo
    );

    // Get the PR's branch name
    let branch = client.get_pr_branch(&owner, &repo, pr_number).await?;

    // Get the latest failed workflow run for this branch
    let run_id = client
        .get_latest_failed_run_for_branch(&owner, &repo, &branch)
        .await?;

    let run_id = match run_id {
        Some(id) => id,
        None => {
            println!("No failed workflow runs found for PR #{}.", pr_number);
            return Ok(());
        }
    };

    // Get failed jobs in that run
    let failed_jobs = client.get_failed_jobs(&owner, &repo, run_id).await?;

    if failed_jobs.is_empty() {
        println!("No failed jobs found in run {}.", run_id);
        return Ok(());
    }

    // Only process test-related jobs (rspec, jest, etc.)
    let test_jobs: Vec<_> = failed_jobs
        .into_iter()
        .filter(|(_, name)| {
            let name_lower = name.to_lowercase();
            name_lower.contains("rspec")
                || name_lower.contains("test")
                || name_lower.contains("spec")
        })
        .collect();

    if test_jobs.is_empty() {
        println!("No test-related job failures found.");
        return Ok(());
    }

    let mut all_failures = Vec::new();

    for (job_id, job_name) in test_jobs {
        eprintln!("Fetching logs for job: {}", job_name);

        match client.get_job_logs(&owner, &repo, job_id).await {
            Ok(logs) => {
                let failures = GithubClient::parse_test_failures(&logs);
                all_failures.extend(failures);
            }
            Err(e) => {
                eprintln!("Warning: Failed to fetch logs for {}: {}", job_name, e);
            }
        }
    }

    if all_failures.is_empty() {
        println!("No test failures found in logs.");
        return Ok(());
    }

    // Output in a format useful for Claude
    println!("\n# Test Failures\n");
    for failure in &all_failures {
        println!("## {}\n", failure.spec_file);
        println!("```");
        println!("{}", failure.failure_text);
        println!("```\n");
    }

    // Also output the rspec commands to rerun
    println!("# Rerun Commands\n");
    println!("```bash");
    for failure in &all_failures {
        println!("bundle exec rspec {}", failure.spec_file);
    }
    println!("```");

    Ok(())
}

/// Parse owner/repo from command line argument
fn parse_owner_repo(repo: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = repo.split('/').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid repo format. Expected owner/repo, got: {}", repo);
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

/// Get owner/repo from git remote
fn get_current_repo() -> Result<(String, String)> {
    let output = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .context("Failed to get git remote")?;

    let url = String::from_utf8_lossy(&output.stdout);
    parse_github_url(url.trim())
}

/// Parse GitHub URL to extract owner/repo
fn parse_github_url(url: &str) -> Result<(String, String)> {
    // Handle SSH: git@github.com:owner/repo.git
    // Handle HTTPS: https://github.com/owner/repo.git
    let url = url.trim_end_matches(".git").trim_end_matches('/');

    if url.contains("github.com:") {
        // SSH format
        let parts: Vec<&str> = url.split(':').collect();
        if let Some(path) = parts.last() {
            let segments: Vec<&str> = path.split('/').collect();
            if segments.len() >= 2 {
                return Ok((
                    segments[segments.len() - 2].to_string(),
                    segments[segments.len() - 1].to_string(),
                ));
            }
        }
    } else if url.contains("github.com/") {
        // HTTPS format
        let parts: Vec<&str> = url.split("github.com/").collect();
        if let Some(path) = parts.last() {
            let segments: Vec<&str> = path.split('/').collect();
            if segments.len() >= 2 {
                return Ok((segments[0].to_string(), segments[1].to_string()));
            }
        }
    }

    anyhow::bail!("Could not parse GitHub URL: {}", url)
}

/// Get PR number for current branch
async fn get_current_branch_pr(owner: &str, repo: &str) -> Result<u64> {
    // Get current branch name
    let output = std::process::Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .context("Failed to get current branch")?;

    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if branch.is_empty() {
        anyhow::bail!("Not on a branch. Use --pr to specify a PR number.");
    }

    // Use gh CLI to find PR for this branch
    let output = std::process::Command::new("gh")
        .args([
            "pr",
            "list",
            "--repo",
            &format!("{}/{}", owner, repo),
            "--head",
            &branch,
            "--json",
            "number",
            "--limit",
            "1",
        ])
        .output()
        .context("Failed to find PR for current branch")?;

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("Failed to parse gh pr list output")?;

    json.as_array()
        .and_then(|arr| arr.first())
        .and_then(|pr| pr["number"].as_u64())
        .context("No PR found for current branch. Use --pr to specify a PR number.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ssh_url() {
        let (owner, repo) = parse_github_url("git@github.com:owner/repo.git").unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(repo, "repo");
    }

    #[test]
    fn parse_https_url() {
        let (owner, repo) = parse_github_url("https://github.com/owner/repo.git").unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(repo, "repo");
    }

    #[test]
    fn parse_https_url_no_git_suffix() {
        let (owner, repo) = parse_github_url("https://github.com/owner/repo").unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(repo, "repo");
    }
}
