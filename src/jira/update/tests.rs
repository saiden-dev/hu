use std::io::Write;

use serde_json::json;
use tempfile::NamedTempFile;

use super::super::types::User;
use super::*;

fn empty_args(key: &str) -> UpdateArgs {
    UpdateArgs {
        key: key.to_string(),
        summary: None,
        status: None,
        assign: None,
        body: None,
        body_adf: None,
    }
}

#[test]
fn update_args_debug() {
    let args = UpdateArgs {
        summary: Some("New".to_string()),
        ..empty_args("X-1")
    };
    let debug_str = format!("{:?}", args);
    assert!(debug_str.contains("UpdateArgs"));
}

#[test]
fn update_args_clone() {
    let args = UpdateArgs {
        summary: Some("S".to_string()),
        status: Some("Done".to_string()),
        assign: Some("user".to_string()),
        body: Some("B".to_string()),
        ..empty_args("X-1")
    };
    let cloned = args.clone();
    assert_eq!(cloned.key, args.key);
    assert_eq!(cloned.summary, args.summary);
    assert_eq!(cloned.status, args.status);
    assert_eq!(cloned.assign, args.assign);
}

#[test]
fn find_transition_exact_match() {
    let transitions = vec![
        Transition {
            id: "11".to_string(),
            name: "To Do".to_string(),
        },
        Transition {
            id: "21".to_string(),
            name: "In Progress".to_string(),
        },
        Transition {
            id: "31".to_string(),
            name: "Done".to_string(),
        },
    ];

    let t = find_transition(&transitions, "Done").unwrap();
    assert_eq!(t.id, "31");
    assert_eq!(t.name, "Done");
}

#[test]
fn find_transition_case_insensitive() {
    let transitions = vec![Transition {
        id: "21".to_string(),
        name: "In Progress".to_string(),
    }];

    let t = find_transition(&transitions, "in progress").unwrap();
    assert_eq!(t.id, "21");

    let t2 = find_transition(&transitions, "IN PROGRESS").unwrap();
    assert_eq!(t2.id, "21");
}

#[test]
fn find_transition_partial_match() {
    let transitions = vec![
        Transition {
            id: "11".to_string(),
            name: "Start Progress".to_string(),
        },
        Transition {
            id: "21".to_string(),
            name: "In Progress".to_string(),
        },
    ];

    let t = find_transition(&transitions, "progress").unwrap();
    assert!(t.name.contains("Progress"));
}

#[test]
fn find_transition_not_found() {
    let transitions = vec![
        Transition {
            id: "11".to_string(),
            name: "To Do".to_string(),
        },
        Transition {
            id: "31".to_string(),
            name: "Done".to_string(),
        },
    ];

    let result = find_transition(&transitions, "In Progress");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("In Progress"));
    assert!(err.contains("To Do"));
    assert!(err.contains("Done"));
}

#[test]
fn find_transition_empty_list() {
    let transitions: Vec<Transition> = vec![];
    let result = find_transition(&transitions, "Done");
    assert!(result.is_err());
}

#[test]
fn load_adf_accepts_well_formed_doc() {
    let mut file = NamedTempFile::new().unwrap();
    let doc = json!({
        "type": "doc",
        "version": 1,
        "content": [{"type": "paragraph", "content": [{"type": "text", "text": "hi"}]}]
    });
    file.write_all(doc.to_string().as_bytes()).unwrap();
    let loaded = load_adf(file.path()).unwrap();
    assert_eq!(loaded["type"], "doc");
    assert_eq!(loaded["content"][0]["type"], "paragraph");
}

#[test]
fn load_adf_rejects_missing_doc_type() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(br#"{"type": "paragraph", "content": []}"#)
        .unwrap();
    let err = load_adf(file.path()).unwrap_err().to_string();
    assert!(err.contains("\"type\": \"doc\""));
}

#[test]
fn load_adf_rejects_missing_content_array() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(br#"{"type": "doc", "version": 1}"#).unwrap();
    let err = load_adf(file.path()).unwrap_err().to_string();
    assert!(err.contains("content"));
}

#[test]
fn load_adf_rejects_invalid_json() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(b"not json").unwrap();
    let err = load_adf(file.path()).unwrap_err().to_string();
    assert!(err.contains("Invalid JSON"));
}

