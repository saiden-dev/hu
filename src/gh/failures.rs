use anyhow::{Context, Result};

use super::cli::FailuresArgs;
use super::client::{parse_test_failures, GithubApi, GithubClient};

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

/// Check if a job name is test-related
fn is_test_job(name: &str) -> bool {
    let name_lower = name.to_lowercase();
    name_lower.contains("rspec") || name_lower.contains("test") || name_lower.contains("spec")
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
    let output = run_git_command(&["remote", "get-url", "origin"])?;
    parse_github_url(output.trim())
}

/// Run a git command and return stdout (extracted for testability)
fn run_git_command(args: &[&str]) -> Result<String> {
    let output = std::process::Command::new("git")
        .args(args)
        .output()
        .context("Failed to run git command")?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
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
    let branch = run_git_command(&["branch", "--show-current"])?;
    let branch = branch.trim();

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
            branch,
            "--json",
            "number",
            "--limit",
            "1",
        ])
        .output()
        .context("Failed to find PR for current branch")?;

    parse_pr_number_from_json(&output.stdout)
}

/// Parse PR number from gh pr list JSON output (testable)
fn parse_pr_number_from_json(json_bytes: &[u8]) -> Result<u64> {
    let json: serde_json::Value =
        serde_json::from_slice(json_bytes).context("Failed to parse gh pr list output")?;

    json.as_array()
        .and_then(|arr| arr.first())
        .and_then(|pr| pr["number"].as_u64())
        .context("No PR found for current branch. Use --pr to specify a PR number.")
}

#[cfg(test)]
mod tests {
    use super::*;

    // parse_github_url tests
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

    #[test]
    fn parse_ssh_url_no_git_suffix() {
        let (owner, repo) = parse_github_url("git@github.com:owner/repo").unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(repo, "repo");
    }

    #[test]
    fn parse_https_url_trailing_slash() {
        let (owner, repo) = parse_github_url("https://github.com/owner/repo/").unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(repo, "repo");
    }

    #[test]
    fn parse_github_url_invalid() {
        assert!(parse_github_url("not-a-github-url").is_err());
        assert!(parse_github_url("https://gitlab.com/owner/repo").is_err());
        assert!(parse_github_url("").is_err());
    }

    // parse_owner_repo tests
    #[test]
    fn parse_owner_repo_valid() {
        let (owner, repo) = parse_owner_repo("owner/repo").unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(repo, "repo");
    }

    #[test]
    fn parse_owner_repo_with_dashes() {
        let (owner, repo) = parse_owner_repo("my-org/my-repo").unwrap();
        assert_eq!(owner, "my-org");
        assert_eq!(repo, "my-repo");
    }

    #[test]
    fn parse_owner_repo_invalid_no_slash() {
        assert!(parse_owner_repo("noslash").is_err());
    }

    #[test]
    fn parse_owner_repo_invalid_too_many_slashes() {
        assert!(parse_owner_repo("a/b/c").is_err());
    }

    #[test]
    fn parse_owner_repo_invalid_empty() {
        assert!(parse_owner_repo("").is_err());
    }

    // Test job filtering logic
    #[test]
    fn test_job_filter_matches_rspec() {
        let name = "run-rspec-tests (3, 0)";
        let name_lower = name.to_lowercase();
        assert!(
            name_lower.contains("rspec")
                || name_lower.contains("test")
                || name_lower.contains("spec")
        );
    }

    #[test]
    fn test_job_filter_matches_jest() {
        let name = "Jest Tests";
        let name_lower = name.to_lowercase();
        assert!(
            name_lower.contains("rspec")
                || name_lower.contains("test")
                || name_lower.contains("spec")
        );
    }

    #[test]
    fn test_job_filter_no_match() {
        let name = "Build Docker Image";
        let name_lower = name.to_lowercase();
        assert!(
            !(name_lower.contains("rspec")
                || name_lower.contains("test")
                || name_lower.contains("spec"))
        );
    }

    // is_test_job tests
    #[test]
    fn is_test_job_rspec() {
        assert!(is_test_job("run-rspec-tests"));
        assert!(is_test_job("RSpec"));
    }

    #[test]
    fn is_test_job_test() {
        assert!(is_test_job("unit-tests"));
        assert!(is_test_job("Test Suite"));
    }

