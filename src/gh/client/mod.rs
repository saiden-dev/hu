use anyhow::{Context, Result};
use octocrab::Octocrab;

use super::auth::get_token;
use super::types::{CiStatus, PullRequest, TestFailure};

#[cfg(test)]
mod tests;

/// Trait for GitHub API operations (enables mocking in tests)
pub trait GithubApi: Send + Sync {
    /// List open PRs authored by the current user
    fn list_user_prs(&self) -> impl std::future::Future<Output = Result<Vec<PullRequest>>> + Send;

    /// Get CI status for a PR
    fn get_ci_status(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> impl std::future::Future<Output = Result<CiStatus>> + Send;

    /// Get the branch name for a PR
    fn get_pr_branch(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> impl std::future::Future<Output = Result<String>> + Send;

    /// Get the latest failed workflow run for a branch
    fn get_latest_failed_run_for_branch(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
    ) -> impl std::future::Future<Output = Result<Option<u64>>> + Send;

    /// Get failed jobs for a workflow run
    fn get_failed_jobs(
        &self,
        owner: &str,
        repo: &str,
        run_id: u64,
    ) -> impl std::future::Future<Output = Result<Vec<(u64, String)>>> + Send;

    /// Download logs for a job
    fn get_job_logs(
        &self,
        owner: &str,
        repo: &str,
        job_id: u64,
    ) -> impl std::future::Future<Output = Result<String>> + Send;
}

/// Parse CI status from GitHub API responses (pure function, testable)
pub fn parse_ci_status(state: &str, check_runs: Option<&Vec<serde_json::Value>>) -> CiStatus {
    if let Some(runs) = check_runs {
        if runs.is_empty() && state == "pending" {
            return CiStatus::Pending;
        }

        let any_failed = runs
            .iter()
            .any(|r| r["conclusion"].as_str() == Some("failure"));
        let any_pending = runs.iter().any(|r| {
            r["status"].as_str() != Some("completed") || r["conclusion"].as_str().is_none()
        });
        let all_success = runs
            .iter()
            .all(|r| r["conclusion"].as_str() == Some("success"));

        if any_failed {
            CiStatus::Failed
        } else if any_pending {
            CiStatus::Pending
        } else if all_success && !runs.is_empty() {
            CiStatus::Success
        } else {
            parse_state_string(state)
        }
    } else {
        parse_state_string(state)
    }
}

/// Parse state string to CiStatus
fn parse_state_string(state: &str) -> CiStatus {
    match state {
        "success" => CiStatus::Success,
        "pending" => CiStatus::Pending,
        "failure" | "error" => CiStatus::Failed,
        _ => CiStatus::Unknown,
    }
}

/// Extract failed jobs from GitHub jobs API response (pure function, testable)
pub fn extract_failed_jobs(jobs: &serde_json::Value) -> Vec<(u64, String)> {
    jobs["jobs"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter(|j| j["conclusion"].as_str() == Some("failure"))
        .filter_map(|j| {
            let id = j["id"].as_u64()?;
            let name = j["name"].as_str()?.to_string();
            Some((id, name))
        })
        .collect()
}

/// Extract run ID from workflow runs response (pure function, testable)
pub fn extract_run_id(runs: &serde_json::Value) -> Option<u64> {
    runs["workflow_runs"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|r| r["id"].as_u64())
}

pub struct GithubClient {
    client: Octocrab,
}

impl GithubClient {
    /// Create a new authenticated GitHub client
    pub fn new() -> Result<Self> {
        let token = get_token().context("Not authenticated. Run `hu gh login` first.")?;

        let client = Octocrab::builder()
            .personal_token(token)
            .build()
            .context("Failed to create GitHub client")?;

        Ok(Self { client })
    }

    /// Create client from provided token (for testing)
    #[allow(dead_code)]
    pub fn with_token(token: &str) -> Result<Self> {
        let client = Octocrab::builder()
            .personal_token(token.to_string())
            .build()
            .context("Failed to create GitHub client")?;

        Ok(Self { client })
    }
}

impl GithubApi for GithubClient {
    async fn list_user_prs(&self) -> Result<Vec<PullRequest>> {
        // Use the search API to find PRs where author is current user
        let result = self
            .client
            .search()
            .issues_and_pull_requests("is:pr is:open author:@me")
            .send()
            .await
            .context("Failed to search for PRs")?;

        let prs: Vec<PullRequest> = result
            .items
            .into_iter()
            .filter_map(|issue| {
                // Extract repo from URL: https://api.github.com/repos/owner/repo/issues/123
                let repo_full_name = issue
                    .repository_url
                    .path_segments()?
                    .skip(1) // skip "repos"
                    .take(2) // take "owner" and "repo"
                    .collect::<Vec<_>>()
                    .join("/");

                let state = match issue.state {
                    octocrab::models::IssueState::Open => "open",
                    octocrab::models::IssueState::Closed => "closed",
                    _ => "unknown",
                };

                Some(PullRequest {
                    number: issue.number,
                    title: issue.title,
                    html_url: issue.html_url.to_string(),
                    state: state.to_string(),
                    repo_full_name,
                    created_at: issue.created_at.to_rfc3339(),
                    updated_at: issue.updated_at.to_rfc3339(),
                    ci_status: None,
                })
            })
            .collect();

        Ok(prs)
    }

    async fn get_ci_status(&self, owner: &str, repo: &str, pr_number: u64) -> Result<CiStatus> {
        // Get the PR to find the head SHA
        let pr = self
            .client
            .pulls(owner, repo)
            .get(pr_number)
            .await
            .context("Failed to get PR")?;

        let sha = &pr.head.sha;

        // Get combined status
        let status: serde_json::Value = self
            .client
            .get(
                format!("/repos/{}/{}/commits/{}/status", owner, repo, sha),
                None::<&()>,
            )
            .await
            .context("Failed to get commit status")?;

        let state = status["state"].as_str().unwrap_or("unknown");

        // Also check for check runs (GitHub Actions uses this)
        let checks: serde_json::Value = self
            .client
            .get(
                format!("/repos/{}/{}/commits/{}/check-runs", owner, repo, sha),
                None::<&()>,
            )
            .await
            .unwrap_or_default();

        let check_runs = checks["check_runs"].as_array();

        Ok(parse_ci_status(state, check_runs))
    }

    async fn get_pr_branch(&self, owner: &str, repo: &str, pr_number: u64) -> Result<String> {
        let pr = self
            .client
            .pulls(owner, repo)
            .get(pr_number)
            .await
            .context("Failed to get PR")?;

        Ok(pr.head.ref_field)
    }

    async fn get_latest_failed_run_for_branch(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
    ) -> Result<Option<u64>> {
        let runs: serde_json::Value = self
            .client
            .get(
                format!(
                    "/repos/{}/{}/actions/runs?branch={}&status=failure&per_page=1",
                    owner, repo, branch
                ),
                None::<&()>,
            )
            .await
            .context("Failed to get workflow runs")?;

        Ok(extract_run_id(&runs))
    }

    async fn get_failed_jobs(
        &self,
        owner: &str,
        repo: &str,
        run_id: u64,
    ) -> Result<Vec<(u64, String)>> {
        let jobs: serde_json::Value = self
            .client
            .get(
                format!("/repos/{}/{}/actions/runs/{}/jobs", owner, repo, run_id),
                None::<&()>,
            )
            .await
            .context("Failed to get jobs")?;

        Ok(extract_failed_jobs(&jobs))
    }

    async fn get_job_logs(&self, owner: &str, repo: &str, job_id: u64) -> Result<String> {
        // The logs endpoint returns a redirect to a download URL
        // We need to use reqwest directly for this
        let token = get_token().context("Not authenticated")?;

        let client = reqwest::Client::new();
        let url = format!(
            "https://api.github.com/repos/{}/{}/actions/jobs/{}/logs",
            owner, repo, job_id
        );

        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("User-Agent", "hu-cli")
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .context("Failed to request job logs")?;

        let logs = response.text().await.context("Failed to read job logs")?;

        Ok(logs)
    }
}

/// Extract test failures from logs (RSpec format)
pub fn parse_test_failures(logs: &str) -> Vec<TestFailure> {
    let mut failures = Vec::new();

    // Collect failure error messages in order
    let mut error_messages: Vec<String> = Vec::new();

    // Find the Failures section and parse each failure block
    if let Some(failures_start) = logs.find("Failures:") {
        let failures_end = logs.find("Failed examples:").unwrap_or(logs.len());
        let failures_section = &logs[failures_start..failures_end];

        // Split by numbered failure pattern "N) description"
        let block_starts: Vec<usize> = regex::Regex::new(r"\d+\)\s+\S")
            .ok()
            .map(|re| re.find_iter(failures_section).map(|m| m.start()).collect())
            .unwrap_or_default();

        let mut positions = block_starts.clone();
        positions.push(failures_section.len());

        for i in 0..block_starts.len() {
            let block = &failures_section[positions[i]..positions[i + 1]];

            // Extract error: code line after Failure/Error: and the error message on next line
            if let Some(fe_idx) = block.find("Failure/Error:") {
                let after_fe = &block[fe_idx..];
                let lines: Vec<String> = after_fe
                    .lines()
                    .map(clean_ci_line)
                    .filter(|l| !l.is_empty())
                    .take(4)
                    .collect();

                // lines[0] = "Failure/Error: <code>"
                // lines[1] = "<error message>" or "# <stack trace>"
                let code_line = lines
                    .first()
                    .map(|l| l.strip_prefix("Failure/Error:").unwrap_or(l).trim())
                    .unwrap_or("");
                let error_msg = lines.get(1).map(|s| s.as_str()).unwrap_or("");

                let error_text = if error_msg.is_empty() || error_msg.starts_with("# ") {
                    code_line.to_string()
                } else {
                    format!("{}\n{}", code_line, error_msg)
                };

                error_messages.push(error_text);
            }
        }
    }

    // Extract failed examples from the "Failed examples:" section
    // Format: rspec ./spec/helpers/prices_api_helper_spec.rb:289 # description
    let failed_examples_re = regex::Regex::new(r"rspec\s+(\./spec/[^\s]+:\d+)").ok();

    if let Some(re) = &failed_examples_re {
        for (i, cap) in re.captures_iter(logs).enumerate() {
            let spec_file = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            // Get error message by index (failures appear in same order)
            let failure_text = error_messages
                .get(i)
                .cloned()
                .unwrap_or_else(|| "Test failed".to_string());

            // Avoid duplicates
            if !failures
                .iter()
                .any(|f: &TestFailure| f.spec_file == spec_file)
            {
                failures.push(TestFailure {
                    spec_file: spec_file.to_string(),
                    failure_text,
                });
            }
        }
    }

    failures
}

/// Clean up CI log line by removing timestamp prefix
fn clean_ci_line(line: &str) -> String {
    // Remove timestamp prefix like "2026-01-27T18:51:46.1029380Z"
    let re = regex::Regex::new(r"^\d{4}-\d{2}-\d{2}T[\d:.]+Z\s*").ok();
    if let Some(re) = re {
        re.replace(line, "").trim().to_string()
    } else {
        line.trim().to_string()
    }
}
