use super::*;
use crate::gh::client::GithubApi;
use crate::gh::types::{CiStatus, PullRequest};

// Mock implementation
struct MockGithubApi {
    branch: String,
    run_id: Option<u64>,
    failed_jobs: Vec<(u64, String)>,
    logs: String,
    pr_for_branch: Option<u64>,
}

impl GithubApi for MockGithubApi {
    async fn list_user_prs(&self) -> Result<Vec<PullRequest>> {
        Ok(vec![])
    }

    async fn get_ci_status(&self, _owner: &str, _repo: &str, _pr: u64) -> Result<CiStatus> {
        Ok(CiStatus::Unknown)
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
        Ok(self.pr_for_branch)
    }

    async fn list_workflow_runs(
        &self,
        _query: &crate::gh::types::RunsQuery<'_>,
    ) -> Result<Vec<crate::gh::types::WorkflowRun>> {
        Ok(vec![])
    }

    async fn search_prs_by_title(
        &self,
        _owner: &str,
        _repo: &str,
        _query: &str,
    ) -> Result<Vec<PullRequest>> {
        Ok(vec![])
    }
}

fn query(owner: &str, repo: &str) -> FixQuery {
    FixQuery {
        owner: owner.to_string(),
        repo: repo.to_string(),
        pr: None,
        run: None,
        branch: None,
    }
}

// build_fix_report tests
#[tokio::test]
async fn build_fix_report_no_runs() {
    let mock = MockGithubApi {
        branch: "main".to_string(),
        run_id: None,
        failed_jobs: vec![],
        logs: String::new(),
        pr_for_branch: None,
    };
    let mut q = query("owner", "repo");
    q.pr = Some(42);
    let result = build_fix_report(&mock, &q).await;
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn build_fix_report_no_failed_jobs() {
    let mock = MockGithubApi {
        branch: "main".to_string(),
        run_id: Some(100),
        failed_jobs: vec![],
        logs: String::new(),
        pr_for_branch: None,
    };
    let mut q = query("owner", "repo");
    q.pr = Some(42);
    let result = build_fix_report(&mock, &q).await;
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn build_fix_report_no_test_jobs() {
    let mock = MockGithubApi {
        branch: "main".to_string(),
        run_id: Some(100),
        failed_jobs: vec![(1, "build".to_string()), (2, "deploy".to_string())],
        logs: String::new(),
        pr_for_branch: None,
    };
    let mut q = query("owner", "repo");
    q.pr = Some(42);
    let result = build_fix_report(&mock, &q).await;
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn build_fix_report_with_failures() {
    let mock = MockGithubApi {
        branch: "feature".to_string(),
        run_id: Some(100),
        failed_jobs: vec![(1, "rspec-tests".to_string())],
        logs: r#"
Failures:

  1) User model validates name
     Failure/Error: expect(user).to be_valid
       expected true, got false

Failed examples:

rspec ./spec/models/user_spec.rb:10 # User model validates name
"#
        .to_string(),
        pr_for_branch: Some(42),
    };

    let mut q = query("owner", "repo");
    q.pr = Some(42);
    let result = build_fix_report(&mock, &q).await.unwrap();

    assert!(result.is_some());
    let report = result.unwrap();
    assert_eq!(report.repository, "owner/repo");
    assert_eq!(report.pr_number, Some(42));
    assert_eq!(report.run_id, 100);
    assert_eq!(report.failures.len(), 1);
    assert_eq!(report.failures[0].test_file, "./spec/models/user_spec.rb");
    assert_eq!(report.failures[0].language, "ruby");
    assert!(!report.failures[0].source_files.is_empty());
}

#[tokio::test]
async fn build_fix_report_with_run_id() {
    let mock = MockGithubApi {
        branch: "main".to_string(),
        run_id: Some(200),
        failed_jobs: vec![(1, "test-suite".to_string())],
        logs: r#"
Failures:

  1) Test fails
     Failure/Error: fail
       err

Failed examples:

rspec ./spec/test_spec.rb:5 # Test fails
"#
        .to_string(),
        pr_for_branch: None,
    };

    let mut q = query("owner", "repo");
    q.run = Some(200);
    let result = build_fix_report(&mock, &q).await.unwrap();

    assert!(result.is_some());
    let report = result.unwrap();
    assert_eq!(report.run_id, 200);
    assert!(report.pr_number.is_none());
}

#[tokio::test]
async fn build_fix_report_with_branch() {
    let mock = MockGithubApi {
        branch: "feature-x".to_string(),
        run_id: Some(300),
        failed_jobs: vec![(1, "rspec".to_string())],
        logs: r#"
Failures:

  1) Fail
     Failure/Error: x
       y

Failed examples:

rspec ./spec/x_spec.rb:1 # Fail
"#
        .to_string(),
        pr_for_branch: Some(99),
    };

    let mut q = query("o", "r");
    q.branch = Some("feature-x".to_string());
    let result = build_fix_report(&mock, &q).await.unwrap();

    assert!(result.is_some());
    let report = result.unwrap();
    assert_eq!(report.pr_number, Some(99));
}