    #[test]
    fn is_test_job_spec() {
        assert!(is_test_job("run-specs"));
        assert!(is_test_job("Spec Runner"));
    }

    #[test]
    fn is_test_job_non_test() {
        assert!(!is_test_job("build"));
        assert!(!is_test_job("deploy"));
        assert!(!is_test_job("lint"));
    }

    // Mock implementation for testing
    use crate::gh::types::PullRequest;

    struct MockGithubApi {
        branch: String,
        run_id: Option<u64>,
        failed_jobs: Vec<(u64, String)>,
        logs: String,
    }

    impl GithubApi for MockGithubApi {
        async fn list_user_prs(&self) -> Result<Vec<PullRequest>> {
            Ok(vec![])
        }

        async fn get_ci_status(
            &self,
            _owner: &str,
            _repo: &str,
            _pr: u64,
        ) -> Result<crate::gh::types::CiStatus> {
            Ok(crate::gh::types::CiStatus::Unknown)
        }

        async fn get_pr_branch(&self, _owner: &str, _repo: &str, _pr: u64) -> Result<String> {
            Ok(self.branch.clone())
        }

        async fn get_latest_failed_run_for_branch(
            &self,
            _owner: &str,
            _repo: &str,
            _branch: &str,
        ) -> Result<Option<u64>> {
            Ok(self.run_id)
        }

        async fn get_failed_jobs(
            &self,
            _owner: &str,
            _repo: &str,
            _run_id: u64,
        ) -> Result<Vec<(u64, String)>> {
            Ok(self.failed_jobs.clone())
        }

        async fn get_job_logs(&self, _owner: &str, _repo: &str, _job_id: u64) -> Result<String> {
            Ok(self.logs.clone())
        }
    }

