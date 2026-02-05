use super::*;

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

    async fn find_pr_for_branch(
        &self,
        _owner: &str,
        _repo: &str,
        _branch: &str,
    ) -> Result<Option<u64>> {
        Ok(None)
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

    async fn find_pr_for_branch(
        &self,
        _owner: &str,
        _repo: &str,
        _branch: &str,
    ) -> Result<Option<u64>> {
        Ok(None)
    }
}

#[tokio::test]
async fn process_failures_handles_log_fetch_error() {
    let mock = MockGithubApiWithLogError {
        branch: "feature".to_string(),
        run_id: Some(42),
        failed_jobs: vec![(100, "rspec-tests".to_string())],
    };
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
            (3, "build".to_string()),
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