#[tokio::test]
async fn build_fix_report_empty_logs() {
    let mock = MockGithubApi {
        branch: "main".to_string(),
        run_id: Some(100),
        failed_jobs: vec![(1, "test".to_string())],
        logs: String::new(),
        pr_for_branch: None,
    };
    let mut q = query("o", "r");
    q.pr = Some(1);
    let result = build_fix_report(&mock, &q).await.unwrap();
    assert!(result.is_none());
}

// FixQuery tests
#[test]
fn fix_query_debug() {
    let q = query("owner", "repo");
    let d = format!("{:?}", q);
    assert!(d.contains("FixQuery"));
}

#[test]
fn fix_query_clone() {
    let q = query("owner", "repo");
    let c = q.clone();
    assert_eq!(c.owner, q.owner);
    assert_eq!(c.repo, q.repo);
}

// Mock that errors on get_job_logs
struct MockGithubApiWithLogError {
    branch: String,
    run_id: Option<u64>,
    failed_jobs: Vec<(u64, String)>,
    pr_for_branch: Option<u64>,
}

impl GithubApi for MockGithubApiWithLogError {
    async fn list_user_prs(&self) -> Result<Vec<PullRequest>> {
        Ok(vec![])
    }

    async fn get_ci_status(&self, _owner: &str, _repo: &str, _pr: u64) -> Result<CiStatus> {
        Ok(CiStatus::Unknown)
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
        Ok(self.pr_for_branch)
    }

    async fn list_workflow_runs(
        &self,
        _query: &crate::gh::types::RunsQuery<'_>,
    ) -> Result<Vec<crate::gh::types::WorkflowRun>> {
        Ok(vec![])
    }

    async fn search_prs_by_title(
        &self,
        _owner: &str,
        _repo: &str,
        _query: &str,
    ) -> Result<Vec<PullRequest>> {
        Ok(vec![])
    }
}

#[tokio::test]
async fn build_fix_report_handles_log_error() {
    let mock = MockGithubApiWithLogError {
        branch: "main".to_string(),
        run_id: Some(100),
        failed_jobs: vec![(1, "rspec-tests".to_string())],
        pr_for_branch: None,
    };
    let mut q = query("o", "r");
    q.pr = Some(1);
    let result = build_fix_report(&mock, &q).await.unwrap();
    // Logs failed, so no failures extracted -> None
    assert!(result.is_none());
}

