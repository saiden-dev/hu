use anyhow::{Context, Result};
use octocrab::Octocrab;

use super::auth::get_token;
use super::types::{CiStatus, PullRequest};

mod parsing;

#[cfg(test)]
use parsing::clean_ci_line;
pub use parsing::parse_test_failures;

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

    /// Find PR number for a branch
    fn find_pr_for_branch(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
    ) -> impl std::future::Future<Output = Result<Option<u64>>> + Send;
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

/// Extract PR number from pull request list response (pure function, testable)
pub fn extract_pr_number_from_list(prs: &serde_json::Value) -> Option<u64> {
    prs.as_array()
        .and_then(|arr| arr.first())
        .and_then(|pr| pr["number"].as_u64())
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

    async fn find_pr_for_branch(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
    ) -> Result<Option<u64>> {
        let prs: serde_json::Value = self
            .client
            .get(
                format!(
                    "/repos/{}/{}/pulls?head={}:{}&state=open&per_page=1",
                    owner, repo, owner, branch
                ),
                None::<&()>,
            )
            .await
            .context("Failed to search for PR by branch")?;

        Ok(extract_pr_number_from_list(&prs))
    }
}
