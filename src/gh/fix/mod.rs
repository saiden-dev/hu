use anyhow::Result;

use super::cli::FixArgs;
use super::client::{parse_test_failures, GithubApi, GithubClient};
use super::helpers::{get_current_branch, get_current_repo, is_test_job, parse_owner_repo};
use super::types::{FixFailure, FixReport, TestFailure};

mod mapping;

#[cfg(test)]
mod tests;

/// Query parameters for building a fix report
#[derive(Debug, Clone)]
pub struct FixQuery {
    pub owner: String,
    pub repo: String,
    pub pr: Option<u64>,
    pub run: Option<u64>,
    pub branch: Option<String>,
}

/// Handle the `hu gh fix` command
#[cfg(not(tarpaulin_include))]
pub async fn run(args: FixArgs) -> Result<()> {
    let client = GithubClient::new()?;

    let (owner, repo) = if let Some(repo_arg) = &args.repo {
        parse_owner_repo(repo_arg)?
    } else {
        get_current_repo()?
    };

    let query = FixQuery {
        owner,
        repo,
        pr: args.pr,
        run: args.run,
        branch: args.branch,
    };

    let report = build_fix_report(&client, &query).await?;

    match report {
        Some(r) => output_report(&r, args.json),
        None => {
            println!("No failures found.");
            Ok(())
        }
    }
}

/// Build a fix report from CI failures (testable, no I/O except API calls)
pub async fn build_fix_report(
    client: &impl GithubApi,
    query: &FixQuery,
) -> Result<Option<FixReport>> {
    let repository = format!("{}/{}", query.owner, query.repo);
    let owner = &query.owner;
    let repo = &query.repo;

    // Determine run_id from args
    let (run_id, pr_number) = if let Some(run_id) = query.run {
        (run_id, query.pr)
    } else if let Some(pr_number) = query.pr {
        let branch = client.get_pr_branch(owner, repo, pr_number).await?;
        let run_id = client
            .get_latest_failed_run_for_branch(owner, repo, &branch)
            .await?;
        match run_id {
            Some(id) => (id, Some(pr_number)),
            None => return Ok(None),
        }
    } else {
        // Use branch arg or current branch
        let branch_name = match &query.branch {
            Some(b) => b.clone(),
            None => get_current_branch()?,
        };

        let pr_number = client.find_pr_for_branch(owner, repo, &branch_name).await?;

        let run_id = client
            .get_latest_failed_run_for_branch(owner, repo, &branch_name)
            .await?;

        match run_id {
            Some(id) => (id, pr_number),
            None => return Ok(None),
        }
    };

    // Get failed jobs
    let failed_jobs = client.get_failed_jobs(owner, repo, run_id).await?;

    if failed_jobs.is_empty() {
        return Ok(None);
    }

    // Filter to test jobs and fetch logs
    let test_jobs: Vec<_> = failed_jobs
        .into_iter()
        .filter(|(_, name)| is_test_job(name))
        .collect();

    if test_jobs.is_empty() {
        return Ok(None);
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
        return Ok(None);
    }

    let fix_failures = enrich_failures(&all_failures);
    let test_files: Vec<String> = fix_failures.iter().map(|f| f.test_file.clone()).collect();
    let source_files: Vec<String> = fix_failures
        .iter()
        .flat_map(|f| f.source_files.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    Ok(Some(FixReport {
        repository,
        pr_number,
        run_id,
        failures: fix_failures,
        test_files,
        source_files,
    }))
}

/// Enrich test failures with source file mappings (pure function)
pub fn enrich_failures(failures: &[TestFailure]) -> Vec<FixFailure> {
    failures
        .iter()
        .map(|f| {
            let language = mapping::detect_language(&f.spec_file).to_string();
            let source_files = mapping::map_test_to_source(&f.spec_file);
            let test_file = mapping::strip_line_number(&f.spec_file).to_string();

            FixFailure {
                test_file,
                source_files,
                failure_text: f.failure_text.clone(),
                language,
            }
        })
        .collect()
}

/// Output the fix report (markdown or JSON)
fn output_report(report: &FixReport, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(report)?);
    } else {
        print!("{}", format_markdown(report));
    }
    Ok(())
}

/// Format report as markdown (pure function, testable)
pub fn format_markdown(report: &FixReport) -> String {
    let mut out = String::new();

    out.push_str(&format!("# Fix Report: {}\n\n", report.repository));

    if let Some(pr) = report.pr_number {
        out.push_str(&format!("**PR:** #{}\n", pr));
    }
    out.push_str(&format!("**Run:** {}\n", report.run_id));
    out.push_str(&format!("**Failures:** {}\n\n", report.failures.len()));

    // Failures
    for failure in &report.failures {
        out.push_str(&format!("## {}\n\n", failure.test_file));
        out.push_str(&format!("**Language:** {}\n", failure.language));

        if !failure.source_files.is_empty() {
            out.push_str("**Source files:**\n");
            for sf in &failure.source_files {
                out.push_str(&format!("- `{}`\n", sf));
            }
        }

        out.push_str("\n```\n");
        out.push_str(&failure.failure_text);
        out.push_str("\n```\n\n");
    }

    // Rerun commands
    out.push_str(&format_rerun_commands(&report.failures));

    // File lists
    if !report.source_files.is_empty() {
        out.push_str("## Source Files to Investigate\n\n");
        for f in &report.source_files {
            out.push_str(&format!("- `{}`\n", f));
        }
        out.push('\n');
    }

    out
}

/// Format rerun commands section (pure function, testable)
pub fn format_rerun_commands(failures: &[FixFailure]) -> String {
    if failures.is_empty() {
        return String::new();
    }

    let mut out = String::from("## Rerun Commands\n\n```bash\n");

    for failure in failures {
        match failure.language.as_str() {
            "ruby" => out.push_str(&format!("bundle exec rspec {}\n", failure.test_file)),
            "rust" => out.push_str(&format!("cargo test {}\n", failure.test_file)),
            "python" => out.push_str(&format!("pytest {}\n", failure.test_file)),
            "javascript" => out.push_str(&format!("npx jest {}\n", failure.test_file)),
            _ => out.push_str(&format!("# run {}\n", failure.test_file)),
        }
    }

    out.push_str("```\n\n");
    out
}
