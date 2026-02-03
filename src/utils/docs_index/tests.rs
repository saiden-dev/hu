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
    std::fs::write(
        hidden_dir.join("secret.md"),
        "# Secret\n\nHidden content.\n",
    )
    .unwrap();

    let index = build_index(tmp_dir.to_str().unwrap()).unwrap();

    // Should have 2 files (README.md and docs/api.md) but not .hidden/secret.md
    assert_eq!(index.file_count(), 2);
    assert!(!index.files.contains_key(".hidden/secret.md"));

    cleanup_test_docs(&tmp_dir);
}
