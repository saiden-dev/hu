//! Status table rendering for `hu setup status`.
//!
//! Pure-function renderer: takes a `Vec<StatusRow>` and produces a string
//! using `comfy_table` with the project-standard `UTF8_FULL_CONDENSED` preset.
//! Tested without I/O via snapshot-style equality on the rendered output.

// reason: render is invoked only by `hu setup status` (this chunk) and `preview`.
// Tests cover the rendered output directly.
#![allow(dead_code)]

use comfy_table::{presets::UTF8_FULL_CONDENSED, Cell, Color, Table};

use crate::setup::types::Status;

/// One row of the status table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusRow {
    pub category: String,
    pub name: String,
    pub status: Status,
    pub note: String,
}

impl StatusRow {
    pub fn new(category: &str, name: &str, status: Status) -> Self {
        Self {
            category: category.to_string(),
            name: name.to_string(),
            status,
            note: String::new(),
        }
    }

    pub fn with_note(mut self, note: &str) -> Self {
        self.note = note.to_string();
        self
    }
}

/// Render a status table to a string.
pub fn render(rows: &[StatusRow]) -> String {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL_CONDENSED);
    table.set_header(vec!["", "Category", "Name", "Note"]);
    for row in rows {
        let icon_cell = Cell::new(row.status.icon()).fg(status_color(row.status));
        table.add_row(vec![
            icon_cell,
            Cell::new(&row.category),
            Cell::new(&row.name),
            Cell::new(&row.note),
        ]);
    }
    table.to_string()
}

/// Summary line: "X/Y satisfied".
pub fn summary(rows: &[StatusRow]) -> String {
    let total = rows.len();
    let satisfied = rows.iter().filter(|r| r.status.is_satisfied()).count();
    format!("{}/{} satisfied", satisfied, total)
}

fn status_color(s: Status) -> Color {
    match s {
        Status::Already | Status::Installed => Color::Green,
        Status::Skipped => Color::Yellow,
        Status::Failed => Color::Red,
        Status::Unknown => Color::DarkGrey,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_empty_table() {
        let out = render(&[]);
        // header still present
        assert!(out.contains("Category"));
        assert!(out.contains("Name"));
    }

    #[test]
    fn renders_single_row_with_icon() {
        let rows = vec![StatusRow::new("brew", "gh", Status::Already)];
        let out = render(&rows);
        assert!(out.contains("brew"));
        assert!(out.contains("gh"));
        assert!(out.contains("✓"));
    }

    #[test]
    fn renders_failed_with_x_icon() {
        let rows = vec![StatusRow::new("brew", "missing", Status::Failed)];
        let out = render(&rows);
        assert!(out.contains("✗"));
    }

    #[test]
    fn summary_counts_satisfied_only() {
        let rows = vec![
            StatusRow::new("brew", "a", Status::Already),
            StatusRow::new("brew", "b", Status::Installed),
            StatusRow::new("brew", "c", Status::Failed),
            StatusRow::new("brew", "d", Status::Unknown),
        ];
        assert_eq!(summary(&rows), "2/4 satisfied");
    }

    #[test]
    fn summary_handles_empty() {
        assert_eq!(summary(&[]), "0/0 satisfied");
    }

    #[test]
    fn with_note_attaches_string() {
        let row = StatusRow::new("ssh", "id_ed25519", Status::Already).with_note("chmod 600");
        assert_eq!(row.note, "chmod 600");
    }

    #[test]
    fn unknown_status_renders_open_circle() {
        let rows = vec![StatusRow::new("brew", "?", Status::Unknown)];
        let out = render(&rows);
        assert!(out.contains("○"));
    }

    #[test]
    fn skipped_status_renders_half_circle() {
        let rows = vec![StatusRow::new("brew", "x", Status::Skipped)];
        let out = render(&rows);
        assert!(out.contains("◐"));
    }
}
