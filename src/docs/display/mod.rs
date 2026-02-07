use comfy_table::presets::UTF8_FULL_CONDENSED;
use comfy_table::{Cell, Table};

use super::types::DocEntry;

#[cfg(test)]
mod tests;

/// Format documents for display
pub fn format_docs(docs: &[DocEntry], json: bool) -> String {
    if json {
        format_json(docs)
    } else {
        format_table(docs)
    }
}

/// Format documents as JSON
fn format_json(docs: &[DocEntry]) -> String {
    serde_json::to_string_pretty(docs).unwrap_or_else(|_| "[]".to_string())
}

/// Format documents as a table
fn format_table(docs: &[DocEntry]) -> String {
    if docs.is_empty() {
        return "No documentation files found.".to_string();
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL_CONDENSED);
    table.set_header(vec!["File", "Title", "Source", "Date"]);

    for doc in docs {
        let file = doc
            .path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        let source = doc
            .source
            .as_ref()
            .map(|s| truncate_url(s, 40))
            .unwrap_or_else(|| "-".to_string());

        let date = doc.fetched.as_deref().unwrap_or("-");

        table.add_row(vec![
            Cell::new(file),
            Cell::new(truncate(&doc.title, 30)),
            Cell::new(source),
            Cell::new(date),
        ]);
    }

    table.to_string()
}

/// Truncate string with ellipsis
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max - 3])
    }
}

/// Truncate URL, keeping domain visible
fn truncate_url(url: &str, max: usize) -> String {
    if url.len() <= max {
        return url.to_string();
    }

    // Try to keep domain visible
    let url = url
        .trim_start_matches("https://")
        .trim_start_matches("http://");

    if url.len() <= max {
        return url.to_string();
    }

    format!("{}...", &url[..max - 3])
}

/// Format sync result for display
pub fn format_sync_result(result: &crate::git::SyncResult, json: bool) -> String {
    if json {
        return serde_json::to_string_pretty(result).unwrap_or_else(|_| "{}".to_string());
    }

    if result.files_committed == 0 {
        return "Nothing to commit, working tree clean".to_string();
    }

    let mut output = Vec::new();

    if let Some(hash) = &result.commit_hash {
        let branch = result.branch.as_deref().unwrap_or("unknown");
        output.push(format!(
            "\x1b[32m\u{2713}\x1b[0m Committed {} {} [{}] {}",
            result.files_committed,
            if result.files_committed == 1 {
                "file"
            } else {
                "files"
            },
            branch,
            hash
        ));
    }

    if result.pushed {
        output.push("\x1b[32m\u{2713}\x1b[0m Pushed to origin".to_string());
    } else if result.commit_hash.is_some() {
        output.push("\x1b[33m\u{25D0}\x1b[0m No remote or --no-push".to_string());
    }

    output.join("\n")
}

/// Format file creation result
pub fn format_created(path: &std::path::Path, topic: &str) -> String {
    format!(
        "\x1b[32m\u{2713}\x1b[0m Created {} ({})",
        path.display(),
        topic
    )
}

/// Format file removal result
pub fn format_removed(path: &std::path::Path) -> String {
    format!("\x1b[32m\u{2713}\x1b[0m Removed {}", path.display())
}
