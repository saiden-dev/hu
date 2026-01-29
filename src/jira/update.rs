use anyhow::{bail, Result};

use super::client::{JiraApi, JiraClient};
use super::types::{IssueUpdate, Transition};

/// Arguments for update command
#[derive(Debug, Clone)]
pub struct UpdateArgs {
    pub key: String,
    pub summary: Option<String>,
    pub status: Option<String>,
    pub assign: Option<String>,
}

/// Run the jira update command
pub async fn run(args: UpdateArgs) -> Result<()> {
    let client = JiraClient::new().await?;
    let output = process_update(&client, &args).await?;
    print!("{}", output);
    Ok(())
}

/// Process update command (business logic, testable)
pub async fn process_update(client: &impl JiraApi, args: &UpdateArgs) -> Result<String> {
    let mut output = String::new();
    let mut changes_made = false;

    // Handle field updates
    let has_field_updates = args.summary.is_some() || args.assign.is_some();
    if has_field_updates {
        let assignee = match &args.assign {
            Some(a) if a == "me" => {
                let user = client.get_current_user().await?;
                Some(user.account_id)
            }
            Some(a) => Some(a.clone()),
            None => None,
        };

        let update = IssueUpdate {
            summary: args.summary.clone(),
            description: None,
            assignee,
        };

        client.update_issue(&args.key, &update).await?;
        changes_made = true;

        if let Some(summary) = &args.summary {
            output.push_str(&format!(
                "\x1b[32m\u{2713}\x1b[0m Updated summary: \"{}\"\n",
                summary
            ));
        }
        if args.assign.is_some() {
            output.push_str("\x1b[32m\u{2713}\x1b[0m Updated assignee\n");
        }
    }

    // Handle status transition
    if let Some(target_status) = &args.status {
        let transitions = client.get_transitions(&args.key).await?;
        let transition = find_transition(&transitions, target_status)?;

        client.transition_issue(&args.key, &transition.id).await?;
        changes_made = true;

        output.push_str(&format!(
            "\x1b[32m\u{2713}\x1b[0m Transitioned to: {}\n",
            transition.name
        ));
    }

    if !changes_made {
        bail!("No changes specified. Use --summary, --status, or --assign.");
    }

    Ok(output)
}

/// Find a transition by name (case-insensitive)
fn find_transition<'a>(transitions: &'a [Transition], target: &str) -> Result<&'a Transition> {
    let target_lower = target.to_lowercase();

    // Exact match first
    if let Some(t) = transitions
        .iter()
        .find(|t| t.name.to_lowercase() == target_lower)
    {
        return Ok(t);
    }

    // Partial match
    if let Some(t) = transitions
        .iter()
        .find(|t| t.name.to_lowercase().contains(&target_lower))
    {
        return Ok(t);
    }

    // Build error message with available transitions
    let available: Vec<_> = transitions.iter().map(|t| t.name.as_str()).collect();
    bail!(
        "Status '{}' not found. Available transitions: {}",
        target,
        available.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::super::types::{Board, Issue, Sprint, User};
    use super::*;

    #[test]
    fn update_args_debug() {
        let args = UpdateArgs {
            key: "X-1".to_string(),
            summary: Some("New".to_string()),
            status: None,
            assign: None,
        };
        let debug_str = format!("{:?}", args);
        assert!(debug_str.contains("UpdateArgs"));
    }

    #[test]
    fn update_args_clone() {
        let args = UpdateArgs {
            key: "X-1".to_string(),
            summary: Some("S".to_string()),
            status: Some("Done".to_string()),
            assign: Some("user".to_string()),
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

        async fn get_boards(&self) -> Result<Vec<Board>> {
            unimplemented!()
        }

        async fn get_active_sprint(&self, _board_id: u64) -> Result<Option<Sprint>> {
            unimplemented!()
        }

        async fn get_sprint_issues(&self, _sprint_id: u64) -> Result<Vec<Issue>> {
            unimplemented!()
        }

        async fn get_issue(&self, _key: &str) -> Result<Issue> {
            unimplemented!()
        }

        async fn search_issues(&self, _jql: &str) -> Result<Vec<Issue>> {
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
    }

    #[tokio::test]
    async fn process_update_changes_summary() {
        let client = MockJiraClient {
            user: User {
                account_id: "me123".to_string(),
                display_name: "Me".to_string(),
                email_address: None,
            },
            transitions: vec![],
            updated_fields: std::sync::Mutex::new(None),
            transitioned_to: std::sync::Mutex::new(None),
        };

        let args = UpdateArgs {
            key: "X-1".to_string(),
            summary: Some("New summary".to_string()),
            status: None,
            assign: None,
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
        let client = MockJiraClient {
            user: User {
                account_id: "my-account-id".to_string(),
                display_name: "Me".to_string(),
                email_address: None,
            },
            transitions: vec![],
            updated_fields: std::sync::Mutex::new(None),
            transitioned_to: std::sync::Mutex::new(None),
        };

        let args = UpdateArgs {
            key: "X-1".to_string(),
            summary: None,
            status: None,
            assign: Some("me".to_string()),
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
        let client = MockJiraClient {
            user: User {
                account_id: "me".to_string(),
                display_name: "Me".to_string(),
                email_address: None,
            },
            transitions: vec![],
            updated_fields: std::sync::Mutex::new(None),
            transitioned_to: std::sync::Mutex::new(None),
        };

        let args = UpdateArgs {
            key: "X-1".to_string(),
            summary: None,
            status: None,
            assign: Some("other-user-123".to_string()),
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
        let client = MockJiraClient {
            user: User {
                account_id: "me".to_string(),
                display_name: "Me".to_string(),
                email_address: None,
            },
            transitions: vec![
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
            updated_fields: std::sync::Mutex::new(None),
            transitioned_to: std::sync::Mutex::new(None),
        };

        let args = UpdateArgs {
            key: "X-1".to_string(),
            summary: None,
            status: Some("Done".to_string()),
            assign: None,
        };

        let output = process_update(&client, &args).await.unwrap();
        assert!(output.contains("Transitioned to: Done"));

        let transitioned = client.transitioned_to.lock().unwrap();
        assert_eq!(transitioned.as_ref().unwrap(), "31");
    }

    #[tokio::test]
    async fn process_update_fails_no_changes() {
        let client = MockJiraClient {
            user: User {
                account_id: "me".to_string(),
                display_name: "Me".to_string(),
                email_address: None,
            },
            transitions: vec![],
            updated_fields: std::sync::Mutex::new(None),
            transitioned_to: std::sync::Mutex::new(None),
        };

        let args = UpdateArgs {
            key: "X-1".to_string(),
            summary: None,
            status: None,
            assign: None,
        };

        let result = process_update(&client, &args).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No changes specified"));
    }

    #[tokio::test]
    async fn process_update_multiple_changes() {
        let client = MockJiraClient {
            user: User {
                account_id: "me123".to_string(),
                display_name: "Me".to_string(),
                email_address: None,
            },
            transitions: vec![Transition {
                id: "31".to_string(),
                name: "Done".to_string(),
            }],
            updated_fields: std::sync::Mutex::new(None),
            transitioned_to: std::sync::Mutex::new(None),
        };

        let args = UpdateArgs {
            key: "X-1".to_string(),
            summary: Some("Updated".to_string()),
            status: Some("Done".to_string()),
            assign: Some("me".to_string()),
        };

        let output = process_update(&client, &args).await.unwrap();
        assert!(output.contains("Updated summary"));
        assert!(output.contains("Updated assignee"));
        assert!(output.contains("Transitioned to: Done"));
    }
}
