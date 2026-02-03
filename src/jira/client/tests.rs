use super::*;
use serde_json::json;

#[test]
fn parse_user_extracts_fields() {
    let json = json!({
        "accountId": "123",
        "displayName": "John Doe",
        "emailAddress": "john@example.com"
    });
    let user = parse_user(&json).unwrap();
    assert_eq!(user.account_id, "123");
    assert_eq!(user.display_name, "John Doe");
    assert_eq!(user.email_address, Some("john@example.com".to_string()));
}

#[test]
fn parse_user_without_email() {
    let json = json!({
        "accountId": "456",
        "displayName": "Jane"
    });
    let user = parse_user(&json).unwrap();
    assert_eq!(user.account_id, "456");
    assert!(user.email_address.is_none());
}

#[test]
fn parse_user_returns_none_for_missing_fields() {
    let json = json!({
        "displayName": "Missing ID"
    });
    let user = parse_user(&json);
    assert!(user.is_none());
}

#[test]
fn parse_issues_extracts_issues() {
    let json = json!({
        "issues": [{
            "key": "PROJ-123",
            "fields": {
                "summary": "Fix bug",
                "status": {"name": "In Progress"},
                "issuetype": {"name": "Bug"},
                "assignee": {"displayName": "John"},
                "updated": "2024-01-15T10:00:00Z"
            }
        }]
    });
    let issues = parse_issues(&json);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].key, "PROJ-123");
    assert_eq!(issues[0].summary, "Fix bug");
    assert_eq!(issues[0].status, "In Progress");
    assert_eq!(issues[0].issue_type, "Bug");
    assert_eq!(issues[0].assignee, Some("John".to_string()));
}

#[test]
fn parse_issues_handles_unassigned() {
    let json = json!({
        "issues": [{
            "key": "PROJ-456",
            "fields": {
                "summary": "Task",
                "status": {"name": "Open"},
                "issuetype": {"name": "Task"},
                "assignee": null,
                "updated": "2024-01-15T12:00:00Z"
            }
        }]
    });
    let issues = parse_issues(&json);
    assert_eq!(issues.len(), 1);
    assert!(issues[0].assignee.is_none());
}

#[test]
fn parse_issues_handles_empty() {
    let json = json!({"issues": []});
    let issues = parse_issues(&json);
    assert!(issues.is_empty());
}

#[test]
fn parse_single_issue_extracts_fields() {
    let json = json!({
        "key": "TEST-1",
        "fields": {
            "summary": "Test issue",
            "status": {"name": "Done"},
            "issuetype": {"name": "Story"},
            "assignee": {"displayName": "Tester"},
            "description": {
                "type": "doc",
                "content": [{
                    "type": "paragraph",
                    "content": [{"type": "text", "text": "Description text"}]
                }]
            },
            "updated": "2024-01-01T00:00:00Z"
        }
    });
    let issue = parse_single_issue(&json).unwrap();
    assert_eq!(issue.key, "TEST-1");
    assert_eq!(issue.summary, "Test issue");
    assert_eq!(issue.status, "Done");
    assert_eq!(issue.issue_type, "Story");
    assert_eq!(issue.assignee, Some("Tester".to_string()));
    assert_eq!(issue.description, Some("Description text".to_string()));
}

#[test]
fn parse_single_issue_returns_none_for_missing_key() {
    let json = json!({
        "fields": {
            "summary": "No key",
            "status": {"name": "Open"},
            "issuetype": {"name": "Task"},
            "updated": "2024-01-01T00:00:00Z"
        }
    });
    let issue = parse_single_issue(&json);
    assert!(issue.is_none());
}

#[test]
fn parse_single_issue_handles_null_description() {
    let json = json!({
        "key": "X-1",
        "fields": {
            "summary": "S",
            "status": {"name": "Open"},
            "issuetype": {"name": "Task"},
            "description": null,
            "updated": "2024-01-01T00:00:00Z"
        }
    });
    let issue = parse_single_issue(&json).unwrap();
    assert!(issue.description.is_none());
}