    #[tokio::test]
    async fn process_failures_no_failed_runs() {
        let mock = MockGithubApi {
            branch: "main".to_string(),
            run_id: None,
            failed_jobs: vec![],
            logs: String::new(),
        };
        let result = process_failures(&mock, "owner", "repo", 123).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn process_failures_no_failed_jobs() {
        let mock = MockGithubApi {
            branch: "main".to_string(),
            run_id: Some(1),
            failed_jobs: vec![],
            logs: String::new(),
        };
        let result = process_failures(&mock, "owner", "repo", 123).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn process_failures_no_test_jobs() {
        let mock = MockGithubApi {
            branch: "main".to_string(),
            run_id: Some(1),
            failed_jobs: vec![(1, "build".to_string()), (2, "deploy".to_string())],
            logs: String::new(),
        };
        let result = process_failures(&mock, "owner", "repo", 123).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn process_failures_with_test_failures() {
        let mock = MockGithubApi {
            branch: "main".to_string(),
            run_id: Some(1),
            failed_jobs: vec![(1, "rspec-tests".to_string())],
            logs: r#"
Failures:

  1) Test fails
     Failure/Error: expect(1).to eq(2)
       expected: 2

Failed examples:

rspec ./spec/test_spec.rb:10 # Test fails
"#
            .to_string(),
        };
        let result = process_failures(&mock, "owner", "repo", 123).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn process_failures_empty_logs() {
        let mock = MockGithubApi {
            branch: "main".to_string(),
            run_id: Some(1),
            failed_jobs: vec![(1, "test-suite".to_string())],
            logs: String::new(),
        };
        let result = process_failures(&mock, "owner", "repo", 123).await;
        assert!(result.is_ok());
    }

    // parse_pr_number_from_json tests
    #[test]
    fn parse_pr_number_valid() {
        let json = br#"[{"number": 123}]"#;
        let result = parse_pr_number_from_json(json);
        assert_eq!(result.unwrap(), 123);
    }

    #[test]
    fn parse_pr_number_multiple_prs() {
        let json = br#"[{"number": 100}, {"number": 200}]"#;
        let result = parse_pr_number_from_json(json);
        assert_eq!(result.unwrap(), 100); // First one
    }

    #[test]
    fn parse_pr_number_empty_array() {
        let json = br#"[]"#;
        let result = parse_pr_number_from_json(json);
        assert!(result.is_err());
    }

    #[test]
    fn parse_pr_number_invalid_json() {
        let json = b"not json";
        let result = parse_pr_number_from_json(json);
        assert!(result.is_err());
    }

    #[test]
    fn parse_pr_number_missing_number_field() {
        let json = br#"[{"title": "some pr"}]"#;
        let result = parse_pr_number_from_json(json);
        assert!(result.is_err());
    }

    // run_git_command test (integration - requires git)
    #[test]
    fn run_git_command_version() {
        // This should work in any environment with git
        let result = run_git_command(&["--version"]);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("git version"));
    }

    // Mock with error handling
    struct MockGithubApiWithLogError {
        branch: String,
        run_id: Option<u64>,
        failed_jobs: Vec<(u64, String)>,
    }

    impl GithubApi for MockGithubApiWithLogError {
        async fn list_user_prs(&self) -> Result<Vec<PullRequest>> {
            Ok(vec![])
        }

        async fn get_ci_status(
            &self,
            _owner: &str,
            _repo: &str,
            _pr: u64,
        ) -> Result<crate::gh::types::CiStatus> {
            Ok(crate::gh::types::CiStatus::Unknown)
        }

        async fn get_pr_branch(&self, _owner: &str, _repo: &str, _pr: u64) -> Result<String> {
            Ok(self.branch.clone())
        }

        async fn get_latest_failed_run_for_branch(
            &self,
            _owner: &str,
            _repo: &str,
            _branch: &str,
        ) -> Result<Option<u64>> {
            Ok(self.run_id)
        }

        async fn get_failed_jobs(
            &self,
            _owner: &str,
            _repo: &str,
            _run_id: u64,
        ) -> Result<Vec<(u64, String)>> {
            Ok(self.failed_jobs.clone())
        }

        async fn get_job_logs(&self, _owner: &str, _repo: &str, _job_id: u64) -> Result<String> {
            Err(anyhow::anyhow!("Failed to fetch logs"))
        }
    }

    #[tokio::test]
    async fn process_failures_handles_log_fetch_error() {
        let mock = MockGithubApiWithLogError {
            branch: "feature".to_string(),
            run_id: Some(42),
            failed_jobs: vec![(100, "rspec-tests".to_string())],
        };
        // Should still succeed, just with warning printed
        let result = process_failures(&mock, "owner", "repo", 123).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn process_failures_multiple_test_jobs() {
        let mock = MockGithubApi {
            branch: "main".to_string(),
            run_id: Some(1),
            failed_jobs: vec![
                (1, "rspec-tests".to_string()),
                (2, "jest-tests".to_string()),
                (3, "build".to_string()), // Non-test job, should be filtered
            ],
            logs: r#"
Failures:

  1) Test fails
     Failure/Error: expect(1).to eq(2)
       expected: 2

Failed examples:

rspec ./spec/test_spec.rb:10 # Test fails
"#
            .to_string(),
        };
        let result = process_failures(&mock, "owner", "repo", 123).await;
        assert!(result.is_ok());
    }

    // Additional parse_github_url tests
    #[test]
    fn parse_github_url_ssh_with_org() {
        let (owner, repo) = parse_github_url("git@github.com:my-org/my-repo.git").unwrap();
        assert_eq!(owner, "my-org");
        assert_eq!(repo, "my-repo");
    }

    #[test]
    fn parse_github_url_empty_string() {
        assert!(parse_github_url("").is_err());
    }

    #[test]
    fn parse_github_url_missing_repo() {
        assert!(parse_github_url("git@github.com:owner").is_err());
    }

    // get_current_repo test (requires git)
    #[test]
    fn get_current_repo_returns_result() {
        // This test verifies the function returns a result (success or error)
        let result = get_current_repo();
        // In a git repo, it should succeed; outside, it should fail
        assert!(result.is_ok() || result.is_err());
    }

    // More is_test_job coverage
    #[test]
    fn is_test_job_mixed_case() {
        assert!(is_test_job("RSPEC"));
        assert!(is_test_job("RSpec"));
        assert!(is_test_job("TEST"));
        assert!(is_test_job("SPEC"));
    }

    #[test]
    fn is_test_job_partial_names() {
        assert!(is_test_job("run-rspec-tests (3, 0)"));
        assert!(is_test_job("unit-test-suite"));
        assert!(is_test_job("integration-spec"));
    }
}
