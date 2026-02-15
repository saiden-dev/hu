use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub kind: FileKind,
    pub size: u64,
    #[serde(skip, default)]
    #[allow(dead_code)] // kept for future sorting
    pub modified: Option<SystemTime>,
    pub modified_str: String,
    pub permissions: String,
    pub is_hidden: bool,
    pub is_executable: bool,
    pub link_target: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileKind {
    Directory,
    File,
    Symlink,
    Socket,
    Fifo,
    BlockDevice,
    CharDevice,
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_kind_variants() {
        assert_eq!(FileKind::Directory, FileKind::Directory);
        assert_ne!(FileKind::File, FileKind::Directory);
    }

    #[test]
    fn file_entry_fields() {
        let entry = FileEntry {
            name: "test.txt".to_string(),
            kind: FileKind::File,
            size: 100,
            modified: None,
            modified_str: "Feb 15 12:00".to_string(),
            permissions: "-rw-r--r--".to_string(),
            is_hidden: false,
            is_executable: false,
            link_target: None,
        };
        assert_eq!(entry.name, "test.txt");
        assert_eq!(entry.kind, FileKind::File);
        assert_eq!(entry.size, 100);
    }

    #[test]
    fn hidden_file_detection() {
        let entry = FileEntry {
            name: ".gitignore".to_string(),
            kind: FileKind::File,
            size: 50,
            modified: None,
            modified_str: String::new(),
            permissions: String::new(),
            is_hidden: true,
            is_executable: false,
            link_target: None,
        };
        assert!(entry.is_hidden);
    }
}
