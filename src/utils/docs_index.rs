use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Section in a markdown file
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Section {
    /// Heading text (without # prefix)
    pub heading: String,
    /// Heading level (1-6)
    pub level: u8,
    /// Start line (1-indexed)
    pub start_line: usize,
    /// End line (exclusive, 0 means end of file)
    pub end_line: usize,
}

impl Section {
    pub fn new(heading: String, level: u8, start_line: usize) -> Self {
        Self {
            heading,
            level,
            start_line,
            end_line: 0,
        }
    }
}

/// Index of a single markdown file
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileIndex {
    /// File path (relative to index root)
    pub path: String,
    /// Total line count
    pub line_count: usize,
    /// Sections in the file
    pub sections: Vec<Section>,
}

impl FileIndex {
    pub fn new(path: String, line_count: usize) -> Self {
        Self {
            path,
            line_count,
            sections: Vec::new(),
        }
    }
}

/// Index of all markdown files in a directory
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct DocsIndex {
    /// Root directory path
    pub root: String,
    /// Map of relative path to file index
    pub files: HashMap<String, FileIndex>,
}

impl DocsIndex {
    pub fn new(root: String) -> Self {
        Self {
            root,
            files: HashMap::new(),
        }
    }

    /// Add a file index
    pub fn add_file(&mut self, index: FileIndex) {
        self.files.insert(index.path.clone(), index);
    }

    /// Get file count
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Get total section count
    pub fn section_count(&self) -> usize {
        self.files.values().map(|f| f.sections.len()).sum()
    }
}

/// Build an index for markdown files in a directory
pub fn build_index(dir: &str) -> Result<DocsIndex> {
    let root = Path::new(dir);
    if !root.is_dir() {
        anyhow::bail!("Not a directory: {}", dir);
    }

    let mut index = DocsIndex::new(dir.to_string());
    index_directory(root, root, &mut index)?;

    Ok(index)
}

/// Recursively index a directory
fn index_directory(root: &Path, dir: &Path, index: &mut DocsIndex) -> Result<()> {
    let entries = fs::read_dir(dir)
        .with_context(|| format!("Failed to read directory: {}", dir.display()))?;

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Skip hidden directories
        if name.starts_with('.') {
            continue;
        }

        if path.is_dir() {
            index_directory(root, &path, index)?;
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ext == "md" || ext == "markdown" {
                let file_index = index_file(root, &path)?;
                index.add_file(file_index);
            }
        }
    }

    Ok(())
}

/// Index a single markdown file
fn index_file(root: &Path, path: &Path) -> Result<FileIndex> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;

    let relative_path = path
        .strip_prefix(root)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string_lossy().to_string());

    let line_count = content.lines().count();
    let mut file_index = FileIndex::new(relative_path, line_count);

    // Parse headings
    let heading_re = Regex::new(r"^(#{1,6})\s+(.+)$").unwrap();
    let mut sections: Vec<Section> = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let line_num = line_num + 1;

        if let Some(caps) = heading_re.captures(line) {
            let level = caps.get(1).unwrap().as_str().len() as u8;
            let heading = caps.get(2).unwrap().as_str().to_string();

            // Close the most recent unclosed section
            // Each section ends when the next heading starts, regardless of level
            // This gives simple non-overlapping ranges for extraction
            if let Some(last) = sections.iter_mut().rev().find(|s| s.end_line == 0) {
                last.end_line = line_num;
            }

            sections.push(Section::new(heading, level, line_num));
        }
    }

    // Close remaining open sections at end of file
    for section in &mut sections {
        if section.end_line == 0 {
            section.end_line = line_count + 1;
        }
    }

    file_index.sections = sections;
    Ok(file_index)
}

/// Save index to JSON file
pub fn save_index(index: &DocsIndex, path: &str) -> Result<()> {
    let json = serde_json::to_string_pretty(index).context("Failed to serialize index")?;
    fs::write(path, json).with_context(|| format!("Failed to write index to {}", path))?;
    Ok(())
}

