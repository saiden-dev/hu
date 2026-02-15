use super::types::{FileEntry, FileKind};
use anyhow::{Context, Result};
use std::fs;
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::path::Path;
use std::time::SystemTime;

pub fn list_directory(path: &Path, show_hidden: bool) -> Result<Vec<FileEntry>> {
    let entries =
        fs::read_dir(path).with_context(|| format!("Cannot read directory: {}", path.display()))?;

    let mut files: Vec<FileEntry> = Vec::new();

    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden files unless -a flag
        let is_hidden = name.starts_with('.');
        if is_hidden && !show_hidden {
            continue;
        }

        let file_entry = build_entry(&entry.path(), name, is_hidden)?;
        files.push(file_entry);
    }

    // Sort: directories first, then alphabetically (case-insensitive)
    files.sort_by(|a, b| {
        let dir_order = match (&a.kind, &b.kind) {
            (FileKind::Directory, FileKind::Directory) => std::cmp::Ordering::Equal,
            (FileKind::Directory, _) => std::cmp::Ordering::Less,
            (_, FileKind::Directory) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        };
        dir_order.then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(files)
}

fn build_entry(path: &Path, name: String, is_hidden: bool) -> Result<FileEntry> {
    // Use symlink_metadata to not follow symlinks
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("Cannot read metadata: {}", path.display()))?;

    let file_type = metadata.file_type();
    let kind = detect_kind(&file_type);

    let link_target = if file_type.is_symlink() {
        fs::read_link(path)
            .ok()
            .map(|p| p.to_string_lossy().to_string())
    } else {
        None
    };

    let modified = metadata.modified().ok();
    let modified_str = format_time(modified);

    let mode = metadata.permissions().mode();
    let permissions = format_permissions(mode);
    let is_executable = mode & 0o111 != 0 && file_type.is_file();

    Ok(FileEntry {
        name,
        kind,
        size: metadata.len(),
        modified,
        modified_str,
        permissions,
        is_hidden,
        is_executable,
        link_target,
    })
}

fn detect_kind(file_type: &fs::FileType) -> FileKind {
    if file_type.is_dir() {
        FileKind::Directory
    } else if file_type.is_symlink() {
        FileKind::Symlink
    } else if file_type.is_socket() {
        FileKind::Socket
    } else if file_type.is_fifo() {
        FileKind::Fifo
    } else if file_type.is_block_device() {
        FileKind::BlockDevice
    } else if file_type.is_char_device() {
        FileKind::CharDevice
    } else if file_type.is_file() {
        FileKind::File
    } else {
        FileKind::Unknown
    }
}

fn format_permissions(mode: u32) -> String {
    let mut s = String::with_capacity(10);

    // File type
    s.push(match mode & 0o170000 {
        0o140000 => 's', // socket
        0o120000 => 'l', // symlink
        0o100000 => '-', // regular file
        0o060000 => 'b', // block device
        0o040000 => 'd', // directory
        0o020000 => 'c', // char device
        0o010000 => 'p', // fifo
        _ => '?',
    });

    // Owner
    s.push(if mode & 0o400 != 0 { 'r' } else { '-' });
    s.push(if mode & 0o200 != 0 { 'w' } else { '-' });
    s.push(if mode & 0o100 != 0 { 'x' } else { '-' });

    // Group
    s.push(if mode & 0o040 != 0 { 'r' } else { '-' });
    s.push(if mode & 0o020 != 0 { 'w' } else { '-' });
    s.push(if mode & 0o010 != 0 { 'x' } else { '-' });

    // Others
    s.push(if mode & 0o004 != 0 { 'r' } else { '-' });
    s.push(if mode & 0o002 != 0 { 'w' } else { '-' });
    s.push(if mode & 0o001 != 0 { 'x' } else { '-' });

    s
}

