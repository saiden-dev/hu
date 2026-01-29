use anyhow::{bail, Context, Result};

use super::client::{JiraApi, JiraClient};
use super::types::{Board, Issue, Sprint};

// ANSI color codes
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const BLUE: &str = "\x1b[34m";
const GRAY: &str = "\x1b[90m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

/// Run the jira tickets command (list current sprint tickets)
pub async fn run(board_id: Option<u64>) -> Result<()> {
    let client = JiraClient::new().await?;
    let output = process_tickets(&client, board_id).await?;
    print!("{}", output);
    Ok(())
}

/// Process tickets command (business logic, testable)
pub async fn process_tickets(client: &impl JiraApi, board_id: Option<u64>) -> Result<String> {
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

    Ok(format_tickets(&sprint, &issues))
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

fn get_terminal_width() -> usize {
    terminal_size::terminal_size()
        .map(|(w, _)| w.0 as usize)
        .unwrap_or(120)
}

/// Format tickets as a table
fn format_tickets(sprint: &Sprint, issues: &[Issue]) -> String {
    let mut output = String::new();
    let term_width = get_terminal_width();

    // Sprint header
    output.push_str(&format!(
        "{}{}{} ({} issues)\n\n",
        BOLD,
        sprint.name,
        RESET,
        issues.len()
    ));

    if issues.is_empty() {
        output.push_str("No tickets in sprint\n");
        return output;
    }

    // Calculate column widths based on content
    let key_width = issues
        .iter()
        .map(|i| i.key.chars().count())
        .max()
        .unwrap_or(4)
        .max(4);
    let status_width = issues
        .iter()
        .map(|i| i.status.chars().count())
        .max()
        .unwrap_or(6)
        .max(6);
    let type_width = issues
        .iter()
        .map(|i| i.issue_type.chars().count())
        .max()
        .unwrap_or(4)
        .max(4);
    let assignee_width = issues
        .iter()
        .map(|i| i.assignee.as_ref().map(|a| a.chars().count()).unwrap_or(1))
        .max()
        .unwrap_or(8)
        .clamp(8, 20); // Cap assignee at 20

    // Layout: │ Key │ Status │ Type │ Assignee │ Summary │
    // Borders take: 6 separators × 3 chars = 18 chars
    let border_overhead = 18;
    let fixed_cols = key_width + status_width + type_width + assignee_width;
    let available_for_summary = term_width
        .saturating_sub(border_overhead + fixed_cols)
        .max(20);

    // Top border
    output.push_str(&format!(
        "┌{}┬{}┬{}┬{}┬{}┐\n",
        "─".repeat(key_width + 2),
        "─".repeat(status_width + 2),
        "─".repeat(type_width + 2),
        "─".repeat(assignee_width + 2),
        "─".repeat(available_for_summary + 2)
    ));

    // Header row
    output.push_str(&format!(
        "│ {}{:<key_w$}{} │ {}{:<status_w$}{} │ {}{:<type_w$}{} │ {}{:<assign_w$}{} │ {}{:<sum_w$}{} │\n",
        BOLD, "Key", RESET,
        BOLD, "Status", RESET,
        BOLD, "Type", RESET,
        BOLD, "Assignee", RESET,
        BOLD, "Summary", RESET,
        key_w = key_width,
        status_w = status_width,
        type_w = type_width,
        assign_w = assignee_width,
        sum_w = available_for_summary,
    ));

    // Header separator
    output.push_str(&format!(
        "├{}┼{}┼{}┼{}┼{}┤\n",
        "─".repeat(key_width + 2),
        "─".repeat(status_width + 2),
        "─".repeat(type_width + 2),
        "─".repeat(assignee_width + 2),
        "─".repeat(available_for_summary + 2)
    ));

    // Data rows
    for issue in issues {
        let status_color = match issue.status.as_str() {
            "Done" => GREEN,
            "In Progress" | "In Review" | "CODE REVIEW" => YELLOW,
            _ => BLUE,
        };

        let assignee = issue.assignee.as_deref().unwrap_or("-");
        let assignee_display = truncate(assignee, assignee_width);
        let summary_display = truncate(&issue.summary, available_for_summary);

        output.push_str(&format!(
            "│ {:<key_w$} │ {}{:<status_w$}{} │ {:<type_w$} │ {}{:<assign_w$}{} │ {:<sum_w$} │\n",
            issue.key,
            status_color,
            truncate(&issue.status, status_width),
            RESET,
            truncate(&issue.issue_type, type_width),
            GRAY,
            assignee_display,
            RESET,
            summary_display,
            key_w = key_width,
            status_w = status_width,
            type_w = type_width,
            assign_w = assignee_width,
            sum_w = available_for_summary,
        ));
    }

    // Bottom border
    output.push_str(&format!(
        "└{}┴{}┴{}┴{}┴{}┘\n",
        "─".repeat(key_width + 2),
        "─".repeat(status_width + 2),
        "─".repeat(type_width + 2),
        "─".repeat(assignee_width + 2),
        "─".repeat(available_for_summary + 2)
    ));

    output
}

/// Truncate string to max length with ellipsis
fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len.saturating_sub(1)).collect();
        format!("{}…", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::{IssueUpdate, Transition, User};
    use super::*;

    #[test]
    fn truncate_short_unchanged() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_long_adds_ellipsis() {
        assert_eq!(truncate("hello world", 8), "hello w…");
    }

    #[test]
    fn truncate_unicode() {
        assert_eq!(truncate("héllo", 5), "héllo");
        assert_eq!(truncate("héllo world", 6), "héllo…");
    }

    #[test]
    fn truncate_exact_length() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn get_terminal_width_returns_reasonable_value() {
        let width = get_terminal_width();
        assert!(width >= 20);
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
        assert!(output.contains("--board 2"));
    }

    #[test]
    fn format_tickets_empty() {
        let sprint = Sprint {
            id: 1,
            name: "Sprint 1".to_string(),
            state: "active".to_string(),
            start_date: None,
            end_date: None,
        };
        let issues: Vec<Issue> = vec![];
        let output = format_tickets(&sprint, &issues);
        assert!(output.contains("Sprint 1"));
        assert!(output.contains("0 issues"));
        assert!(output.contains("No tickets"));
    }

    #[test]
    fn format_tickets_with_issues() {
        let sprint = Sprint {
            id: 1,
            name: "Sprint 1".to_string(),
            state: "active".to_string(),
            start_date: None,
            end_date: None,
        };
        let issues = vec![
            Issue {
                key: "A-1".to_string(),
                summary: "First task".to_string(),
                status: "Done".to_string(),
                issue_type: "Task".to_string(),
                assignee: Some("Alice".to_string()),
                description: None,
                updated: "U".to_string(),
            },
            Issue {
                key: "A-2".to_string(),
                summary: "Second task".to_string(),
                status: "In Progress".to_string(),
                issue_type: "Bug".to_string(),
                assignee: None,
                description: None,
                updated: "U".to_string(),
            },
        ];
        let output = format_tickets(&sprint, &issues);
        assert!(output.contains("Sprint 1"));
        assert!(output.contains("2 issues"));
        assert!(output.contains("A-1"));
        assert!(output.contains("A-2"));
        assert!(output.contains("First task"));
        assert!(output.contains("Second task"));
        assert!(output.contains("Task"));
        assert!(output.contains("Bug"));
        assert!(output.contains("Alice"));
        // Box-drawing characters
        assert!(output.contains("┌"));
        assert!(output.contains("┐"));
        assert!(output.contains("└"));
        assert!(output.contains("┘"));
        assert!(output.contains("│"));
        assert!(output.contains("─"));
    }

    #[test]
    fn format_tickets_colors_status() {
        let sprint = Sprint {
            id: 1,
            name: "S".to_string(),
            state: "active".to_string(),
            start_date: None,
            end_date: None,
        };
        let issues = vec![
            Issue {
                key: "X-1".to_string(),
                summary: "S".to_string(),
                status: "Done".to_string(),
                issue_type: "T".to_string(),
                assignee: None,
                description: None,
                updated: "U".to_string(),
            },
            Issue {
                key: "X-2".to_string(),
                summary: "S".to_string(),
                status: "In Progress".to_string(),
                issue_type: "T".to_string(),
                assignee: None,
                description: None,
                updated: "U".to_string(),
            },
            Issue {
                key: "X-3".to_string(),
                summary: "S".to_string(),
                status: "To Do".to_string(),
                issue_type: "T".to_string(),
                assignee: None,
                description: None,
                updated: "U".to_string(),
            },
        ];
        let output = format_tickets(&sprint, &issues);
        assert!(output.contains(GREEN)); // Done
        assert!(output.contains(YELLOW)); // In Progress
        assert!(output.contains(BLUE)); // To Do
    }

    #[test]
    fn format_tickets_handles_long_summary() {
        let sprint = Sprint {
            id: 1,
            name: "Sprint".to_string(),
            state: "active".to_string(),
            start_date: None,
            end_date: None,
        };
        let issues = vec![Issue {
            key: "LONG-123".to_string(),
            summary: "This is a very long summary that should be truncated to fit within the terminal width appropriately".to_string(),
            status: "Open".to_string(),
            issue_type: "Story".to_string(),
            assignee: Some("A Very Long Username That Should Also Be Truncated".to_string()),
            description: None,
            updated: "U".to_string(),
        }];
        let output = format_tickets(&sprint, &issues);
        // Should contain truncation indicator
        assert!(output.contains("…"));
    }

    // Mock client for testing
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
    async fn process_tickets_auto_detects_board() {
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

        let output = process_tickets(&client, None).await.unwrap();
        assert!(output.contains("Sprint 1"));
    }

    #[tokio::test]
    async fn process_tickets_shows_board_selection() {
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

        let output = process_tickets(&client, None).await.unwrap();
        assert!(output.contains("Multiple boards found"));
    }

    #[tokio::test]
    async fn process_tickets_fails_no_boards() {
        let client = MockJiraClient {
            boards: vec![],
            sprint: None,
            issues: vec![],
        };

        let result = process_tickets(&client, None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn process_tickets_fails_no_sprint() {
        let client = MockJiraClient {
            boards: vec![Board {
                id: 1,
                name: "B".to_string(),
                board_type: "s".to_string(),
            }],
            sprint: None,
            issues: vec![],
        };

        let result = process_tickets(&client, None).await;
        assert!(result.is_err());
    }
}
