use anyhow::Result;

use super::client::{GithubApi, GithubClient};
use super::types::CiStatus;

// ANSI color codes
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";
const GRAY: &str = "\x1b[90m";
const RESET: &str = "\x1b[0m";

/// Handle the `hu gh prs` command
pub async fn run() -> Result<()> {
    let client = GithubClient::new()?;
    run_with_client(&client).await
}

fn get_terminal_width() -> usize {
    terminal_size::terminal_size()
        .map(|(w, _)| w.0 as usize)
        .unwrap_or(80)
}

fn print_prs_table(prs: &[super::types::PullRequest]) {
    let term_width = get_terminal_width();

    // Calculate max link length
    let max_link_len = prs.iter().map(|p| p.html_url.len()).max().unwrap_or(40);

    // Layout: │ S │ Title... │ Link │
    // Borders take: 1 + 1 + 3 + 3 + 1 = 9 chars (│ S │ ... │ ... │)
    let status_col = 1;
    let border_overhead = 10; // "│ " + " │ " + " │ " + "│"

    let available = term_width.saturating_sub(border_overhead + status_col + max_link_len);
    let title_width = available.max(20);
    let link_width = max_link_len;

    // Top border
    println!(
        "┌───┬{}┬{}┐",
        "─".repeat(title_width + 2),
        "─".repeat(link_width + 2)
    );

    // Rows
    for pr in prs {
        let status_icon = match pr.ci_status.unwrap_or(CiStatus::Unknown) {
            CiStatus::Success => format!("{}{}{}", GREEN, "✓", RESET),
            CiStatus::Pending => format!("{}{}{}", YELLOW, "◐", RESET),
            CiStatus::Failed => format!("{}{}{}", RED, "✗", RESET),
            CiStatus::Unknown => format!("{}{}{}", GRAY, "○", RESET),
        };

        let title = truncate(&pr.title, title_width);
        let link = format!("{}{}{}", GRAY, &pr.html_url, RESET);

        println!(
            "│ {} │ {:<width$} │ {} │",
            status_icon,
            title,
            link,
            width = title_width
        );
    }

    // Bottom border
    println!(
        "└───┴{}┴{}┘",
        "─".repeat(title_width + 2),
        "─".repeat(link_width + 2)
    );
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len.saturating_sub(1)).collect();
        format!("{}…", truncated)
    }
}

