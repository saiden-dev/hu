use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[cfg(test)]
mod tests;

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
