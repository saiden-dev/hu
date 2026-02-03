use anyhow::{Context, Result};
use std::fs;
use std::io::BufRead;
use std::path::PathBuf;
use std::time::SystemTime;

use super::store::{default_store, ContextStore};
use super::types::{ContextEntry, ContextState, FileStatus};

#[cfg(test)]
mod tests;

/// Track file(s) as loaded in context
pub async fn track(paths: &[String]) -> Result<()> {
    let store = default_store()?;
    track_with_store(&store, paths)
}

/// Track files using a specific store (for testing)
pub fn track_with_store(store: &impl ContextStore, paths: &[String]) -> Result<()> {
    let mut state = store.load()?;

    for path_str in paths {
        let path = resolve_path(path_str)?;
        let (size, line_count) = get_file_info(&path)?;
        let entry = ContextEntry::new(path.clone(), size, line_count);
        state.track(entry);
        println!(
            "Tracked: {} ({} lines, {} bytes)",
            path.display(),
            line_count,
            size
        );
    }

    store.save(&state)?;
    Ok(())
}

/// Check if file(s) are in context
pub async fn check(paths: &[String]) -> Result<()> {
    let store = default_store()?;
    check_with_store(&store, paths)
}

/// Check files using a specific store (for testing)
pub fn check_with_store(store: &impl ContextStore, paths: &[String]) -> Result<()> {
    let state = store.load()?;
    let now = current_timestamp();

    for path_str in paths {
        let path = resolve_path(path_str)?;
        let status = get_file_status(&state, &path, now)?;
        print_file_status(&status);
    }

    Ok(())
}

/// Show summary of all tracked files
pub async fn summary() -> Result<()> {
    let store = default_store()?;
    summary_with_store(&store)
}

/// Show summary using a specific store (for testing)
pub fn summary_with_store(store: &impl ContextStore) -> Result<()> {
    let state = store.load()?;
    let now = current_timestamp();

    if state.file_count() == 0 {
        println!("No files tracked in context");
        return Ok(());
    }

    println!("Session: {}", state.session_id);
    println!();

    let mut entries: Vec<_> = state.all_entries().into_iter().collect();
    entries.sort_by(|a, b| b.tracked_at.cmp(&a.tracked_at));

    for entry in &entries {
        let age = format_age(now.saturating_sub(entry.tracked_at));
        println!(
            "  {} ({} lines, {}) - {}",
            entry.path.display(),
            entry.line_count,
            format_bytes(entry.size),
            age
        );
    }

    println!();
    println!(
        "Total: {} files, {} lines, {}",
        state.file_count(),
        state.total_lines(),
        format_bytes(state.total_bytes())
    );

    Ok(())
}

/// Clear all tracked files
pub async fn clear() -> Result<()> {
    let store = default_store()?;
    clear_with_store(&store)
}

/// Clear using a specific store (for testing)
pub fn clear_with_store(store: &impl ContextStore) -> Result<()> {
    store.delete()?;
    println!("Context cleared");
    Ok(())
}

/// Get file status relative to current context
pub fn get_file_status(state: &ContextState, path: &PathBuf, now: u64) -> Result<FileStatus> {
    if let Some(entry) = state.get(path) {
        let age_secs = now.saturating_sub(entry.tracked_at);
        Ok(FileStatus::Loaded {
            entry: entry.clone(),
            age_secs,
        })
    } else {
        let (size, line_count) = get_file_info(path)?;
        Ok(FileStatus::NotLoaded {
            path: path.clone(),
            size,
            line_count,
        })
    }
}

/// Resolve a path string to an absolute path
fn resolve_path(path_str: &str) -> Result<PathBuf> {
    let path = PathBuf::from(path_str);
    let resolved = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .context("Failed to get current directory")?
            .join(path)
    };

    resolved
        .canonicalize()
        .with_context(|| format!("Path not found: {}", path_str))
}

/// Get file size and line count
fn get_file_info(path: &PathBuf) -> Result<(u64, usize)> {
    let metadata = fs::metadata(path)
        .with_context(|| format!("Failed to read metadata for {}", path.display()))?;
    let size = metadata.len();

    let file =
        fs::File::open(path).with_context(|| format!("Failed to open {}", path.display()))?;
    let reader = std::io::BufReader::new(file);
    let line_count = reader.lines().count();

    Ok((size, line_count))
}

/// Get current Unix timestamp
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Print file status to stdout
fn print_file_status(status: &FileStatus) {
    match status {
        FileStatus::Loaded { entry, age_secs } => {
            let age = format_age(*age_secs);
            println!(
                "{}: loaded {} ({} lines)",
                entry.path.display(),
                age,
                entry.line_count
            );
        }
        FileStatus::NotLoaded {
            path,
            size: _,
            line_count,
        } => {
            println!("{}: not loaded ({} lines)", path.display(), line_count);
        }
    }
}

/// Format seconds as human-readable age
fn format_age(secs: u64) -> String {
    if secs < 60 {
        format!("{}s ago", secs)
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86400 {
        format!("{}h ago", secs / 3600)
    } else {
        format!("{}d ago", secs / 86400)
    }
}

/// Format bytes as human-readable size
fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
