use super::*;
use crate::gh::client::GithubApi;
use crate::gh::types::{CiStatus, PullRequest, RunsQuery, WorkflowRun};
use anyhow::Result;

// Mock implementation
struct MockGithubApi {
    prs: Vec<PullRequest>,
    runs: Vec<WorkflowRun>,
    branches: std::collections::HashMap<u64, String>,
}

impl MockGithubApi {
    fn new() -> Self {
        Self {
            prs: vec![],
            runs: vec![],
            branches: std::collections::HashMap::new(),
        }
    }

    fn with_runs(mut self, runs: Vec<WorkflowRun>) -> Self {
        self.runs = runs;
        self
    }

    fn with_prs(mut self, prs: Vec<PullRequest>) -> Self {
        self.prs = prs;
        self
    }

    fn with_branch(mut self, pr_number: u64, branch: String) -> Self {
        self.branches.insert(pr_number, branch);
        self
    }
}

impl GithubApi for MockGithubApi {
    async fn list_user_prs(&self) -> Result<Vec<PullRequest>> {
        Ok(self.prs.clone())
    }

    async fn get_ci_status(&self, _owner: &str, _repo: &str, _pr: u64) -> Result<CiStatus> {
        Ok(CiStatus::Unknown)
    }

    async fn get_pr_branch(&self, _owner: &str, _repo: &str, pr: u64) -> Result<String> {
        Ok(self
            .branches
            .get(&pr)
            .cloned()
            .unwrap_or_else(|| "main".to_string()))
    }

    async fn get_latest_failed_run_for_branch(
        &self,
        _owner: &str,
        _repo: &str,
        _branch: &str,
    ) -> Result<Option<u64>> {
        Ok(None)
    }

    async fn get_failed_jobs(
        &self,
        _owner: &str,
        _repo: &str,
        _run_id: u64,
    ) -> Result<Vec<(u64, String)>> {
        Ok(vec![])
    }

    async fn get_job_logs(&self, _owner: &str, _repo: &str, _job_id: u64) -> Result<String> {
        Ok(String::new())
    }

    async fn find_pr_for_branch(
        &self,
        _owner: &str,
        _repo: &str,
        _branch: &str,
    ) -> Result<Option<u64>> {
        Ok(None)
    }

    async fn list_workflow_runs(&self, query: &RunsQuery<'_>) -> Result<Vec<WorkflowRun>> {
        let mut runs: Vec<WorkflowRun> = self
            .runs
            .iter()
            .filter(|r| query.branch.is_none_or(|b| r.branch == b))
            .filter(|r| {
                query
                    .status
                    .is_none_or(|s| r.status == s || r.conclusion.as_deref() == Some(s))
            })
            .cloned()
            .collect();
        runs.truncate(query.limit);
        Ok(runs)
    }

    async fn search_prs_by_title(
        &self,
        _owner: &str,
        _repo: &str,
        query: &str,
    ) -> Result<Vec<PullRequest>> {
        let query_lower = query.to_lowercase();
        Ok(self
            .prs
            .iter()
            .filter(|pr| pr.title.to_lowercase().contains(&query_lower))
            .cloned()
            .collect())
    }
}

fn make_run(
    id: u64,
    name: &str,
    status: &str,
    conclusion: Option<&str>,
    branch: &str,
) -> WorkflowRun {
    WorkflowRun {
        id,
        name: name.to_string(),
        status: status.to_string(),
        conclusion: conclusion.map(|s| s.to_string()),
        branch: branch.to_string(),
        html_url: format!("https://github.com/o/r/actions/runs/{id}"),
        created_at: format!("2024-01-15T{:02}:00:00Z", id % 24),
        updated_at: format!("2024-01-15T{:02}:05:00Z", id % 24),
        run_number: id,
    }
}

fn make_pr(number: u64, title: &str) -> PullRequest {
    PullRequest {
        number,
        title: title.to_string(),
        html_url: format!("https://github.com/o/r/pull/{number}"),
        state: "open".to_string(),
        repo_full_name: "o/r".to_string(),
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
        ci_status: None,
    }
}

fn default_args() -> RunsArgs {
    RunsArgs {
        ticket: None,
        status: None,
        branch: None,
        repo: None,
        limit: 20,
        json: false,
    }
}

