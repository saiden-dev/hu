use serde::{Deserialize, Serialize};

/// CI check status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CiStatus {
    Success,
    Pending,
    Failed,
    #[default]
    Unknown,
}

/// Pull request data for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    pub html_url: String,
    pub state: String,
    pub repo_full_name: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip)]
    pub ci_status: Option<CiStatus>,
}

/// A GitHub Actions workflow run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRun {
    pub id: u64,
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub branch: String,
    pub html_url: String,
    pub created_at: String,
    pub updated_at: String,
    pub run_number: u64,
}

/// Parameters for listing workflow runs
#[derive(Debug, Clone, Default)]
pub struct RunsQuery<'a> {
    pub owner: &'a str,
    pub repo: &'a str,
    pub branch: Option<&'a str>,
    pub status: Option<&'a str>,
    pub limit: usize,
}

/// A test failure extracted from CI logs
#[derive(Debug, Clone)]
pub struct TestFailure {
    /// The spec file path (e.g., "spec/models/user_spec.rb")
    pub spec_file: String,
    /// The failure message/output
    pub failure_text: String,
}

/// A test failure enriched with source file mapping
#[derive(Debug, Clone, Serialize)]
pub struct FixFailure {
    pub test_file: String,
    pub source_files: Vec<String>,
    pub failure_text: String,
    pub language: String,
}

/// Full fix report for a failed CI run
#[derive(Debug, Clone, Serialize)]
pub struct FixReport {
    pub repository: String,
    pub pr_number: Option<u64>,
    pub run_id: u64,
    pub failures: Vec<FixFailure>,
    pub test_files: Vec<String>,
    pub source_files: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pull_request_serializes() {
        let pr = PullRequest {
            number: 123,
            title: "Fix bug".to_string(),
            html_url: "https://github.com/org/repo/pull/123".to_string(),
            state: "open".to_string(),
            repo_full_name: "org/repo".to_string(),
            created_at: "2024-01-15T10:00:00Z".to_string(),
            updated_at: "2024-01-15T12:00:00Z".to_string(),
            ci_status: None,
        };

        let json = serde_json::to_string(&pr).unwrap();
        assert!(json.contains("Fix bug"));
        assert!(json.contains("org/repo"));
    }

    #[test]
    fn pull_request_deserializes() {
        let json = r#"{
            "number": 456,
            "title": "Add feature",
            "html_url": "https://github.com/org/repo/pull/456",
            "state": "open",
            "repo_full_name": "org/repo",
            "created_at": "2024-01-15T10:00:00Z",
            "updated_at": "2024-01-15T12:00:00Z"
        }"#;