fn format_time(time: Option<SystemTime>) -> String {
    let Some(time) = time else {
        return String::from("-");
    };

    let Ok(duration) = time.duration_since(std::time::UNIX_EPOCH) else {
        return String::from("-");
    };

    let secs = duration.as_secs() as i64;
    let dt = chrono::DateTime::from_timestamp(secs, 0);

    match dt {
        Some(dt) => {
            let now = chrono::Utc::now();
            let six_months_ago = now - chrono::Duration::days(180);

            if dt < six_months_ago {
                dt.format("%b %e  %Y").to_string()
            } else {
                dt.format("%b %e %H:%M").to_string()
            }
        }
        None => String::from("-"),
    }
}

pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.1}T", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1}G", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}M", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}K", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::TempDir;

    #[test]
    fn list_empty_directory() {
        let dir = TempDir::new().unwrap();
        let entries = list_directory(dir.path(), false).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn list_with_files() {
        let dir = TempDir::new().unwrap();
        File::create(dir.path().join("file.txt")).unwrap();
        fs::create_dir(dir.path().join("subdir")).unwrap();

        let entries = list_directory(dir.path(), false).unwrap();
        assert_eq!(entries.len(), 2);
        // Directories should come first
        assert_eq!(entries[0].name, "subdir");
        assert_eq!(entries[0].kind, FileKind::Directory);
        assert_eq!(entries[1].name, "file.txt");
        assert_eq!(entries[1].kind, FileKind::File);
    }

    #[test]
    fn hidden_files_excluded_by_default() {
        let dir = TempDir::new().unwrap();
        File::create(dir.path().join(".hidden")).unwrap();
        File::create(dir.path().join("visible")).unwrap();

        let entries = list_directory(dir.path(), false).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "visible");
    }

    #[test]
    fn hidden_files_included_with_flag() {
        let dir = TempDir::new().unwrap();
        File::create(dir.path().join(".hidden")).unwrap();
        File::create(dir.path().join("visible")).unwrap();

        let entries = list_directory(dir.path(), true).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn format_permissions_regular_file() {
        let perms = format_permissions(0o100644);
        assert_eq!(perms, "-rw-r--r--");
    }

    #[test]
    fn format_permissions_executable() {
        let perms = format_permissions(0o100755);
        assert_eq!(perms, "-rwxr-xr-x");
    }

    #[test]
    fn format_permissions_directory() {
        let perms = format_permissions(0o040755);
        assert_eq!(perms, "drwxr-xr-x");
    }

    #[test]
    fn format_size_bytes() {
        assert_eq!(format_size(0), "0B");
        assert_eq!(format_size(512), "512B");
        assert_eq!(format_size(1023), "1023B");
    }

    #[test]
    fn format_size_kilobytes() {
        assert_eq!(format_size(1024), "1.0K");
        assert_eq!(format_size(1536), "1.5K");
    }

    #[test]
    fn format_size_megabytes() {
        assert_eq!(format_size(1024 * 1024), "1.0M");
        assert_eq!(format_size(1024 * 1024 * 5), "5.0M");
    }

    #[test]
    fn format_size_gigabytes() {
        assert_eq!(format_size(1024 * 1024 * 1024), "1.0G");
    }

    #[test]
    fn symlink_detection() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("target");
        let link_path = dir.path().join("link");
        File::create(&file_path).unwrap();
        std::os::unix::fs::symlink(&file_path, &link_path).unwrap();

        let entries = list_directory(dir.path(), false).unwrap();
        let link = entries.iter().find(|e| e.name == "link").unwrap();
        assert_eq!(link.kind, FileKind::Symlink);
        assert!(link.link_target.is_some());
    }

    #[test]
    fn executable_detection() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("script.sh");
        File::create(&file_path).unwrap();

        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&file_path, fs::Permissions::from_mode(0o755)).unwrap();

        let entries = list_directory(dir.path(), false).unwrap();
        assert!(entries[0].is_executable);
    }
}