// status_icon tests
#[test]
fn status_icon_success() {
    let run = make_run(1, "CI", "completed", Some("success"), "main");
    let icon = status_icon(&run);
    assert!(icon.contains("✓"));
    assert!(icon.contains(GREEN));
}

#[test]
fn status_icon_failure() {
    let run = make_run(1, "CI", "completed", Some("failure"), "main");
    let icon = status_icon(&run);
    assert!(icon.contains("✗"));
    assert!(icon.contains(RED));
}

#[test]
fn status_icon_in_progress() {
    let run = make_run(1, "CI", "in_progress", None, "main");
    let icon = status_icon(&run);
    assert!(icon.contains("◐"));
    assert!(icon.contains(YELLOW));
}

#[test]
fn status_icon_queued() {
    let run = make_run(1, "CI", "queued", None, "main");
    let icon = status_icon(&run);
    assert!(icon.contains("○"));
    assert!(icon.contains(GRAY));
}

#[test]
fn status_icon_cancelled() {
    let run = make_run(1, "CI", "completed", Some("cancelled"), "main");
    let icon = status_icon(&run);
    assert!(icon.contains("○"));
    assert!(icon.contains(GRAY));
}

#[test]
fn status_icon_unknown_status() {
    let run = make_run(1, "CI", "unknown", None, "main");
    let icon = status_icon(&run);
    assert!(icon.contains("○"));
}

// truncate tests
#[test]
fn truncate_short() {
    assert_eq!(truncate("hello", 10), "hello");
}

#[test]
fn truncate_long() {
    assert_eq!(truncate("hello world", 8), "hello w…");
}

#[test]
fn truncate_exact() {
    assert_eq!(truncate("hello", 5), "hello");
}

#[test]
fn truncate_empty() {
    assert_eq!(truncate("", 10), "");
}

// print_runs_table tests
#[test]
fn print_runs_table_renders_without_panic() {
    let runs = vec![
        make_run(1, "CI", "completed", Some("success"), "main"),
        make_run(2, "Lint", "completed", Some("failure"), "feature"),
        make_run(3, "Deploy", "in_progress", None, "main"),
    ];
    print_runs_table(&runs);
}

#[test]
fn print_runs_table_empty() {
    let runs: Vec<WorkflowRun> = vec![];
    print_runs_table(&runs);
}

#[test]
fn print_runs_table_long_names() {
    let runs = vec![make_run(
        1,
        "A very long workflow name that should be truncated",
        "completed",
        Some("success"),
        "a-very-long-branch-name-too",
    )];
    print_runs_table(&runs);
}

// print_runs_json tests
#[test]
fn print_runs_json_renders() {
    let runs = vec![make_run(1, "CI", "completed", Some("success"), "main")];
    print_runs_json(&runs);
}

#[test]
fn print_runs_json_empty() {
    let runs: Vec<WorkflowRun> = vec![];
    print_runs_json(&runs);
}

// get_terminal_width test
#[test]
fn get_terminal_width_reasonable() {
    let width = get_terminal_width();
    assert!(width >= 20);
}