/// Fetch and display PRs using the given API client
pub async fn run_with_client(client: &impl GithubApi) -> Result<()> {
    let mut prs = client.list_user_prs().await?;

    if prs.is_empty() {
        println!("No open pull requests found.");
        return Ok(());
    }

    // Fetch CI status for each PR
    for pr in &mut prs {
        let parts: Vec<&str> = pr.repo_full_name.split('/').collect();
        if parts.len() == 2 {
            if let Ok(status) = client.get_ci_status(parts[0], parts[1], pr.number).await {
                pr.ci_status = Some(status);
            }
        }
    }

    print_prs_table(&prs);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gh::types::PullRequest;

    #[test]
    fn truncate_short_string() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_long_string() {
        assert_eq!(truncate("hello world", 8), "hello w…");
    }

    #[test]
    fn truncate_exact_length() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn truncate_unicode() {
        // Unicode chars are counted by char, not byte
        assert_eq!(truncate("héllo", 5), "héllo");
        assert_eq!(truncate("héllo world", 6), "héllo…");
    }

    #[test]
    fn truncate_empty() {
        assert_eq!(truncate("", 10), "");
    }

    #[test]
    fn truncate_zero_length() {
        // Edge case: max_len = 0 means we try to take 0 chars + ellipsis
        // saturating_sub(1) on 0 = 0, so we get just "…" if string is not empty
        let result = truncate("hello", 0);
        // With max_len=0, chars.count()=5 > 0, so we truncate
        // take(0.saturating_sub(1)) = take(0), so we get "" + "…" = "…"
        assert_eq!(result, "…");
    }

    #[test]
    fn status_icons_render() {
        let _ = format!("{}✓{}", GREEN, RESET);
        let _ = format!("{}◐{}", YELLOW, RESET);
        let _ = format!("{}✗{}", RED, RESET);
    }

    #[test]
    fn get_terminal_width_returns_reasonable_value() {
        let width = get_terminal_width();
        // Should return at least 80 (default) or actual terminal width
        assert!(width >= 20);
    }

    #[test]
    fn status_icon_formatting_success() {
        let icon = format!("{}{}{}", GREEN, "✓", RESET);
        assert!(icon.contains("✓"));
        assert!(icon.starts_with("\x1b[32m"));
        assert!(icon.ends_with("\x1b[0m"));
    }

    #[test]
    fn status_icon_formatting_pending() {
        let icon = format!("{}{}{}", YELLOW, "◐", RESET);
        assert!(icon.contains("◐"));
    }

    #[test]
    fn status_icon_formatting_failed() {
        let icon = format!("{}{}{}", RED, "✗", RESET);
        assert!(icon.contains("✗"));
    }

    #[test]
    fn status_icon_formatting_unknown() {
        let icon = format!("{}{}{}", GRAY, "○", RESET);
        assert!(icon.contains("○"));
    }

    #[test]
    fn print_prs_table_renders_without_panic() {
        let prs = vec![
            PullRequest {
                number: 1,
                title: "Short title".to_string(),
                html_url: "https://github.com/o/r/pull/1".to_string(),
                state: "open".to_string(),
                repo_full_name: "o/r".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                updated_at: "2024-01-01T00:00:00Z".to_string(),
                ci_status: Some(CiStatus::Success),
            },
            PullRequest {
                number: 2,
                title: "A very long title that will definitely need truncation because it exceeds the available width".to_string(),
                html_url: "https://github.com/owner/repo/pull/2".to_string(),
                state: "open".to_string(),
                repo_full_name: "owner/repo".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                updated_at: "2024-01-01T00:00:00Z".to_string(),
                ci_status: Some(CiStatus::Failed),
            },
            PullRequest {
                number: 3,
                title: "Pending PR".to_string(),
                html_url: "https://github.com/o/r/pull/3".to_string(),
                state: "open".to_string(),
                repo_full_name: "o/r".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                updated_at: "2024-01-01T00:00:00Z".to_string(),
                ci_status: Some(CiStatus::Pending),
            },
            PullRequest {
                number: 4,
                title: "Unknown status".to_string(),
                html_url: "https://github.com/o/r/pull/4".to_string(),
                state: "open".to_string(),
                repo_full_name: "o/r".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                updated_at: "2024-01-01T00:00:00Z".to_string(),
                ci_status: None,
            },
        ];
        // This just verifies it doesn't panic
        print_prs_table(&prs);
    }

    #[test]
    fn print_prs_table_empty_list() {
        let prs: Vec<PullRequest> = vec![];
        print_prs_table(&prs);
    }

    // Mock implementation for testing
    struct MockGithubApi {
        prs: Vec<PullRequest>,
        ci_status: CiStatus,
    }

    impl GithubApi for MockGithubApi {
        async fn list_user_prs(&self) -> Result<Vec<PullRequest>> {
            Ok(self.prs.clone())
        }

        async fn get_ci_status(&self, _owner: &str, _repo: &str, _pr: u64) -> Result<CiStatus> {
            Ok(self.ci_status)
        }

        async fn get_pr_branch(&self, _owner: &str, _repo: &str, _pr: u64) -> Result<String> {
            Ok("main".to_string())
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
    }

    #[tokio::test]
    async fn run_with_client_empty_prs() {
        let mock = MockGithubApi {
            prs: vec![],
            ci_status: CiStatus::Unknown,
        };
        let result = run_with_client(&mock).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn run_with_client_with_prs() {
        let mock = MockGithubApi {
            prs: vec![PullRequest {
                number: 1,
                title: "Test PR".to_string(),
                html_url: "https://github.com/o/r/pull/1".to_string(),
                state: "open".to_string(),
                repo_full_name: "o/r".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                updated_at: "2024-01-01T00:00:00Z".to_string(),
                ci_status: None,
            }],
            ci_status: CiStatus::Success,
        };
        let result = run_with_client(&mock).await;
        assert!(result.is_ok());
    }
}
