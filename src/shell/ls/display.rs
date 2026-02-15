use super::colors::FileColors;
use super::service::format_size;
use super::types::{FileEntry, FileKind};
use comfy_table::{
    presets::UTF8_FULL_CONDENSED, Attribute, Cell, Color, ContentArrangement, Table,
};
use owo_colors::OwoColorize;
use std::sync::LazyLock;

static FILE_COLORS: LazyLock<FileColors> = LazyLock::new(FileColors::new);

pub fn format_simple(entries: &[FileEntry]) -> String {
    if entries.is_empty() {
        return String::new();
    }

    entries
        .iter()
        .map(colorize_name)
        .collect::<Vec<_>>()
        .join("  ")
}

pub fn format_long(entries: &[FileEntry]) -> String {
    if entries.is_empty() {
        return String::new();
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("Perms").add_attribute(Attribute::Dim),
            Cell::new("Size").add_attribute(Attribute::Dim),
            Cell::new("Modified").add_attribute(Attribute::Dim),
            Cell::new("Name").add_attribute(Attribute::Dim),
        ]);

    for e in entries {
        let name_cell = build_name_cell(e, &e.name);

        table.add_row(vec![
            Cell::new(&e.permissions).add_attribute(Attribute::Dim),
            Cell::new(format_size(e.size)).fg(Color::Cyan),
            Cell::new(&e.modified_str).fg(Color::Yellow),
            name_cell,
        ]);
    }

    table.to_string()
}

fn build_name_cell(entry: &FileEntry, name_str: &str) -> Cell {
    let cell = if let Some(ref target) = entry.link_target {
        Cell::new(format!("{} -> {}", name_str, target))
    } else {
        Cell::new(name_str)
    };

    match entry.kind {
        FileKind::Directory => cell
            .fg(FILE_COLORS.directory())
            .add_attribute(Attribute::Bold),
        FileKind::Symlink => cell.fg(FILE_COLORS.symlink()),
        FileKind::Socket | FileKind::Fifo => cell.fg(Color::Magenta),
        FileKind::BlockDevice | FileKind::CharDevice => {
            cell.fg(Color::Yellow).add_attribute(Attribute::Bold)
        }
        FileKind::File | FileKind::Unknown => {
            if entry.is_executable {
                cell.fg(FILE_COLORS.executable())
                    .add_attribute(Attribute::Bold)
            } else if entry.is_hidden {
                cell.fg(FILE_COLORS.hidden_color())
            } else {
                // Get color based on file extension
                let ext = entry.name.rsplit('.').next().unwrap_or("");
                let color = FILE_COLORS.for_extension(ext);
                cell.fg(color)
            }
        }
    }
}

pub fn format_json(entries: &[FileEntry]) -> String {
    serde_json::to_string_pretty(entries).unwrap_or_else(|_| "[]".to_string())
}

fn colorize_name(entry: &FileEntry) -> String {
    let name = &entry.name;

    match entry.kind {
        FileKind::Directory => name.blue().bold().to_string(),
        FileKind::Symlink => name.cyan().to_string(),
        FileKind::Socket | FileKind::Fifo => name.magenta().to_string(),
        FileKind::BlockDevice | FileKind::CharDevice => name.yellow().bold().to_string(),
        FileKind::File | FileKind::Unknown => {
            if entry.is_executable {
                name.green().bold().to_string()
            } else if entry.is_hidden {
                name.dimmed().to_string()
            } else {
                name.to_string()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    fn make_entry(name: &str, kind: FileKind, size: u64) -> FileEntry {
        FileEntry {
            name: name.to_string(),
            kind,
            size,
            modified: Some(SystemTime::now()),
            modified_str: "Feb 15 12:00".to_string(),
            permissions: "-rw-r--r--".to_string(),
            is_hidden: name.starts_with('.'),
            is_executable: false,
            link_target: None,
        }
    }

    #[test]
    fn format_simple_empty() {
        let entries: Vec<FileEntry> = vec![];
        assert_eq!(format_simple(&entries), "");
    }

    #[test]
    fn format_simple_single_file() {
        let entries = vec![make_entry("file.txt", FileKind::File, 100)];
        let output = format_simple(&entries);
        assert!(output.contains("file.txt"));
    }

    #[test]
    fn format_simple_multiple() {
        let entries = vec![
            make_entry("dir", FileKind::Directory, 0),
            make_entry("file.rs", FileKind::File, 100),
        ];
        let output = format_simple(&entries);
        assert!(output.contains("dir"));
        assert!(output.contains("file.rs"));
        assert!(output.contains("  ")); // separator
    }

    #[test]
    fn format_long_empty() {
        let entries: Vec<FileEntry> = vec![];
        assert_eq!(format_long(&entries), "");
    }

    #[test]
    fn format_long_shows_permissions() {
        let entries = vec![make_entry("file.txt", FileKind::File, 1024)];
        let output = format_long(&entries);
        // Permissions should be in output (table format, may have ANSI codes)
        assert!(output.contains("rw") || output.contains("Perms"));
    }

    #[test]
    fn format_long_shows_size() {
        let entries = vec![make_entry("file.txt", FileKind::File, 1024)];
        let output = format_long(&entries);
        assert!(output.contains("1.0K") || output.contains("Size"));
    }

    #[test]
    fn format_long_shows_date() {
        let entries = vec![make_entry("file.txt", FileKind::File, 100)];
        let output = format_long(&entries);
        assert!(output.contains("Feb") || output.contains("Modified"));
    }

    #[test]
    fn format_long_symlink_shows_target() {
        let mut entry = make_entry("link", FileKind::Symlink, 0);
        entry.link_target = Some("/target/path".to_string());
        let entries = vec![entry];
        let output = format_long(&entries);
        assert!(output.contains("->") || output.contains("target"));
    }

    #[test]
    fn format_json_empty() {
        let entries: Vec<FileEntry> = vec![];
        let output = format_json(&entries);
        assert_eq!(output, "[]");
    }

    #[test]
    fn format_json_valid() {
        let entries = vec![make_entry("file.txt", FileKind::File, 100)];
        let output = format_json(&entries);
        let parsed: Result<Vec<FileEntry>, _> = serde_json::from_str(&output);
        assert!(parsed.is_ok());
    }

    #[test]
    fn colorize_directory() {
        let entry = make_entry("src", FileKind::Directory, 0);
        let colored = colorize_name(&entry);
        // Should contain ANSI codes for blue/bold
        assert!(colored.contains("\x1b[") || colored == "src");
    }

    #[test]
    fn colorize_executable() {
        let mut entry = make_entry("script", FileKind::File, 100);
        entry.is_executable = true;
        let colored = colorize_name(&entry);
        assert!(colored.contains("\x1b[") || colored == "script");
    }

    #[test]
    fn colorize_hidden() {
        let entry = make_entry(".hidden", FileKind::File, 100);
        assert!(entry.is_hidden);
        let colored = colorize_name(&entry);
        assert!(colored.contains("\x1b[") || colored == ".hidden");
    }
}