/// Load index from JSON file
pub fn load_index(path: &str) -> Result<DocsIndex> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read index from {}", path))?;
    serde_json::from_str(&content).with_context(|| format!("Failed to parse index from {}", path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_new() {
        let section = Section::new("Test".to_string(), 2, 5);
        assert_eq!(section.heading, "Test");
        assert_eq!(section.level, 2);
        assert_eq!(section.start_line, 5);
        assert_eq!(section.end_line, 0);
    }

    #[test]
    fn section_clone() {
        let section = Section::new("Test".to_string(), 1, 1);
        let cloned = section.clone();
        assert_eq!(section, cloned);
    }

    #[test]
    fn section_debug() {
        let section = Section::new("Test".to_string(), 1, 1);
        let debug = format!("{:?}", section);
        assert!(debug.contains("Section"));
    }

    #[test]
    fn section_serialize() {
        let section = Section::new("Test".to_string(), 1, 1);
        let json = serde_json::to_string(&section).unwrap();
        let parsed: Section = serde_json::from_str(&json).unwrap();
        assert_eq!(section, parsed);
    }

    #[test]
    fn file_index_new() {
        let index = FileIndex::new("test.md".to_string(), 100);
        assert_eq!(index.path, "test.md");
        assert_eq!(index.line_count, 100);
        assert!(index.sections.is_empty());
    }

    #[test]
    fn file_index_clone() {
        let index = FileIndex::new("test.md".to_string(), 50);
        let cloned = index.clone();
        assert_eq!(index, cloned);
    }

    #[test]
    fn file_index_debug() {
        let index = FileIndex::new("test.md".to_string(), 50);
        let debug = format!("{:?}", index);
        assert!(debug.contains("FileIndex"));
    }

    #[test]
    fn file_index_serialize() {
        let mut index = FileIndex::new("test.md".to_string(), 50);
        index
            .sections
            .push(Section::new("Heading".to_string(), 1, 1));
        let json = serde_json::to_string(&index).unwrap();
        let parsed: FileIndex = serde_json::from_str(&json).unwrap();
        assert_eq!(index, parsed);
    }

    #[test]
    fn docs_index_new() {
        let index = DocsIndex::new("./docs".to_string());
        assert_eq!(index.root, "./docs");
        assert!(index.files.is_empty());
    }

    #[test]
    fn docs_index_default() {
        let index = DocsIndex::default();
        assert_eq!(index.root, "");
        assert!(index.files.is_empty());
    }

    #[test]
    fn docs_index_add_file() {
        let mut index = DocsIndex::new("./".to_string());
        index.add_file(FileIndex::new("test.md".to_string(), 50));
        assert_eq!(index.file_count(), 1);
    }

    #[test]
    fn docs_index_counts() {
        let mut index = DocsIndex::new("./".to_string());
        let mut file1 = FileIndex::new("a.md".to_string(), 50);
        file1.sections.push(Section::new("H1".to_string(), 1, 1));
        file1.sections.push(Section::new("H2".to_string(), 2, 10));
        let mut file2 = FileIndex::new("b.md".to_string(), 30);
        file2.sections.push(Section::new("Intro".to_string(), 1, 1));

        index.add_file(file1);
        index.add_file(file2);

        assert_eq!(index.file_count(), 2);
        assert_eq!(index.section_count(), 3);
    }

    #[test]
    fn docs_index_clone() {
        let index = DocsIndex::new("./".to_string());
        let cloned = index.clone();
        assert_eq!(index, cloned);
    }

    #[test]
    fn docs_index_debug() {
        let index = DocsIndex::new("./".to_string());
        let debug = format!("{:?}", index);
        assert!(debug.contains("DocsIndex"));
    }

    #[test]
    fn docs_index_serialize() {
        let mut index = DocsIndex::new("./docs".to_string());
        index.add_file(FileIndex::new("test.md".to_string(), 50));
        let json = serde_json::to_string(&index).unwrap();
        let parsed: DocsIndex = serde_json::from_str(&json).unwrap();
        assert_eq!(index, parsed);
    }

    // Test indexing with temp directory
    fn create_test_docs(suffix: &str) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);

        let tmp_dir = std::env::temp_dir().join(format!(
            "hu_docs_test_{}_{}_{suffix}",
            std::process::id(),
            id
        ));
        let _ = std::fs::remove_dir_all(&tmp_dir);
        std::fs::create_dir_all(&tmp_dir).unwrap();

        // Create test markdown files
        std::fs::write(
            tmp_dir.join("README.md"),
            "# Project\n\nIntroduction.\n\n## Setup\n\nSetup steps.\n\n## Usage\n\nUsage info.\n",
        )
        .unwrap();

        std::fs::create_dir_all(tmp_dir.join("docs")).unwrap();
        std::fs::write(
            tmp_dir.join("docs/api.md"),
            "# API Reference\n\n## Endpoints\n\nList of endpoints.\n",
        )
        .unwrap();

        tmp_dir
    }

    fn cleanup_test_docs(path: &std::path::Path) {
        let _ = std::fs::remove_dir_all(path);
    }

    #[test]
    fn build_index_basic() {
        let tmp_dir = create_test_docs("test");
        let index = build_index(tmp_dir.to_str().unwrap()).unwrap();

        assert_eq!(index.file_count(), 2);
        assert!(index.files.contains_key("README.md"));
        assert!(index.files.contains_key("docs/api.md"));

        cleanup_test_docs(&tmp_dir);
    }

    #[test]
    fn build_index_sections() {
        let tmp_dir = create_test_docs("test");
        let index = build_index(tmp_dir.to_str().unwrap()).unwrap();

        let readme = index.files.get("README.md").unwrap();
        assert_eq!(readme.sections.len(), 3);
        assert_eq!(readme.sections[0].heading, "Project");
        assert_eq!(readme.sections[0].level, 1);
        assert_eq!(readme.sections[1].heading, "Setup");
        assert_eq!(readme.sections[1].level, 2);

        cleanup_test_docs(&tmp_dir);
    }

    #[test]
    fn build_index_section_ranges() {
        let tmp_dir = create_test_docs("test");
        let index = build_index(tmp_dir.to_str().unwrap()).unwrap();

        let readme = index.files.get("README.md").unwrap();
        // "# Project" starts at line 1, ends when "## Setup" starts at line 5
        assert_eq!(readme.sections[0].start_line, 1);
        assert_eq!(readme.sections[0].end_line, 5);
        // "## Setup" ends when "## Usage" starts
        assert_eq!(readme.sections[1].start_line, 5);
        assert_eq!(readme.sections[1].end_line, 9);

        cleanup_test_docs(&tmp_dir);
    }

    #[test]
    fn build_index_not_directory() {
        let result = build_index("/nonexistent/path");
        assert!(result.is_err());
    }

    #[test]
    fn save_and_load_index() {
        let tmp_dir = create_test_docs("test");
        let index = build_index(tmp_dir.to_str().unwrap()).unwrap();

        let index_path = tmp_dir.join("index.json");
        save_index(&index, index_path.to_str().unwrap()).unwrap();
        assert!(index_path.exists());

        let loaded = load_index(index_path.to_str().unwrap()).unwrap();
        assert_eq!(index, loaded);

        cleanup_test_docs(&tmp_dir);
    }

    #[test]
    fn load_index_not_found() {
        let result = load_index("/nonexistent/index.json");
        assert!(result.is_err());
    }

    #[test]
    fn build_index_skips_hidden_dirs() {
        let tmp_dir = create_test_docs("hidden");

        // Create a hidden directory with a markdown file
        let hidden_dir = tmp_dir.join(".hidden");
        std::fs::create_dir_all(&hidden_dir).unwrap();
        std::fs::write(hidden_dir.join("secret.md"), "# Secret\n\nHidden content.\n").unwrap();

        let index = build_index(tmp_dir.to_str().unwrap()).unwrap();

        // Should have 2 files (README.md and docs/api.md) but not .hidden/secret.md
        assert_eq!(index.file_count(), 2);
        assert!(!index.files.contains_key(".hidden/secret.md"));

        cleanup_test_docs(&tmp_dir);
    }
}
