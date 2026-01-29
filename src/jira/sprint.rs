use anyhow::{bail, Context, Result};

use super::client::{JiraApi, JiraClient};
use super::types::{Board, Issue, Sprint};

/// Arguments for sprint command
#[derive(Debug, Clone)]
pub struct SprintArgs {
    pub board: Option<u64>,
}

/// Run the jira sprint command
pub async fn run(args: SprintArgs) -> Result<()> {
    let client = JiraClient::new().await?;
    let output = process_sprint(&client, args.board).await?;
    print!("{}", output);
    Ok(())
}

/// Process sprint command (business logic, testable)
pub async fn process_sprint(client: &impl JiraApi, board_id: Option<u64>) -> Result<String> {
    // Get board ID (auto-detect if not provided)
    let board = if let Some(id) = board_id {
        Board {
            id,
            name: String::new(),
            board_type: String::new(),
        }
    } else {
        let boards = client.get_boards().await?;
        if boards.is_empty() {
            bail!("No boards found. Make sure you have access to at least one Jira board.");
        }
        if boards.len() > 1 {
            return Ok(format_board_selection(&boards));
        }
        boards.into_iter().next().unwrap()
    };

    // Get active sprint
    let sprint = client
        .get_active_sprint(board.id)
        .await?
        .context("No active sprint found")?;

    // Get sprint issues
    let issues = client.get_sprint_issues(sprint.id).await?;

    Ok(format_sprint_output(&sprint, &issues))
}

/// Format output when multiple boards exist
fn format_board_selection(boards: &[Board]) -> String {
    let mut output = String::new();
    output.push_str("Multiple boards found. Please specify a board with --board:\n\n");
    for board in boards {
        output.push_str(&format!(
            "  --board {}  # {} ({})\n",
            board.id, board.name, board.board_type
        ));
    }
    output
}

/// Format sprint output
fn format_sprint_output(sprint: &Sprint, issues: &[Issue]) -> String {
    let mut output = String::new();

    // Sprint header
    output.push_str(&format!(
        "\x1b[1m{}\x1b[0m ({})\n",
        sprint.name, sprint.state
    ));
    if let (Some(start), Some(end)) = (&sprint.start_date, &sprint.end_date) {
        output.push_str(&format!("{} - {}\n", format_date(start), format_date(end)));
    }
    output.push('\n');

    if issues.is_empty() {
        output.push_str("No issues in sprint\n");
        return output;
    }

    // Group by status
    let mut by_status: std::collections::HashMap<&str, Vec<&Issue>> =
        std::collections::HashMap::new();
    for issue in issues {
        by_status.entry(&issue.status).or_default().push(issue);
    }

    // Status order preference
    let status_order = ["To Do", "In Progress", "In Review", "Done"];

    // Output in order, then any remaining
    for status in &status_order {
        if let Some(issues) = by_status.remove(*status) {
            output.push_str(&format_status_section(status, &issues));
        }
    }

    // Remaining statuses
    let mut remaining: Vec<_> = by_status.into_iter().collect();
    remaining.sort_by_key(|(status, _)| *status);
    for (status, issues) in remaining {
        output.push_str(&format_status_section(status, &issues));
    }

    output
}

/// Format a status section
fn format_status_section(status: &str, issues: &[&Issue]) -> String {
    let mut output = String::new();
    let status_color = match status {
        "Done" => "\x1b[32m",        // green
        "In Progress" => "\x1b[33m", // yellow
        _ => "\x1b[34m",             // blue
    };
    output.push_str(&format!(
        "{}{}\x1b[0m ({})\n",
        status_color,
        status,
        issues.len()
    ));

    for issue in issues {
        let assignee = issue.assignee.as_deref().unwrap_or("Unassigned");
        output.push_str(&format!(
            "  {} {} \x1b[90m({})\x1b[0m\n",
            issue.key, issue.summary, assignee
        ));
    }
    output.push('\n');
    output
}

