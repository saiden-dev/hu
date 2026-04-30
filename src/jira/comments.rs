//! `hu jira comments <KEY>` — list comments on an issue.

use anyhow::Result;
use comfy_table::{presets::UTF8_FULL_CONDENSED, Cell, Color, ContentArrangement, Table};

use super::client::{JiraApi, JiraClient};
use super::types::Comment;

/// Arguments for the comments command
#[derive(Debug, Clone)]
pub struct CommentsArgs {
    pub key: String,
    /// Show full comment bodies; otherwise truncate to a single-line preview.
    pub full: bool,
    /// Emit JSON instead of a table.
    pub json: bool,
}

/// Run the jira comments command (CLI entry point — formats and prints).
pub async fn run(args: CommentsArgs) -> Result<()> {
    let client = JiraClient::new().await?;
    let output = process_comments(&client, &args).await?;
    print!("{}", output);
    Ok(())
}

/// Process comments command (business logic, testable).
pub async fn process_comments(client: &impl JiraApi, args: &CommentsArgs) -> Result<String> {
    let comments = client.list_comments(&args.key).await?;
    Ok(format_comments(&args.key, &comments, args.full, args.json))
}

/// Render the comments collection as either a table or JSON.
pub fn format_comments(key: &str, comments: &[Comment], full: bool, json: bool) -> String {
    if json {
        return format_json(comments);
    }
    if comments.is_empty() {
        return format!("No comments on {}.\n", key);
    }
    if full {
        format_full(key, comments)
    } else {
        format_table(key, comments)
    }
}

fn format_json(comments: &[Comment]) -> String {
    serde_json::to_string_pretty(comments).unwrap_or_else(|_| "[]".to_string()) + "\n"
}

fn format_table(key: &str, comments: &[Comment]) -> String {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL_CONDENSED);
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec!["WHEN", "AUTHOR", "BODY"]);

    for comment in comments {
        table.add_row(vec![
            Cell::new(format_date(&comment.created)),
            Cell::new(&comment.author.display_name).fg(Color::Cyan),
            Cell::new(truncate_body(&comment.body, 80)),
        ]);
    }

    let mut output = format!(
        "\x1b[1m{}\x1b[0m — {} comment{}\n",
        key,
        comments.len(),
        if comments.len() == 1 { "" } else { "s" }
    );
    output.push_str(&format!("{}\n", table));
    output
}

fn format_full(key: &str, comments: &[Comment]) -> String {
    let mut output = format!(
        "\x1b[1m{}\x1b[0m — {} comment{}\n\n",
        key,
        comments.len(),
        if comments.len() == 1 { "" } else { "s" }
    );
    for (i, c) in comments.iter().enumerate() {
        if i > 0 {
            output.push('\n');
        }
        output.push_str(&format!(
            "\x1b[36m{}\x1b[0m — {}\n",
            c.author.display_name,
            format_date(&c.created)
        ));
        output.push_str(&c.body);
        if !c.body.ends_with('\n') {
            output.push('\n');
        }
    }
    output
}

/// Format an ISO 8601 timestamp as "YYYY-MM-DD HH:MM" for terminal use.
/// Falls back to the input string for unrecognised shapes.
fn format_date(date: &str) -> String {
    if date.is_empty() {
        return "—".to_string();
    }
    if let Some((date_part, time_part)) = date.split_once('T') {
        if let Some((time, _)) = time_part.split_once('.') {
            return format!("{} {}", date_part, time);
        }
        return format!(
            "{} {}",
            date_part,
            time_part.split('+').next().unwrap_or(time_part)
        );
    }
    date.to_string()
}

/// Single-line preview of a comment body, ellipsised at `max` chars.
fn truncate_body(body: &str, max: usize) -> String {
    let single_line: String = body.replace('\n', " ");
    if single_line.chars().count() <= max {
        return single_line;
    }
    let truncated: String = single_line.chars().take(max.saturating_sub(1)).collect();
    format!("{}…", truncated)
}

#[cfg(test)]
mod tests {
    use super::super::types::User;
    use super::*;
    use serde_json::json;

    fn make_comment(id: &str, author: &str, body: &str, created: &str) -> Comment {
        Comment {
            id: id.to_string(),
            author: User {
                account_id: format!("a-{}", id),
                display_name: author.to_string(),
                email_address: None,
            },
            body: body.to_string(),
            body_adf: json!({"type": "doc", "version": 1, "content": []}),
            created: created.to_string(),
            updated: created.to_string(),
        }
    }

    #[test]
    fn format_comments_empty_message() {
        let out = format_comments("HU-1", &[], false, false);
        assert!(out.contains("No comments on HU-1"));
    }

    #[test]
    fn format_comments_table_includes_header_and_rows() {
        let comments = vec![
            make_comment("1", "Alice", "first", "2026-04-30T10:00:00.000Z"),
            make_comment("2", "Bob", "second", "2026-04-30T11:30:00.000Z"),
        ];
        let out = format_comments("HU-1", &comments, false, false);
        assert!(out.contains("HU-1"));
        assert!(out.contains("2 comments"));
        assert!(out.contains("Alice"));
        assert!(out.contains("Bob"));
        assert!(out.contains("first"));
        assert!(out.contains("2026-04-30 10:00:00"));
    }

