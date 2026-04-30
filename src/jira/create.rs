//! `hu jira create` — create a new issue.
//!
//! Requires a project key (passed via `--project` or the
//! `HU_JIRA_PROJECT` environment variable) and a summary. Issue type
//! defaults to "Task" but is validated against the project's
//! createmeta so typos surface a useful "available types: …" error.

use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

use super::client::{JiraApi, JiraClient};
use super::types::{IssueCreate, IssueType};
use super::update::load_adf;

/// Arguments for the create command. Mirrors the CLI struct for ease
/// of testing without the clap parser.
#[derive(Debug, Clone)]
pub struct CreateArgs {
    pub project_key: String,
    pub summary: String,
    pub issue_type: String,
    pub body: Option<String>,
    pub body_adf: Option<PathBuf>,
    pub assign: Option<String>,
    pub json: bool,
}

/// Run the jira create command (CLI entry point — formats and prints).
pub async fn run(args: CreateArgs) -> Result<()> {
    let client = JiraClient::new().await?;
    let output = process_create(&client, &args).await?;
    print!("{}", output);
    Ok(())
}

/// Process create command (business logic, testable).
pub async fn process_create(client: &impl JiraApi, args: &CreateArgs) -> Result<String> {
    if args.summary.trim().is_empty() {
        bail!("Summary is required and cannot be empty");
    }
    if args.project_key.trim().is_empty() {
        bail!("Project key is required (use --project or set HU_JIRA_PROJECT)");
    }

    // Validate issue type against the project's createmeta. Fuzzy match
    // so "task" matches "Task", "bug" matches "Bug", etc.
    let issue_types = client.get_issue_types(&args.project_key).await?;
    let resolved_type = find_issue_type(&issue_types, &args.issue_type)?;

    let description_adf = match &args.body_adf {
        Some(path) => Some(load_adf_from(path)?),
        None => None,
    };

    let assignee = match &args.assign {
        Some(a) if a == "me" => Some(client.get_current_user().await?.account_id),
        Some(a) => Some(a.clone()),
        None => None,
    };

    let new = IssueCreate {
        project_key: args.project_key.clone(),
        summary: args.summary.clone(),
        issue_type: resolved_type.name.clone(),
        description: args.body.clone(),
        description_adf,
        assignee,
    };

    let created = client.create_issue(&new).await?;

    if args.json {
        let json = serde_json::to_string_pretty(&created).unwrap_or_default();
        return Ok(format!("{}\n", json));
    }
    Ok(format!(
        "\x1b[32m\u{2713}\x1b[0m Created \x1b[1m{}\x1b[0m: {}\n   {}\n",
        created.key, args.summary, created.url
    ))
}

/// Wrapper around [`update::load_adf`] so this module can read raw ADF
/// without re-implementing validation.
fn load_adf_from(path: &Path) -> Result<serde_json::Value> {
    load_adf(path)
}

