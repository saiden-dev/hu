use super::*;
use std::path::PathBuf;

#[test]
fn format_docs_empty() {
    let output = format_docs(&[], false);
    assert!(output.contains("No documentation files found"));
}

#[test]
fn format_docs_empty_json() {
    let output = format_docs(&[], true);
    assert_eq!(output, "[]");
}

#[test]
fn format_docs_table() {
    let docs = vec![DocEntry {
        path: PathBuf::from("/docs/test.md"),
        title: "Test Doc".to_string(),
        source: Some("https://example.com".to_string()),
        fetched: Some("2024-01-01".to_string()),
        size: 1234,
    }];

    let output = format_docs(&docs, false);
    assert!(output.contains("test.md"));
    assert!(output.contains("Test Doc"));
    assert!(output.contains("example.com"));
    assert!(output.contains("2024-01-01"));
}

#[test]
fn format_docs_table_no_source() {
    let docs = vec![DocEntry {
        path: PathBuf::from("/docs/local.md"),
        title: "Local Doc".to_string(),
        source: None,
        fetched: None,
        size: 100,
    }];

    let output = format_docs(&docs, false);
    assert!(output.contains("local.md"));
    assert!(output.contains("-")); // placeholder for missing source/date
}

#[test]
fn format_docs_json() {
    let docs = vec![DocEntry {
        path: PathBuf::from("/docs/test.md"),
        title: "Test".to_string(),
        source: Some("https://test.com".to_string()),
        fetched: Some("2024-01-01".to_string()),
        size: 500,
    }];

    let output = format_docs(&docs, true);
    assert!(output.contains("\"title\""));
    assert!(output.contains("\"Test\""));
    assert!(output.contains("\"source\""));
}

#[test]
fn truncate_short() {
    assert_eq!(truncate("short", 10), "short");
}

#[test]
fn truncate_exact() {
    assert_eq!(truncate("exactly10!", 10), "exactly10!");
}

#[test]
fn truncate_long() {
    assert_eq!(truncate("this is too long", 10), "this is...");
}

#[test]
fn truncate_url_short() {
    assert_eq!(
        truncate_url("https://example.com", 30),
        "https://example.com"
    );
}

#[test]
fn truncate_url_strips_protocol() {
    assert_eq!(
        truncate_url("https://example.com/very/long/path/here", 20),
        "example.com/very/..."
    );
}

#[test]
fn truncate_url_http() {
    assert_eq!(
        truncate_url("http://example.com/path", 20),
        "example.com/path"
    );
}

#[test]
fn format_sync_result_clean() {
    let result = crate::git::SyncResult {
        files_committed: 0,
        commit_hash: None,
        pushed: false,
        branch: None,
    };
    let output = format_sync_result(&result, false);
    assert!(output.contains("Nothing to commit"));
}

#[test]
fn format_sync_result_committed() {
    let result = crate::git::SyncResult {
        files_committed: 3,
        commit_hash: Some("abc1234".to_string()),
        pushed: false,
        branch: Some("main".to_string()),
    };
    let output = format_sync_result(&result, false);
    assert!(output.contains("Committed 3 files"));
    assert!(output.contains("[main]"));
    assert!(output.contains("abc1234"));
}

#[test]
fn format_sync_result_single_file() {
    let result = crate::git::SyncResult {
        files_committed: 1,
        commit_hash: Some("def5678".to_string()),
        pushed: true,
        branch: Some("feature".to_string()),
    };
    let output = format_sync_result(&result, false);
    assert!(output.contains("1 file"));
    assert!(!output.contains("1 files"));
    assert!(output.contains("Pushed to origin"));
}

#[test]
fn format_sync_result_json() {
    let result = crate::git::SyncResult {
        files_committed: 2,
        commit_hash: Some("xyz".to_string()),
        pushed: true,
        branch: Some("main".to_string()),
    };
    let output = format_sync_result(&result, true);
    assert!(output.contains("\"files_committed\""));
    assert!(output.contains("\"pushed\": true"));
}

#[test]
fn format_created_output() {
    let path = PathBuf::from("/docs/test.md");
    let output = format_created(&path, "Test Topic");
    assert!(output.contains("/docs/test.md"));
    assert!(output.contains("Test Topic"));
    assert!(output.contains("\u{2713}")); // checkmark
}

#[test]
fn format_removed_output() {
    let path = PathBuf::from("/docs/removed.md");
    let output = format_removed(&path);
    assert!(output.contains("/docs/removed.md"));
    assert!(output.contains("Removed"));
}

#[test]
fn format_docs_multiple() {
    let docs = vec![
        DocEntry {
            path: PathBuf::from("/docs/first.md"),
            title: "First".to_string(),
            source: None,
            fetched: None,
            size: 100,
        },
        DocEntry {
            path: PathBuf::from("/docs/second.md"),
            title: "Second".to_string(),
            source: Some("https://second.com".to_string()),
            fetched: Some("2024-02-01".to_string()),
            size: 200,
        },
    ];

    let output = format_docs(&docs, false);
    assert!(output.contains("first.md"));
    assert!(output.contains("second.md"));
    assert!(output.contains("First"));
    assert!(output.contains("Second"));
}

#[test]
fn format_docs_long_title_truncated() {
    let docs = vec![DocEntry {
        path: PathBuf::from("/docs/test.md"),
        title: "This is a very long title that should be truncated".to_string(),
        source: None,
        fetched: None,
        size: 100,
    }];

    let output = format_docs(&docs, false);
    assert!(output.contains("..."));
}

#[test]
fn format_sync_result_no_push() {
    let result = crate::git::SyncResult {
        files_committed: 1,
        commit_hash: Some("abc".to_string()),
        pushed: false,
        branch: Some("main".to_string()),
    };
    let output = format_sync_result(&result, false);
    assert!(output.contains("--no-push"));
}

#[test]
fn truncate_url_already_short_after_strip() {
    // After stripping protocol, URL is short enough
    assert_eq!(truncate_url("https://ex.com", 10), "ex.com");
}

#[test]
fn format_sync_json_zero_files() {
    let result = crate::git::SyncResult {
        files_committed: 0,
        commit_hash: None,
        pushed: false,
        branch: None,
    };
    let output = format_sync_result(&result, true);
    assert!(output.contains("\"files_committed\": 0"));
}