#[test]
fn load_adf_rejects_missing_file() {
    let err = load_adf(std::path::Path::new("/nonexistent/path/no.json"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("Failed to read"));
}

// Mock client for testing process_update
struct MockJiraClient {
    user: User,
    transitions: Vec<Transition>,
    updated_fields: std::sync::Mutex<Option<IssueUpdate>>,
    transitioned_to: std::sync::Mutex<Option<String>>,
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

    async fn update_issue(&self, _key: &str, update: &IssueUpdate) -> Result<()> {
        *self.updated_fields.lock().unwrap() = Some(update.clone());
        Ok(())
    }

    async fn get_transitions(&self, _key: &str) -> Result<Vec<Transition>> {
        Ok(self.transitions.clone())
    }

    async fn transition_issue(&self, _key: &str, transition_id: &str) -> Result<()> {
        *self.transitioned_to.lock().unwrap() = Some(transition_id.to_string());
        Ok(())
    }

    async fn list_comments(&self, _key: &str) -> Result<Vec<super::super::types::Comment>> {
        unimplemented!()
    }
}

fn make_mock(user_account_id: &str, transitions: Vec<Transition>) -> MockJiraClient {
    MockJiraClient {
        user: User {
            account_id: user_account_id.to_string(),
            display_name: "Me".to_string(),
            email_address: None,
        },
        transitions,
        updated_fields: std::sync::Mutex::new(None),
        transitioned_to: std::sync::Mutex::new(None),
    }
}

#[tokio::test]
async fn process_update_changes_summary() {
    let client = make_mock("me123", vec![]);

    let args = UpdateArgs {
        summary: Some("New summary".to_string()),
        ..empty_args("X-1")
    };

    let output = process_update(&client, &args).await.unwrap();
    assert!(output.contains("Updated summary"));
    assert!(output.contains("New summary"));

    let updated = client.updated_fields.lock().unwrap();
    assert!(updated.is_some());
    assert_eq!(
        updated.as_ref().unwrap().summary,
        Some("New summary".to_string())
    );
}

#[tokio::test]
async fn process_update_assigns_to_me() {
    let client = make_mock("my-account-id", vec![]);

    let args = UpdateArgs {
        assign: Some("me".to_string()),
        ..empty_args("X-1")
    };

    let output = process_update(&client, &args).await.unwrap();
    assert!(output.contains("Updated assignee"));

    let updated = client.updated_fields.lock().unwrap();
    assert_eq!(
        updated.as_ref().unwrap().assignee,
        Some("my-account-id".to_string())
    );
}

#[tokio::test]
async fn process_update_assigns_to_user() {
    let client = make_mock("me", vec![]);

    let args = UpdateArgs {
        assign: Some("other-user-123".to_string()),
        ..empty_args("X-1")
    };

    let output = process_update(&client, &args).await.unwrap();
    assert!(output.contains("Updated assignee"));

    let updated = client.updated_fields.lock().unwrap();
    assert_eq!(
        updated.as_ref().unwrap().assignee,
        Some("other-user-123".to_string())
    );
}

#[tokio::test]
async fn process_update_transitions_status() {
    let client = make_mock(
        "me",
        vec![
            Transition {
                id: "11".to_string(),
                name: "To Do".to_string(),
            },
            Transition {
                id: "21".to_string(),
                name: "In Progress".to_string(),
            },
            Transition {
                id: "31".to_string(),
                name: "Done".to_string(),
            },
        ],
    );

    let args = UpdateArgs {
        status: Some("Done".to_string()),
        ..empty_args("X-1")
    };

    let output = process_update(&client, &args).await.unwrap();
    assert!(output.contains("Transitioned to: Done"));

    let transitioned = client.transitioned_to.lock().unwrap();
    assert_eq!(transitioned.as_ref().unwrap(), "31");
}

#[tokio::test]
async fn process_update_fails_no_changes() {
    let client = make_mock("me", vec![]);
    let args = empty_args("X-1");

    let result = process_update(&client, &args).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("No changes specified"));
}

#[tokio::test]
async fn process_update_multiple_changes() {
    let client = make_mock(
        "me123",
        vec![Transition {
            id: "31".to_string(),
            name: "Done".to_string(),
        }],
    );

    let args = UpdateArgs {
        summary: Some("Updated".to_string()),
        status: Some("Done".to_string()),
        assign: Some("me".to_string()),
        ..empty_args("X-1")
    };

    let output = process_update(&client, &args).await.unwrap();
    assert!(output.contains("Updated summary"));
    assert!(output.contains("Updated assignee"));
    assert!(output.contains("Transitioned to: Done"));
}

#[tokio::test]
async fn process_update_body_adf_passes_through() {
    let client = make_mock("me", vec![]);

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

    let args = UpdateArgs {
        body_adf: Some(file.path().to_path_buf()),
        ..empty_args("X-1")
    };

    let output = process_update(&client, &args).await.unwrap();
    assert!(output.contains("Updated description (raw ADF)"));

    let updated = client.updated_fields.lock().unwrap();
    let captured = updated.as_ref().unwrap().description_adf.as_ref().unwrap();
    assert_eq!(captured["type"], "doc");
    assert_eq!(captured["content"][0]["content"][0]["text"], "raw");
}

#[tokio::test]
async fn process_update_body_adf_missing_file_fails_before_network() {
    let client = make_mock("me", vec![]);

    let args = UpdateArgs {
        body_adf: Some(std::path::PathBuf::from("/nonexistent/adf.json")),
        ..empty_args("X-1")
    };

    let result = process_update(&client, &args).await;
    assert!(result.is_err());
    // Mock should not have been touched.
    assert!(client.updated_fields.lock().unwrap().is_none());
}