#[tokio::test]
async fn build_fix_report_uses_current_branch() {
    // No pr, no run, no branch -> uses get_current_branch()
    let mock = MockGithubApi {
        branch: "main".to_string(),
        run_id: None,
        failed_jobs: vec![],
        logs: String::new(),
        pr_for_branch: None,
    };
    let q = query("o", "r"); // no pr, run, or branch set
    let result = build_fix_report(&mock, &q).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn build_fix_report_branch_no_runs() {
    let mock = MockGithubApi {
        branch: "main".to_string(),
        run_id: None,
        failed_jobs: vec![],
        logs: String::new(),
        pr_for_branch: None,
    };
    let mut q = query("o", "r");
    q.branch = Some("feature".to_string());
    let result = build_fix_report(&mock, &q).await.unwrap();
    assert!(result.is_none());
}

// enrich_failures tests
#[test]
fn enrich_failures_ruby() {
    let failures = vec![TestFailure {
        spec_file: "./spec/models/user_spec.rb:10".to_string(),
        failure_text: "expected true".to_string(),
    }];
    let enriched = enrich_failures(&failures);
    assert_eq!(enriched.len(), 1);
    assert_eq!(enriched[0].language, "ruby");
    assert_eq!(enriched[0].test_file, "./spec/models/user_spec.rb");
    assert!(enriched[0]
        .source_files
        .contains(&"app/models/user.rb".to_string()));
}

#[test]
fn enrich_failures_mixed_languages() {
    let failures = vec![
        TestFailure {
            spec_file: "spec/user_spec.rb:5".to_string(),
            failure_text: "ruby error".to_string(),
        },
        TestFailure {
            spec_file: "tests/test_sync.rs".to_string(),
            failure_text: "rust error".to_string(),
        },
        TestFailure {
            spec_file: "Button.test.tsx".to_string(),
            failure_text: "js error".to_string(),
        },
    ];
    let enriched = enrich_failures(&failures);
    assert_eq!(enriched.len(), 3);
    assert_eq!(enriched[0].language, "ruby");
    assert_eq!(enriched[1].language, "rust");
    assert_eq!(enriched[2].language, "javascript");
}

#[test]
fn enrich_failures_empty() {
    let enriched = enrich_failures(&[]);
    assert!(enriched.is_empty());
}

#[test]
fn enrich_failures_unknown_language() {
    let failures = vec![TestFailure {
        spec_file: "README.md".to_string(),
        failure_text: "error".to_string(),
    }];
    let enriched = enrich_failures(&failures);
    assert_eq!(enriched[0].language, "unknown");
    assert!(enriched[0].source_files.is_empty());
}

// format_markdown tests
#[test]
fn format_markdown_basic() {
    let report = FixReport {
        repository: "owner/repo".to_string(),
        pr_number: Some(42),
        run_id: 100,
        failures: vec![FixFailure {
            test_file: "spec/models/user_spec.rb".to_string(),
            source_files: vec!["app/models/user.rb".to_string()],
            failure_text: "expected true".to_string(),
            language: "ruby".to_string(),
        }],
        test_files: vec!["spec/models/user_spec.rb".to_string()],
        source_files: vec!["app/models/user.rb".to_string()],
    };

    let md = format_markdown(&report);
    assert!(md.contains("# Fix Report: owner/repo"));
    assert!(md.contains("**PR:** #42"));
    assert!(md.contains("**Run:** 100"));
    assert!(md.contains("## spec/models/user_spec.rb"));
    assert!(md.contains("**Language:** ruby"));
    assert!(md.contains("`app/models/user.rb`"));
    assert!(md.contains("expected true"));
    assert!(md.contains("bundle exec rspec"));
}

#[test]
fn format_markdown_no_pr() {
    let report = FixReport {
        repository: "o/r".to_string(),
        pr_number: None,
        run_id: 1,
        failures: vec![],
        test_files: vec![],
        source_files: vec![],
    };

    let md = format_markdown(&report);
    assert!(!md.contains("**PR:**"));
    assert!(md.contains("**Run:** 1"));
}

#[test]
fn format_markdown_multiple_failures() {
    let report = FixReport {
        repository: "o/r".to_string(),
        pr_number: Some(1),
        run_id: 1,
        failures: vec![
            FixFailure {
                test_file: "spec/a_spec.rb".to_string(),
                source_files: vec!["app/a.rb".to_string()],
                failure_text: "err1".to_string(),
                language: "ruby".to_string(),
            },
            FixFailure {
                test_file: "tests/test_b.rs".to_string(),
                source_files: vec!["src/b.rs".to_string()],
                failure_text: "err2".to_string(),
                language: "rust".to_string(),
            },
        ],
        test_files: vec!["spec/a_spec.rb".to_string(), "tests/test_b.rs".to_string()],
        source_files: vec!["app/a.rb".to_string(), "src/b.rs".to_string()],
    };

    let md = format_markdown(&report);
    assert!(md.contains("## spec/a_spec.rb"));
    assert!(md.contains("## tests/test_b.rs"));
    assert!(md.contains("bundle exec rspec"));
    assert!(md.contains("cargo test"));
}

// format_rerun_commands tests
#[test]
fn format_rerun_commands_ruby() {
    let failures = vec![FixFailure {
        test_file: "spec/user_spec.rb".to_string(),
        source_files: vec![],
        failure_text: String::new(),
        language: "ruby".to_string(),
    }];
    let cmds = format_rerun_commands(&failures);
    assert!(cmds.contains("bundle exec rspec spec/user_spec.rb"));
}

#[test]
fn format_rerun_commands_rust() {
    let failures = vec![FixFailure {
        test_file: "tests/test_sync.rs".to_string(),
        source_files: vec![],
        failure_text: String::new(),
        language: "rust".to_string(),
    }];
    let cmds = format_rerun_commands(&failures);
    assert!(cmds.contains("cargo test tests/test_sync.rs"));
}

#[test]
fn format_rerun_commands_python() {
    let failures = vec![FixFailure {
        test_file: "tests/test_utils.py".to_string(),
        source_files: vec![],
        failure_text: String::new(),
        language: "python".to_string(),
    }];
    let cmds = format_rerun_commands(&failures);
    assert!(cmds.contains("pytest tests/test_utils.py"));
}

#[test]
fn format_rerun_commands_javascript() {
    let failures = vec![FixFailure {
        test_file: "Button.test.tsx".to_string(),
        source_files: vec![],
        failure_text: String::new(),
        language: "javascript".to_string(),
    }];
    let cmds = format_rerun_commands(&failures);
    assert!(cmds.contains("npx jest Button.test.tsx"));
}

#[test]
fn format_rerun_commands_unknown() {
    let failures = vec![FixFailure {
        test_file: "foo.go".to_string(),
        source_files: vec![],
        failure_text: String::new(),
        language: "unknown".to_string(),
    }];
    let cmds = format_rerun_commands(&failures);
    assert!(cmds.contains("# run foo.go"));
}

#[test]
fn format_rerun_commands_empty() {
    let cmds = format_rerun_commands(&[]);
    assert!(cmds.is_empty());
}

#[test]
fn format_rerun_commands_mixed() {
    let failures = vec![
        FixFailure {
            test_file: "spec/a_spec.rb".to_string(),
            source_files: vec![],
            failure_text: String::new(),
            language: "ruby".to_string(),
        },
        FixFailure {
            test_file: "app.test.js".to_string(),
            source_files: vec![],
            failure_text: String::new(),
            language: "javascript".to_string(),
        },
    ];
    let cmds = format_rerun_commands(&failures);
    assert!(cmds.contains("bundle exec rspec"));
    assert!(cmds.contains("npx jest"));
}

// JSON output test
#[test]
fn fix_report_json_output() {
    let report = FixReport {
        repository: "owner/repo".to_string(),
        pr_number: Some(42),
        run_id: 100,
        failures: vec![FixFailure {
            test_file: "spec/user_spec.rb".to_string(),
            source_files: vec!["app/user.rb".to_string()],
            failure_text: "error".to_string(),
            language: "ruby".to_string(),
        }],
        test_files: vec!["spec/user_spec.rb".to_string()],
        source_files: vec!["app/user.rb".to_string()],
    };

    let json = serde_json::to_string_pretty(&report).unwrap();
    assert!(json.contains("\"repository\": \"owner/repo\""));
    assert!(json.contains("\"pr_number\": 42"));
    assert!(json.contains("\"run_id\": 100"));
    assert!(json.contains("\"test_file\": \"spec/user_spec.rb\""));
    assert!(json.contains("\"language\": \"ruby\""));
}

// output_report tests
#[test]
fn output_report_json() {
    let report = FixReport {
        repository: "o/r".to_string(),
        pr_number: None,
        run_id: 1,
        failures: vec![],
        test_files: vec![],
        source_files: vec![],
    };
    let result = output_report(&report, true);
    assert!(result.is_ok());
}

#[test]
fn output_report_markdown() {
    let report = FixReport {
        repository: "o/r".to_string(),
        pr_number: None,
        run_id: 1,
        failures: vec![],
        test_files: vec![],
        source_files: vec![],
    };
    let result = output_report(&report, false);
    assert!(result.is_ok());
}

// format_markdown edge cases
#[test]
fn format_markdown_no_source_files() {
    let report = FixReport {
        repository: "o/r".to_string(),
        pr_number: None,
        run_id: 1,
        failures: vec![FixFailure {
            test_file: "README.md".to_string(),
            source_files: vec![],
            failure_text: "err".to_string(),
            language: "unknown".to_string(),
        }],
        test_files: vec!["README.md".to_string()],
        source_files: vec![],
    };

    let md = format_markdown(&report);
    assert!(!md.contains("Source Files to Investigate"));
    assert!(!md.contains("**Source files:**"));
}