        let pr: PullRequest = serde_json::from_str(json).unwrap();
        assert_eq!(pr.number, 456);
        assert_eq!(pr.title, "Add feature");
        assert!(pr.ci_status.is_none());
    }

    #[test]
    fn ci_status_default_is_unknown() {
        let status = CiStatus::default();
        assert_eq!(status, CiStatus::Unknown);
    }

    #[test]
    fn ci_status_equality() {
        assert_eq!(CiStatus::Success, CiStatus::Success);
        assert_eq!(CiStatus::Pending, CiStatus::Pending);
        assert_eq!(CiStatus::Failed, CiStatus::Failed);
        assert_eq!(CiStatus::Unknown, CiStatus::Unknown);
        assert_ne!(CiStatus::Success, CiStatus::Failed);
    }

    #[test]
    fn ci_status_clone() {
        let status = CiStatus::Success;
        let cloned = status;
        assert_eq!(status, cloned);
    }

    #[test]
    fn ci_status_debug_format() {
        let debug_str = format!("{:?}", CiStatus::Pending);
        assert!(debug_str.contains("Pending"));
    }

    #[test]
    fn test_failure_clone() {
        let failure = TestFailure {
            spec_file: "./spec/test_spec.rb:10".to_string(),
            failure_text: "expected true, got false".to_string(),
        };
        let cloned = failure.clone();
        assert_eq!(cloned.spec_file, failure.spec_file);
        assert_eq!(cloned.failure_text, failure.failure_text);
    }

    #[test]
    fn test_failure_debug_format() {
        let failure = TestFailure {
            spec_file: "./spec/test_spec.rb:10".to_string(),
            failure_text: "error".to_string(),
        };
        let debug_str = format!("{:?}", failure);
        assert!(debug_str.contains("TestFailure"));
        assert!(debug_str.contains("spec_file"));
    }

    #[test]
    fn pull_request_clone() {
        let pr = PullRequest {
            number: 123,
            title: "Test".to_string(),
            html_url: "https://github.com/a/b/pull/123".to_string(),
            state: "open".to_string(),
            repo_full_name: "a/b".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            ci_status: Some(CiStatus::Success),
        };
        let cloned = pr.clone();
        assert_eq!(cloned.number, pr.number);
        assert_eq!(cloned.ci_status, pr.ci_status);
    }

    #[test]
    fn pull_request_debug_format() {
        let pr = PullRequest {
            number: 1,
            title: "T".to_string(),
            html_url: "u".to_string(),
            state: "open".to_string(),
            repo_full_name: "r".to_string(),
            created_at: "c".to_string(),
            updated_at: "u".to_string(),
            ci_status: None,
        };
        let debug_str = format!("{:?}", pr);
        assert!(debug_str.contains("PullRequest"));
    }

    #[test]
    fn fix_failure_serializes() {
        let f = FixFailure {
            test_file: "spec/models/user_spec.rb:10".to_string(),
            source_files: vec!["app/models/user.rb".to_string()],
            failure_text: "expected true".to_string(),
            language: "ruby".to_string(),
        };
        let json = serde_json::to_string(&f).unwrap();
        assert!(json.contains("user_spec.rb"));
        assert!(json.contains("app/models/user.rb"));
        assert!(json.contains("ruby"));
    }

    #[test]
    fn fix_failure_clone() {
        let f = FixFailure {
            test_file: "test.rb".to_string(),
            source_files: vec!["src.rb".to_string()],
            failure_text: "err".to_string(),
            language: "ruby".to_string(),
        };
        let c = f.clone();
        assert_eq!(c.test_file, f.test_file);
        assert_eq!(c.source_files, f.source_files);
    }

    #[test]
    fn fix_failure_debug() {
        let f = FixFailure {
            test_file: "t".to_string(),
            source_files: vec![],
            failure_text: "e".to_string(),
            language: "rust".to_string(),
        };
        let d = format!("{:?}", f);
        assert!(d.contains("FixFailure"));
    }

    #[test]
    fn fix_report_serializes() {
        let r = FixReport {
            repository: "owner/repo".to_string(),
            pr_number: Some(42),
            run_id: 123,
            failures: vec![],
            test_files: vec!["spec/a_spec.rb".to_string()],
            source_files: vec!["app/a.rb".to_string()],
        };
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("owner/repo"));
        assert!(json.contains("42"));
        assert!(json.contains("123"));
    }

    #[test]
    fn fix_report_clone() {
        let r = FixReport {
            repository: "o/r".to_string(),
            pr_number: None,
            run_id: 1,
            failures: vec![],
            test_files: vec![],
            source_files: vec![],
        };
        let c = r.clone();
        assert_eq!(c.repository, r.repository);
        assert_eq!(c.pr_number, r.pr_number);
    }

    #[test]
    fn fix_report_debug() {
        let r = FixReport {
            repository: "o/r".to_string(),
            pr_number: None,
            run_id: 1,
            failures: vec![],
            test_files: vec![],
            source_files: vec![],
        };
        let d = format!("{:?}", r);
        assert!(d.contains("FixReport"));
    }

    #[test]
    fn workflow_run_serializes() {
        let run = WorkflowRun {
            id: 100,
            name: "Test Suite".to_string(),
            status: "completed".to_string(),
            conclusion: Some("success".to_string()),
            branch: "main".to_string(),
            html_url: "https://github.com/o/r/actions/runs/100".to_string(),
            created_at: "2024-01-15T10:00:00Z".to_string(),
            updated_at: "2024-01-15T10:05:00Z".to_string(),
            run_number: 42,
        };
        let json = serde_json::to_string(&run).unwrap();
        assert!(json.contains("Test Suite"));
        assert!(json.contains("100"));
        assert!(json.contains("main"));
    }

    #[test]
    fn workflow_run_deserializes() {
        let json = r#"{
            "id": 200,
            "name": "Lint",
            "status": "in_progress",
            "conclusion": null,
            "branch": "feature",
            "html_url": "https://github.com/o/r/actions/runs/200",
            "created_at": "2024-01-15T10:00:00Z",
            "updated_at": "2024-01-15T10:05:00Z",
            "run_number": 7
        }"#;
        let run: WorkflowRun = serde_json::from_str(json).unwrap();
        assert_eq!(run.id, 200);
        assert_eq!(run.name, "Lint");
        assert!(run.conclusion.is_none());
    }

    #[test]
    fn workflow_run_clone() {
        let run = WorkflowRun {
            id: 1,
            name: "CI".to_string(),
            status: "completed".to_string(),
            conclusion: Some("failure".to_string()),
            branch: "dev".to_string(),
            html_url: "u".to_string(),
            created_at: "c".to_string(),
            updated_at: "u".to_string(),
            run_number: 1,
        };
        let cloned = run.clone();
        assert_eq!(cloned.id, run.id);
        assert_eq!(cloned.conclusion, run.conclusion);
    }

    #[test]
    fn runs_query_debug() {
        let q = RunsQuery {
            owner: "o",
            repo: "r",
            branch: Some("main"),
            status: None,
            limit: 20,
        };
        let d = format!("{:?}", q);
        assert!(d.contains("RunsQuery"));
    }

    #[test]
    fn runs_query_clone() {
        let q = RunsQuery {
            owner: "o",
            repo: "r",
            branch: None,
            status: Some("completed"),
            limit: 10,
        };
        let c = q.clone();
        assert_eq!(c.owner, q.owner);
        assert_eq!(c.limit, q.limit);
    }

    #[test]
    fn runs_query_default() {
        let q = RunsQuery::default();
        assert_eq!(q.owner, "");
        assert_eq!(q.repo, "");
        assert!(q.branch.is_none());
        assert!(q.status.is_none());
        assert_eq!(q.limit, 0);
    }

    #[test]
    fn workflow_run_debug() {
        let run = WorkflowRun {
            id: 1,
            name: "N".to_string(),
            status: "s".to_string(),
            conclusion: None,
            branch: "b".to_string(),
            html_url: "u".to_string(),
            created_at: "c".to_string(),
            updated_at: "u".to_string(),
            run_number: 1,
        };
        let d = format!("{:?}", run);
        assert!(d.contains("WorkflowRun"));
    }

    #[test]
    fn fix_report_no_pr() {
        let r = FixReport {
            repository: "o/r".to_string(),
            pr_number: None,
            run_id: 1,
            failures: vec![],
            test_files: vec![],
            source_files: vec![],
        };
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("null"));
    }
}