    #[test]
    fn format_comments_singular_count() {
        let comments = vec![make_comment(
            "1",
            "Alice",
            "only",
            "2026-04-30T10:00:00.000Z",
        )];
        let out = format_comments("HU-1", &comments, false, false);
        assert!(out.contains("1 comment\n") || out.contains("1 comment\u{a0}"));
        assert!(!out.contains("1 comments"));
    }

    #[test]
    fn format_comments_full_mode_renders_complete_body() {
        let body = "line one\nline two\nline three";
        let comments = vec![make_comment("1", "Alice", body, "2026-04-30T10:00:00.000Z")];
        let out = format_comments("HU-1", &comments, true, false);
        assert!(out.contains("line one"));
        assert!(out.contains("line two"));
        assert!(out.contains("line three"));
        assert!(out.contains("Alice"));
    }

    #[test]
    fn format_comments_table_truncates_body() {
        let long = "a".repeat(200);
        let comments = vec![make_comment(
            "1",
            "Alice",
            &long,
            "2026-04-30T10:00:00.000Z",
        )];
        let out = format_comments("HU-1", &comments, false, false);
        assert!(out.contains('…'));
    }

    #[test]
    fn format_comments_json_emits_valid_array() {
        let comments = vec![make_comment(
            "1",
            "Alice",
            "body",
            "2026-04-30T10:00:00.000Z",
        )];
        let out = format_comments("HU-1", &comments, false, true);
        let parsed: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
        let arr = parsed.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["id"], "1");
        assert_eq!(arr[0]["author"]["display_name"], "Alice");
        assert_eq!(arr[0]["body"], "body");
    }

    #[test]
    fn truncate_body_collapses_newlines() {
        assert_eq!(truncate_body("line\nbreak", 80), "line break");
    }

    #[test]
    fn truncate_body_short_unchanged() {
        assert_eq!(truncate_body("short", 80), "short");
    }

    #[test]
    fn truncate_body_long_ellipsised() {
        let s = "a".repeat(200);
        let t = truncate_body(&s, 50);
        assert_eq!(t.chars().count(), 50);
        assert!(t.ends_with('…'));
    }

    #[test]
    fn format_date_strips_milliseconds_and_timezone() {
        assert_eq!(
            format_date("2026-04-30T10:00:00.000Z"),
            "2026-04-30 10:00:00"
        );
        assert_eq!(
            format_date("2026-04-30T10:00:00+0000"),
            "2026-04-30 10:00:00"
        );
    }

    #[test]
    fn format_date_handles_empty_string() {
        assert_eq!(format_date(""), "—");
    }

    #[test]
    fn format_date_falls_through_unknown_shape() {
        assert_eq!(format_date("yesterday"), "yesterday");
    }

    // Mock client for testing process_comments
    struct MockJiraClient {
        comments: Vec<Comment>,
    }

    impl JiraApi for MockJiraClient {
        async fn get_current_user(&self) -> Result<super::super::types::User> {
            unimplemented!()
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

        async fn get_transitions(
            &self,
            _key: &str,
        ) -> Result<Vec<super::super::types::Transition>> {
            unimplemented!()
        }

        async fn transition_issue(&self, _key: &str, _transition_id: &str) -> Result<()> {
            unimplemented!()
        }

        async fn list_comments(&self, _key: &str) -> Result<Vec<Comment>> {
            Ok(self.comments.clone())
        }
    }

    #[tokio::test]
    async fn process_comments_runs_and_formats_table() {
        let client = MockJiraClient {
            comments: vec![make_comment(
                "1",
                "Alice",
                "hello",
                "2026-04-30T10:00:00.000Z",
            )],
        };
        let args = CommentsArgs {
            key: "HU-1".to_string(),
            full: false,
            json: false,
        };
        let out = process_comments(&client, &args).await.unwrap();
        assert!(out.contains("Alice"));
        assert!(out.contains("hello"));
    }

    #[tokio::test]
    async fn process_comments_json_path() {
        let client = MockJiraClient {
            comments: vec![make_comment(
                "1",
                "Alice",
                "hello",
                "2026-04-30T10:00:00.000Z",
            )],
        };
        let args = CommentsArgs {
            key: "HU-1".to_string(),
            full: false,
            json: true,
        };
        let out = process_comments(&client, &args).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
        assert!(parsed.is_array());
    }

    #[tokio::test]
    async fn process_comments_empty_returns_friendly_message() {
        let client = MockJiraClient { comments: vec![] };
        let args = CommentsArgs {
            key: "HU-1".to_string(),
            full: false,
            json: false,
        };
        let out = process_comments(&client, &args).await.unwrap();
        assert!(out.contains("No comments on HU-1"));
    }
}
