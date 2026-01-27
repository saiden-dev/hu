use anyhow::Result;

use super::client::GithubClient;
use super::types::PullRequest;

/// Handle the `hu gh prs` command
pub async fn run() -> Result<()> {
    let client = GithubClient::new()?;
    let prs = client.list_user_prs().await?;

    if prs.is_empty() {
        println!("No open pull requests found.");
        return Ok(());
    }

    print_prs_table(&prs);
    Ok(())
}

fn print_prs_table(prs: &[PullRequest]) {
    // Calculate column widths
    let num_width = prs
        .iter()
        .map(|p| p.number.to_string().len())
        .max()
        .unwrap_or(2);
    let repo_width = prs
        .iter()
        .map(|p| p.repo_full_name.len())
        .max()
        .unwrap_or(10)
        .min(30);
    let title_width = 50;

    // Print header
    println!(
        "{:>num_width$}  {:<repo_width$}  {:<title_width$}  Updated",
        "#",
        "Repository",
        "Title",
        num_width = num_width,
        repo_width = repo_width,
        title_width = title_width
    );
    println!(
        "{:->num_width$}  {:->repo_width$}  {:->title_width$}  {:->19}",
        "",
        "",
        "",
        "",
        num_width = num_width,
        repo_width = repo_width,
        title_width = title_width
    );

    // Print rows
    for pr in prs {
        let title = truncate(&pr.title, title_width);
        let repo = truncate(&pr.repo_full_name, repo_width);
        let updated = format_time(&pr.updated_at);

        println!(
            "{:>num_width$}  {:<repo_width$}  {:<title_width$}  {}",
            pr.number,
            repo,
            title,
            updated,
            num_width = num_width,
            repo_width = repo_width,
            title_width = title_width
        );
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

fn format_time(iso_time: &str) -> String {
    // Simple relative time formatting
    // In a real implementation, use chrono or similar
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(iso_time) {
        let now = chrono::Utc::now();
        let duration = now.signed_duration_since(dt);

        if duration.num_days() > 0 {
            format!("{}d ago", duration.num_days())
        } else if duration.num_hours() > 0 {
            format!("{}h ago", duration.num_hours())
        } else if duration.num_minutes() > 0 {
            format!("{}m ago", duration.num_minutes())
        } else {
            "just now".to_string()
        }
    } else {
        iso_time.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_short_string() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_long_string() {
        assert_eq!(truncate("hello world", 8), "hello...");
    }

    #[test]
    fn truncate_exact_length() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn format_time_recent() {
        let now = chrono::Utc::now();
        let iso = now.to_rfc3339();
        let formatted = format_time(&iso);
        assert!(
            formatted.contains("just now")
                || formatted.contains("m ago")
                || formatted.contains("h ago")
        );
    }

    #[test]
    fn format_time_days_ago() {
        let past = chrono::Utc::now() - chrono::Duration::days(5);
        let iso = past.to_rfc3339();
        let formatted = format_time(&iso);
        assert!(formatted.contains("5d ago"));
    }

    #[test]
    fn format_time_invalid() {
        let invalid = "not-a-date";
        let formatted = format_time(invalid);
        assert_eq!(formatted, "not-a-date");
    }
}
