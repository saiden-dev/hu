use anyhow::Result;

use super::cli::FailuresArgs;
use super::client::{parse_test_failures, GithubApi, GithubClient};
use super::helpers::{get_current_repo, is_test_job, parse_owner_repo};

#[cfg(test)]
mod tests;

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
        get_current_branch_pr(&client, &owner, &repo).await?
    };

    process_failures(&client, &owner, &repo, pr_number).await
}

/// Process failures using the given API client (testable)
pub async fn process_failures(
    client: &impl GithubApi,
    owner: &str,
    repo: &str,
    pr_number: u64,
) -> Result<()> {
    eprintln!(
        "Fetching failures for PR #{} in {}/{}...",
        pr_number, owner, repo
    );

    // Get the PR's branch name
    let branch = client.get_pr_branch(owner, repo, pr_number).await?;

    // Get the latest failed workflow run for this branch
    let run_id = client
        .get_latest_failed_run_for_branch(owner, repo, &branch)
        .await?;

    let run_id = match run_id {
        Some(id) => id,
        None => {
            println!("No failed workflow runs found for PR #{}.", pr_number);
            return Ok(());
        }
    };

    // Get failed jobs in that run
    let failed_jobs = client.get_failed_jobs(owner, repo, run_id).await?;

    if failed_jobs.is_empty() {
        println!("No failed jobs found in run {}.", run_id);
        return Ok(());
    }

    // Only process test-related jobs (rspec, jest, etc.)
    let test_jobs: Vec<_> = failed_jobs
        .into_iter()
        .filter(|(_, name)| is_test_job(name))
        .collect();

    if test_jobs.is_empty() {
        println!("No test-related job failures found.");
        return Ok(());
    }

    let mut all_failures = Vec::new();

    for (job_id, job_name) in test_jobs {
        eprintln!("Fetching logs for job: {}", job_name);

        match client.get_job_logs(owner, repo, job_id).await {
            Ok(logs) => {
                let failures = parse_test_failures(&logs);
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

/// Get PR number for current branch using octocrab
async fn get_current_branch_pr(client: &impl GithubApi, owner: &str, repo: &str) -> Result<u64> {
    let branch = super::helpers::get_current_branch()?;

    match client.find_pr_for_branch(owner, repo, &branch).await? {
        Some(pr) => Ok(pr),
        None => anyhow::bail!(
            "No PR found for branch '{}'. Use --pr to specify a PR number.",
            branch
        ),
    }
}