// run_with_client tests
#[tokio::test]
async fn run_with_client_no_runs() {
    let mock = MockGithubApi::new();
    let args = default_args();
    let result = run_with_client(&mock, "o", "r", &args).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn run_with_client_with_runs() {
    let runs = vec![
        make_run(1, "CI", "completed", Some("success"), "main"),
        make_run(2, "Lint", "completed", Some("failure"), "main"),
    ];
    let mock = MockGithubApi::new().with_runs(runs);
    let args = default_args();
    let result = run_with_client(&mock, "o", "r", &args).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn run_with_client_json_output() {
    let runs = vec![make_run(1, "CI", "completed", Some("success"), "main")];
    let mock = MockGithubApi::new().with_runs(runs);
    let mut args = default_args();
    args.json = true;
    let result = run_with_client(&mock, "o", "r", &args).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn run_with_client_branch_filter() {
    let runs = vec![
        make_run(1, "CI", "completed", Some("success"), "main"),
        make_run(2, "CI", "completed", Some("failure"), "feature"),
    ];
    let mock = MockGithubApi::new().with_runs(runs);
    let mut args = default_args();
    args.branch = Some("feature".to_string());
    let result = run_with_client(&mock, "o", "r", &args).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn run_with_client_status_filter() {
    let runs = vec![
        make_run(1, "CI", "completed", Some("success"), "main"),
        make_run(2, "CI", "completed", Some("failure"), "main"),
    ];
    let mock = MockGithubApi::new().with_runs(runs);
    let mut args = default_args();
    args.status = Some("failure".to_string());
    let result = run_with_client(&mock, "o", "r", &args).await;
    assert!(result.is_ok());
}

// fetch_runs_for_ticket tests
#[tokio::test]
async fn fetch_runs_for_ticket_no_prs() {
    let mock = MockGithubApi::new();
    let args = default_args();
    let runs = fetch_runs_for_ticket(&mock, "o", "r", "BFR-999", &args).await;
    assert!(runs.is_ok());
    assert!(runs.unwrap().is_empty());
}

#[tokio::test]
async fn fetch_runs_for_ticket_with_prs() {
    let pr = make_pr(1, "BFR-1234 Fix bug");
    let runs = vec![
        make_run(10, "CI", "completed", Some("success"), "bfr-1234-fix"),
        make_run(11, "Lint", "completed", Some("success"), "bfr-1234-fix"),
    ];
    let mock = MockGithubApi::new()
        .with_prs(vec![pr])
        .with_runs(runs)
        .with_branch(1, "bfr-1234-fix".to_string());
    let args = default_args();
    let result = fetch_runs_for_ticket(&mock, "o", "r", "BFR-1234", &args).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 2);
}

#[tokio::test]
async fn fetch_runs_for_ticket_deduplicates_branches() {
    let prs = vec![
        make_pr(1, "BFR-1234 First PR"),
        make_pr(2, "BFR-1234 Second PR"),
    ];
    let runs = vec![make_run(
        10,
        "CI",
        "completed",
        Some("success"),
        "same-branch",
    )];
    let mock = MockGithubApi::new()
        .with_prs(prs)
        .with_runs(runs)
        .with_branch(1, "same-branch".to_string())
        .with_branch(2, "same-branch".to_string());
    let args = default_args();
    let result = fetch_runs_for_ticket(&mock, "o", "r", "BFR-1234", &args).await;
    assert!(result.is_ok());
    // Should only query once since both PRs point to same branch
    assert_eq!(result.unwrap().len(), 1);
}

#[tokio::test]
async fn fetch_runs_for_ticket_respects_limit() {
    let pr = make_pr(1, "BFR-1234 Fix");
    let runs: Vec<WorkflowRun> = (0..10)
        .map(|i| make_run(i, "CI", "completed", Some("success"), "feature"))
        .collect();
    let mock = MockGithubApi::new()
        .with_prs(vec![pr])
        .with_runs(runs)
        .with_branch(1, "feature".to_string());
    let mut args = default_args();
    args.limit = 3;
    let result = fetch_runs_for_ticket(&mock, "o", "r", "BFR-1234", &args).await;
    assert!(result.is_ok());
    assert!(result.unwrap().len() <= 3);
}

#[tokio::test]
async fn run_with_client_ticket_search() {
    let pr = make_pr(1, "BFR-1234 Fix bug");
    let runs = vec![make_run(10, "CI", "completed", Some("success"), "bfr-1234")];
    let mock = MockGithubApi::new()
        .with_prs(vec![pr])
        .with_runs(runs)
        .with_branch(1, "bfr-1234".to_string());
    let mut args = default_args();
    args.ticket = Some("BFR-1234".to_string());
    let result = run_with_client(&mock, "o", "r", &args).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn fetch_runs_for_ticket_invalid_repo_name() {
    let mut pr = make_pr(1, "BFR-1234 Fix bug");
    pr.repo_full_name = "invalid-no-slash".to_string();
    let runs = vec![make_run(10, "CI", "completed", Some("success"), "feature")];
    let mock = MockGithubApi::new()
        .with_prs(vec![pr])
        .with_runs(runs)
        .with_branch(1, "feature".to_string());
    let args = default_args();
    let result = fetch_runs_for_ticket(&mock, "o", "r", "BFR-1234", &args).await;
    assert!(result.is_ok());
    // Should still work, falling back to owner/repo params
    assert_eq!(result.unwrap().len(), 1);
}

#[tokio::test]
async fn run_with_client_ticket_no_results() {
    let mock = MockGithubApi::new();
    let mut args = default_args();
    args.ticket = Some("NONE-999".to_string());
    let result = run_with_client(&mock, "o", "r", &args).await;
    assert!(result.is_ok());
}