#[test]
fn extract_description_handles_string() {
    let fields = json!({"description": "Simple string"});
    let desc = extract_description(&fields);
    assert_eq!(desc, Some("Simple string".to_string()));
}

#[test]
fn extract_description_handles_adf() {
    let fields = json!({
        "description": {
            "type": "doc",
            "content": [{
                "type": "paragraph",
                "content": [
                    {"type": "text", "text": "Hello "},
                    {"type": "text", "text": "world"}
                ]
            }]
        }
    });
    let desc = extract_description(&fields);
    assert_eq!(desc, Some("Hello world".to_string()));
}

#[test]
fn extract_description_handles_null() {
    let fields = json!({"description": null});
    let desc = extract_description(&fields);
    assert!(desc.is_none());
}

#[test]
fn extract_description_handles_empty_content() {
    let fields = json!({
        "description": {
            "type": "doc",
            "content": []
        }
    });
    let desc = extract_description(&fields);
    assert!(desc.is_none());
}

#[test]
fn extract_text_from_adf_node_gets_text() {
    let node = json!({"type": "text", "text": "Hello"});
    let text = extract_text_from_adf_node(&node);
    assert_eq!(text, Some("Hello".to_string()));
}

#[test]
fn extract_text_from_adf_node_recurses() {
    let node = json!({
        "type": "paragraph",
        "content": [
            {"type": "text", "text": "A"},
            {"type": "text", "text": "B"}
        ]
    });
    let text = extract_text_from_adf_node(&node);
    assert_eq!(text, Some("AB".to_string()));
}

#[test]
fn extract_text_from_adf_node_handles_no_content() {
    let node = json!({"type": "hardBreak"});
    let text = extract_text_from_adf_node(&node);
    assert!(text.is_none());
}

#[test]
fn parse_transitions_extracts_transitions() {
    let json = json!({
        "transitions": [
            {"id": "11", "name": "To Do"},
            {"id": "21", "name": "In Progress"},
            {"id": "31", "name": "Done"}
        ]
    });
    let transitions = parse_transitions(&json);
    assert_eq!(transitions.len(), 3);
    assert_eq!(transitions[0].id, "11");
    assert_eq!(transitions[0].name, "To Do");
    assert_eq!(transitions[2].id, "31");
    assert_eq!(transitions[2].name, "Done");
}

#[test]
fn parse_transitions_handles_empty() {
    let json = json!({"transitions": []});
    let transitions = parse_transitions(&json);
    assert!(transitions.is_empty());
}

#[test]
fn parse_transitions_handles_missing() {
    let json = json!({});
    let transitions = parse_transitions(&json);
    assert!(transitions.is_empty());
}

#[test]
fn build_update_body_with_summary() {
    let update = IssueUpdate {
        summary: Some("New summary".to_string()),
        description: None,
        assignee: None,
    };
    let body = build_update_body(&update);
    assert_eq!(body["fields"]["summary"], "New summary");
}

#[test]
fn build_update_body_with_description() {
    let update = IssueUpdate {
        summary: None,
        description: Some("New description".to_string()),
        assignee: None,
    };
    let body = build_update_body(&update);
    assert_eq!(body["fields"]["description"]["type"], "doc");
    assert_eq!(body["fields"]["description"]["version"], 1);
}

#[test]
fn build_update_body_with_assignee() {
    let update = IssueUpdate {
        summary: None,
        description: None,
        assignee: Some("user123".to_string()),
    };
    let body = build_update_body(&update);
    assert_eq!(body["fields"]["assignee"]["accountId"], "user123");
}

#[test]
fn build_update_body_with_all_fields() {
    let update = IssueUpdate {
        summary: Some("Sum".to_string()),
        description: Some("Desc".to_string()),
        assignee: Some("user".to_string()),
    };
    let body = build_update_body(&update);
    assert_eq!(body["fields"]["summary"], "Sum");
    assert!(body["fields"]["description"].is_object());
    assert_eq!(body["fields"]["assignee"]["accountId"], "user");
}

#[test]
fn build_update_body_empty() {
    let update = IssueUpdate::default();
    let body = build_update_body(&update);
    assert_eq!(body["fields"], json!({}));
}