/// Format date string (extract date part only)
fn format_date(date: &str) -> &str {
    // ISO dates are like "2024-01-15T10:00:00.000Z"
    date.split('T').next().unwrap_or(date)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprint_args_debug() {
        let args = SprintArgs { board: Some(42) };
        let debug_str = format!("{:?}", args);
        assert!(debug_str.contains("SprintArgs"));
    }

    #[test]
    fn sprint_args_clone() {
        let args = SprintArgs { board: Some(42) };
        let cloned = args.clone();
        assert_eq!(cloned.board, args.board);
    }

    #[test]
    fn format_date_extracts_date_part() {
        assert_eq!(format_date("2024-01-15T10:00:00.000Z"), "2024-01-15");
        assert_eq!(format_date("2024-01-15"), "2024-01-15");
    }

    #[test]
    fn format_date_handles_empty() {
        assert_eq!(format_date(""), "");
    }

    #[test]
    fn format_board_selection_lists_boards() {
        let boards = vec![
            Board {
                id: 1,
                name: "Board One".to_string(),
                board_type: "scrum".to_string(),
            },
            Board {
                id: 2,
                name: "Board Two".to_string(),
                board_type: "kanban".to_string(),
            },
        ];
        let output = format_board_selection(&boards);
        assert!(output.contains("Multiple boards found"));
        assert!(output.contains("--board 1"));
        assert!(output.contains("Board One"));
        assert!(output.contains("scrum"));
        assert!(output.contains("--board 2"));
        assert!(output.contains("Board Two"));
        assert!(output.contains("kanban"));
    }

    #[test]
    fn format_sprint_output_shows_header() {
        let sprint = Sprint {
            id: 1,
            name: "Sprint 1".to_string(),
            state: "active".to_string(),
            start_date: Some("2024-01-01".to_string()),
            end_date: Some("2024-01-14".to_string()),
        };
        let issues = vec![];
        let output = format_sprint_output(&sprint, &issues);
        assert!(output.contains("Sprint 1"));
        assert!(output.contains("active"));
        assert!(output.contains("2024-01-01"));
        assert!(output.contains("2024-01-14"));
        assert!(output.contains("No issues in sprint"));
    }

    #[test]
    fn format_sprint_output_groups_by_status() {
        let sprint = Sprint {
            id: 1,
            name: "Sprint".to_string(),
            state: "active".to_string(),
            start_date: None,
            end_date: None,
        };
        let issues = vec![
            Issue {
                key: "A-1".to_string(),
                summary: "Task 1".to_string(),
                status: "To Do".to_string(),
                issue_type: "Task".to_string(),
                assignee: Some("Alice".to_string()),
                description: None,
                updated: "2024-01-01T00:00:00Z".to_string(),
            },
            Issue {
                key: "A-2".to_string(),
                summary: "Task 2".to_string(),
                status: "In Progress".to_string(),
                issue_type: "Task".to_string(),
                assignee: Some("Bob".to_string()),
                description: None,
                updated: "2024-01-01T00:00:00Z".to_string(),
            },
            Issue {
                key: "A-3".to_string(),
                summary: "Task 3".to_string(),
                status: "Done".to_string(),
                issue_type: "Task".to_string(),
                assignee: None,
                description: None,
                updated: "2024-01-01T00:00:00Z".to_string(),
            },
        ];
        let output = format_sprint_output(&sprint, &issues);
        assert!(output.contains("A-1"));
        assert!(output.contains("Task 1"));
        assert!(output.contains("Alice"));
        assert!(output.contains("A-2"));
        assert!(output.contains("Bob"));
        assert!(output.contains("A-3"));
        assert!(output.contains("Unassigned"));
    }

    #[test]
    fn format_status_section_shows_count() {
        let issue1 = Issue {
            key: "X-1".to_string(),
            summary: "S1".to_string(),
            status: "Open".to_string(),
            issue_type: "T".to_string(),
            assignee: None,
            description: None,
            updated: "U".to_string(),
        };
        let issue2 = Issue {
            key: "X-2".to_string(),
            summary: "S2".to_string(),
            status: "Open".to_string(),
            issue_type: "T".to_string(),
            assignee: Some("User".to_string()),
            description: None,
            updated: "U".to_string(),
        };
        let issues = vec![&issue1, &issue2];
        let output = format_status_section("Open", &issues);
        assert!(output.contains("Open"));
        assert!(output.contains("(2)"));
        assert!(output.contains("X-1"));
        assert!(output.contains("X-2"));
    }

    #[test]
    fn format_status_section_color_codes() {
        let empty: Vec<&Issue> = vec![];
        let done_output = format_status_section("Done", &empty);
        assert!(done_output.contains("\x1b[32m")); // green

        let progress_output = format_status_section("In Progress", &empty);
        assert!(progress_output.contains("\x1b[33m")); // yellow

        let other_output = format_status_section("Other", &empty);
        assert!(other_output.contains("\x1b[34m")); // blue
    }

    use super::super::types::{IssueUpdate, Transition, User};

    // Mock client for testing process_sprint
    struct MockJiraClient {
        boards: Vec<Board>,
        sprint: Option<Sprint>,
        issues: Vec<Issue>,
    }

    impl JiraApi for MockJiraClient {
        async fn get_current_user(&self) -> Result<User> {
            unimplemented!()
        }

        async fn get_boards(&self) -> Result<Vec<Board>> {
            Ok(self.boards.clone())
        }

        async fn get_active_sprint(&self, _board_id: u64) -> Result<Option<Sprint>> {
            Ok(self.sprint.clone())
        }

        async fn get_sprint_issues(&self, _sprint_id: u64) -> Result<Vec<Issue>> {
            Ok(self.issues.clone())
        }

        async fn get_issue(&self, _key: &str) -> Result<Issue> {
            unimplemented!()
        }

        async fn search_issues(&self, _jql: &str) -> Result<Vec<Issue>> {
            unimplemented!()
        }

        async fn update_issue(&self, _key: &str, _update: &IssueUpdate) -> Result<()> {
            unimplemented!()
        }

        async fn get_transitions(&self, _key: &str) -> Result<Vec<Transition>> {
            unimplemented!()
        }

        async fn transition_issue(&self, _key: &str, _transition_id: &str) -> Result<()> {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn process_sprint_auto_detects_single_board() {
        let client = MockJiraClient {
            boards: vec![Board {
                id: 42,
                name: "Main".to_string(),
                board_type: "scrum".to_string(),
            }],
            sprint: Some(Sprint {
                id: 100,
                name: "Sprint 1".to_string(),
                state: "active".to_string(),
                start_date: None,
                end_date: None,
            }),
            issues: vec![],
        };

        let output = process_sprint(&client, None).await.unwrap();
        assert!(output.contains("Sprint 1"));
    }

    #[tokio::test]
    async fn process_sprint_shows_board_selection_for_multiple() {
        let client = MockJiraClient {
            boards: vec![
                Board {
                    id: 1,
                    name: "B1".to_string(),
                    board_type: "scrum".to_string(),
                },
                Board {
                    id: 2,
                    name: "B2".to_string(),
                    board_type: "kanban".to_string(),
                },
            ],
            sprint: None,
            issues: vec![],
        };

        let output = process_sprint(&client, None).await.unwrap();
        assert!(output.contains("Multiple boards found"));
        assert!(output.contains("--board 1"));
        assert!(output.contains("--board 2"));
    }

    #[tokio::test]
    async fn process_sprint_uses_specified_board() {
        let client = MockJiraClient {
            boards: vec![],
            sprint: Some(Sprint {
                id: 200,
                name: "Sprint 2".to_string(),
                state: "active".to_string(),
                start_date: None,
                end_date: None,
            }),
            issues: vec![],
        };

        let output = process_sprint(&client, Some(99)).await.unwrap();
        assert!(output.contains("Sprint 2"));
    }

    #[tokio::test]
    async fn process_sprint_fails_no_boards() {
        let client = MockJiraClient {
            boards: vec![],
            sprint: None,
            issues: vec![],
        };

        let result = process_sprint(&client, None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No boards found"));
    }

    #[tokio::test]
    async fn process_sprint_fails_no_active_sprint() {
        let client = MockJiraClient {
            boards: vec![Board {
                id: 1,
                name: "B".to_string(),
                board_type: "s".to_string(),
            }],
            sprint: None,
            issues: vec![],
        };

        let result = process_sprint(&client, None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No active sprint"));
    }
}