/// Resolve a user-supplied type string against the project's available
/// issue types. Exact match (case-insensitive) wins; falls back to a
/// substring match. On miss, lists what was offered so the user can
/// retry without poking around in Jira.
pub fn find_issue_type<'a>(types: &'a [IssueType], requested: &str) -> Result<&'a IssueType> {
    let target = requested.to_lowercase();

    if let Some(t) = types.iter().find(|t| t.name.to_lowercase() == target) {
        return Ok(t);
    }
    if let Some(t) = types
        .iter()
        .find(|t| t.name.to_lowercase().contains(&target))
    {
        return Ok(t);
    }

    let available: Vec<&str> = types.iter().map(|t| t.name.as_str()).collect();
    if available.is_empty() {
        bail!("No issue types returned for this project. Check project key and permissions.");
    }
    bail!(
        "Issue type '{}' not found. Available: {}",
        requested,
        available.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::super::types::{Comment, CreatedIssue, Transition, User};
    use super::*;
    use serde_json::json;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn make_types(names: &[&str]) -> Vec<IssueType> {
        names
            .iter()
            .enumerate()
            .map(|(i, n)| IssueType {
                id: format!("{}", 10000 + i),
                name: n.to_string(),
                description: None,
            })
            .collect()
    }

    #[test]
    fn find_issue_type_exact_match() {
        let types = make_types(&["Task", "Bug", "Story"]);
        let t = find_issue_type(&types, "Bug").unwrap();
        assert_eq!(t.name, "Bug");
    }

    #[test]
    fn find_issue_type_case_insensitive() {
        let types = make_types(&["Task", "Bug", "Story"]);
        let t = find_issue_type(&types, "bug").unwrap();
        assert_eq!(t.name, "Bug");
    }

    #[test]
    fn find_issue_type_substring_match() {
        let types = make_types(&["Story", "Sub-task", "Epic"]);
        let t = find_issue_type(&types, "sub").unwrap();
        assert_eq!(t.name, "Sub-task");
    }

    #[test]
    fn find_issue_type_lists_available_on_miss() {
        let types = make_types(&["Task", "Bug", "Story"]);
        let err = find_issue_type(&types, "Feature").unwrap_err().to_string();
        assert!(err.contains("Feature"));
        assert!(err.contains("Task"));
        assert!(err.contains("Bug"));
        assert!(err.contains("Story"));
    }

    #[test]
    fn find_issue_type_empty_list_explains() {
        let types = make_types(&[]);
        let err = find_issue_type(&types, "Task").unwrap_err().to_string();
        assert!(err.contains("No issue types"));
    }

    // Mock used by process_create tests
    struct MockJiraClient {
        types: Vec<IssueType>,
        user: User,
        captured: std::sync::Mutex<Option<IssueCreate>>,
        created: CreatedIssue,
    }

    impl JiraApi for MockJiraClient {
        async fn get_current_user(&self) -> Result<User> {
            Ok(self.user.clone())
        }

        async fn get_issue(&self, _key: &str) -> Result<super::super::types::Issue> {
            unimplemented!()
        }

        async fn search_issues(&self, _jql: &str) -> Result<Vec<super::super::types::Issue>> {
            unimplemented!()
        }

        async fn update_issue(
            &self,
            _key: &str,
            _update: &super::super::types::IssueUpdate,
        ) -> Result<()> {
            unimplemented!()
        }

        async fn get_transitions(&self, _key: &str) -> Result<Vec<Transition>> {
            unimplemented!()
        }

        async fn transition_issue(&self, _key: &str, _transition_id: &str) -> Result<()> {
            unimplemented!()
        }

        async fn list_comments(&self, _key: &str) -> Result<Vec<Comment>> {
            unimplemented!()
        }

        async fn create_issue(&self, new: &IssueCreate) -> Result<CreatedIssue> {
            *self.captured.lock().unwrap() = Some(new.clone());
            Ok(self.created.clone())
        }

        async fn get_issue_types(&self, _project_key: &str) -> Result<Vec<IssueType>> {
            Ok(self.types.clone())
        }
    }

    fn make_mock() -> MockJiraClient {
        MockJiraClient {
            types: make_types(&["Task", "Bug", "Story"]),
            user: User {
                account_id: "me-account".to_string(),
                display_name: "Me".to_string(),
                email_address: None,
            },
            captured: std::sync::Mutex::new(None),
            created: CreatedIssue {
                id: "10000".to_string(),
                key: "HU-1".to_string(),
                url: "https://example.atlassian.net/browse/HU-1".to_string(),
            },
        }
    }

    fn args(project: &str, summary: &str, issue_type: &str) -> CreateArgs {
        CreateArgs {
            project_key: project.to_string(),
            summary: summary.to_string(),
            issue_type: issue_type.to_string(),
            body: None,
            body_adf: None,
            assign: None,
            json: false,
        }
    }

    #[tokio::test]
    async fn process_create_succeeds_and_renders_url() {
        let client = make_mock();
        let out = process_create(&client, &args("HU", "Test issue", "Task"))
            .await
            .unwrap();
        assert!(out.contains("HU-1"));
        assert!(out.contains("Test issue"));
        assert!(out.contains("https://example.atlassian.net/browse/HU-1"));

        let captured = client.captured.lock().unwrap();
        let cap = captured.as_ref().unwrap();
        assert_eq!(cap.project_key, "HU");
        assert_eq!(cap.summary, "Test issue");
        assert_eq!(cap.issue_type, "Task");
    }

    #[tokio::test]
    async fn process_create_resolves_lowercase_type() {
        let client = make_mock();
        let _ = process_create(&client, &args("HU", "x", "task"))
            .await
            .unwrap();
        assert_eq!(
            client.captured.lock().unwrap().as_ref().unwrap().issue_type,
            "Task"
        );
    }

    #[tokio::test]
    async fn process_create_rejects_unknown_type() {
        let client = make_mock();
        let err = process_create(&client, &args("HU", "x", "Feature"))
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("Feature"));
    }

    #[tokio::test]
    async fn process_create_rejects_empty_summary() {
        let client = make_mock();
        let err = process_create(&client, &args("HU", "  ", "Task"))
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("Summary"));
    }

    #[tokio::test]
    async fn process_create_rejects_empty_project() {
        let client = make_mock();
        let err = process_create(&client, &args("", "x", "Task"))
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("Project key"));
    }

    #[tokio::test]
    async fn process_create_resolves_assign_me_to_account_id() {
        let client = make_mock();
        let mut a = args("HU", "x", "Task");
        a.assign = Some("me".to_string());
        let _ = process_create(&client, &a).await.unwrap();
        assert_eq!(
            client
                .captured
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .assignee
                .as_deref(),
            Some("me-account")
        );
    }

    #[tokio::test]
    async fn process_create_passes_explicit_assignee_through() {
        let client = make_mock();
        let mut a = args("HU", "x", "Task");
        a.assign = Some("other-user-id".to_string());
        let _ = process_create(&client, &a).await.unwrap();
        assert_eq!(
            client
                .captured
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .assignee
                .as_deref(),
            Some("other-user-id")
        );
    }

    #[tokio::test]
    async fn process_create_loads_body_adf_from_file() {
        let client = make_mock();
        let mut file = NamedTempFile::new().unwrap();
        let doc = json!({
            "type": "doc",
            "version": 1,
            "content": [{
                "type": "paragraph",
                "content": [{"type": "text", "text": "raw"}]
            }]
        });
        file.write_all(doc.to_string().as_bytes()).unwrap();

        let mut a = args("HU", "x", "Task");
        a.body_adf = Some(file.path().to_path_buf());
        let _ = process_create(&client, &a).await.unwrap();

        let captured = client.captured.lock().unwrap();
        let cap = captured.as_ref().unwrap();
        let adf = cap.description_adf.as_ref().unwrap();
        assert_eq!(adf["content"][0]["content"][0]["text"], "raw");
    }

    #[tokio::test]
    async fn process_create_json_output_serialises_created_issue() {
        let client = make_mock();
        let mut a = args("HU", "x", "Task");
        a.json = true;
        let out = process_create(&client, &a).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
        assert_eq!(parsed["key"], "HU-1");
        assert_eq!(parsed["url"], "https://example.atlassian.net/browse/HU-1");
    }
}
