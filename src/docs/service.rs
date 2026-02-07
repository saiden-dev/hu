use anyhow::{Context, Result};
use chrono::Local;
use std::fs;
use std::path::{Path, PathBuf};

use super::types::{extract_title, to_slug, DocEntry, Frontmatter};
use crate::git::{self, SyncOptions};

/// Default docs directory
pub fn default_docs_dir() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join("Projects/docs"))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Create a scaffold file for a topic
pub fn add(topic: &str, output_dir: Option<&Path>, no_commit: bool) -> Result<PathBuf> {
    let dir = output_dir
        .map(PathBuf::from)
        .unwrap_or_else(default_docs_dir);
    fs::create_dir_all(&dir)?;

    let slug = to_slug(topic);
    let filename = format!("{}.md", slug);
    let path = dir.join(&filename);

    if path.exists() {
        anyhow::bail!("File already exists: {}", path.display());
    }

    let date = Local::now().format("%Y-%m-%d").to_string();
    let frontmatter = Frontmatter {
        topic: Some(topic.to_string()),
        created: Some(date),
        ..Default::default()
    };

    let content = format!(
        "{}
# {}

<!-- Research and document this topic -->
",
        frontmatter.to_block(),
        topic
    );

    fs::write(&path, content).with_context(|| format!("Failed to write {}", path.display()))?;

    if !no_commit {
        commit_file(&dir, &filename, &format!("Add {}", topic))?;
    }

    Ok(path)
}

/// Fetch documentation from a URL
pub async fn get(
    url: &str,
    name: Option<&str>,
    output_dir: Option<&Path>,
    no_commit: bool,
) -> Result<PathBuf> {
    let dir = output_dir
        .map(PathBuf::from)
        .unwrap_or_else(default_docs_dir);
    fs::create_dir_all(&dir)?;

    // Derive filename from URL or use provided name
    let slug = name.map(to_slug).unwrap_or_else(|| slug_from_url(url));
    let filename = format!("{}.md", slug);
    let path = dir.join(&filename);

    // Fetch content
    let html = fetch_url(url).await?;
    let markdown = html_to_markdown(&html);

    let date = Local::now().format("%Y-%m-%d").to_string();
    let frontmatter = Frontmatter {
        source: Some(url.to_string()),
        fetched: Some(date),
        ..Default::default()
    };

    let content = format!("{}\n{}", frontmatter.to_block(), markdown);

    fs::write(&path, content).with_context(|| format!("Failed to write {}", path.display()))?;

    if !no_commit {
        let title = extract_title(&markdown).unwrap_or_else(|| slug.clone());
        commit_file(&dir, &filename, &format!("Add docs: {}", title))?;
    }

    Ok(path)
}

/// List documentation files in a directory
pub fn list(path: Option<&Path>) -> Result<Vec<DocEntry>> {
    let dir = path.map(PathBuf::from).unwrap_or_else(default_docs_dir);

    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut entries = Vec::new();
    collect_docs(&dir, &mut entries)?;
    entries.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(entries)
}

/// Recursively collect documentation files
fn collect_docs(dir: &Path, entries: &mut Vec<DocEntry>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            collect_docs(&path, entries)?;
        } else if path.extension().is_some_and(|e| e == "md") {
            if let Ok(entry) = parse_doc_entry(&path) {
                entries.push(entry);
            }
        }
    }
    Ok(())
}

/// Parse a documentation file into a DocEntry
fn parse_doc_entry(path: &Path) -> Result<DocEntry> {
    let content = fs::read_to_string(path)?;
    let metadata = fs::metadata(path)?;

    let frontmatter = Frontmatter::parse(&content);
    let title = extract_title(&content)
        .or_else(|| frontmatter.as_ref().and_then(|f| f.topic.clone()))
        .unwrap_or_else(|| {
            path.file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default()
        });

    Ok(DocEntry {
        path: path.to_path_buf(),
        title,
        source: frontmatter.as_ref().and_then(|f| f.source.clone()),
        fetched: frontmatter
            .as_ref()
            .and_then(|f| f.fetched.clone().or_else(|| f.created.clone())),
        size: metadata.len(),
    })
}

/// Remove a documentation file
pub fn remove(file: &str, base_dir: Option<&Path>, no_commit: bool) -> Result<PathBuf> {
    let dir = base_dir.map(PathBuf::from).unwrap_or_else(default_docs_dir);

    // Try to resolve the file path
    let path = resolve_file_path(file, &dir)?;

    if !path.exists() {
        anyhow::bail!("File not found: {}", path.display());
    }

    let filename = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    fs::remove_file(&path).with_context(|| format!("Failed to remove {}", path.display()))?;

    if !no_commit {
        commit_removal(&dir, &filename)?;
    }

    Ok(path)
}

/// Resolve file path from slug or relative/absolute path
fn resolve_file_path(file: &str, base_dir: &Path) -> Result<PathBuf> {
    let path = PathBuf::from(file);

    // If absolute path, use it directly
    if path.is_absolute() {
        return Ok(path);
    }

    // Try relative to base_dir
    let in_base = base_dir.join(&path);
    if in_base.exists() {
        return Ok(in_base);
    }

    // Try with .md extension
    let with_ext = base_dir.join(format!("{}.md", file));
    if with_ext.exists() {
        return Ok(with_ext);
    }

    // Try as slug
    let slug = to_slug(file);
    let as_slug = base_dir.join(format!("{}.md", slug));
    if as_slug.exists() {
        return Ok(as_slug);
    }

    // Return the path in base_dir (will fail with "not found" later)
    Ok(in_base)
}

/// Sync (commit and push) documentation changes
pub fn sync(path: Option<&Path>, no_push: bool, message: Option<&str>) -> Result<git::SyncResult> {
    let dir = path.map(PathBuf::from).unwrap_or_else(default_docs_dir);

    let options = SyncOptions {
        no_commit: false,
        no_push,
        message: message.map(String::from),
        path: Some(dir),
    };

    git::sync(&options)
}

/// Commit a single file
fn commit_file(dir: &Path, filename: &str, message: &str) -> Result<()> {
    use std::process::Command;

    Command::new("git")
        .args(["add", filename])
        .current_dir(dir)
        .output()
        .context("Failed to stage file")?;

    Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(dir)
        .output()
        .context("Failed to commit")?;

    Ok(())
}

/// Commit a file removal
fn commit_removal(dir: &Path, filename: &str) -> Result<()> {
    use std::process::Command;

    Command::new("git")
        .args(["add", filename])
        .current_dir(dir)
        .output()
        .context("Failed to stage removal")?;

    Command::new("git")
        .args(["commit", "-m", &format!("Remove {}", filename)])
        .current_dir(dir)
        .output()
        .context("Failed to commit removal")?;

    Ok(())
}

/// Extract slug from URL
fn slug_from_url(url: &str) -> String {
    // Remove protocol
    let url = url
        .trim_start_matches("https://")
        .trim_start_matches("http://");

    // Get path parts
    let parts: Vec<&str> = url.split('/').filter(|s| !s.is_empty()).collect();

    // Try to get meaningful slug from path
    if parts.len() > 1 {
        // Use last path segment
        let last = parts.last().unwrap_or(&"doc");
        let slug = last.trim_end_matches(".html").trim_end_matches(".htm");
        return to_slug(slug);
    }

    // Use domain name
    if let Some(domain) = parts.first() {
        let domain = domain.split('.').next().unwrap_or("doc");
        return to_slug(domain);
    }

    "doc".to_string()
}

/// Fetch URL content (async)
async fn fetch_url(url: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .user_agent("hu-cli/0.1")
        .build()?;

    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to fetch {}", url))?;

    response
        .text()
        .await
        .with_context(|| format!("Failed to read response from {}", url))
}

/// Convert HTML to markdown (simplified version)
fn html_to_markdown(html: &str) -> String {
    use regex::Regex;

    let mut result = html.to_string();

    // Remove script, style, nav, footer
    for tag in &["script", "style", "noscript", "nav", "footer", "header"] {
        let re = Regex::new(&format!(r"(?is)<{}\b[^>]*>.*?</{}>", tag, tag)).unwrap();
        result = re.replace_all(&result, "").to_string();
    }

    // Convert headings
    for level in 1..=6 {
        let prefix = "#".repeat(level);
        let open_re = Regex::new(&format!(r"(?i)<h{}\b[^>]*>", level)).unwrap();
        let close_re = Regex::new(&format!(r"(?i)</h{}>", level)).unwrap();
        result = open_re
            .replace_all(&result, format!("\n{} ", prefix))
            .to_string();
        result = close_re.replace_all(&result, "\n").to_string();
    }

    // Convert links
    let link_re = Regex::new(r#"(?i)<a\s+[^>]*href=["']([^"']+)["'][^>]*>([^<]*)</a>"#).unwrap();
    result = link_re.replace_all(&result, "[$2]($1)").to_string();

    // Convert emphasis
    for tag in ["strong", "b"] {
        let re = Regex::new(&format!(r"(?i)<{}\b[^>]*>([^<]*)</{}>", tag, tag)).unwrap();
        result = re.replace_all(&result, "**$1**").to_string();
    }
    for tag in ["em", "i"] {
        let re = Regex::new(&format!(r"(?i)<{}\b[^>]*>([^<]*)</{}>", tag, tag)).unwrap();
        result = re.replace_all(&result, "*$1*").to_string();
    }

    // Convert code
    result = Regex::new(r"(?i)<code\b[^>]*>([^<]*)</code>")
        .unwrap()
        .replace_all(&result, "`$1`")
        .to_string();

    // Convert paragraphs
    result = Regex::new(r"(?i)<p\b[^>]*>")
        .unwrap()
        .replace_all(&result, "\n\n")
        .to_string();
    result = Regex::new(r"(?i)</p>")
        .unwrap()
        .replace_all(&result, "\n")
        .to_string();

    // Convert lists
    result = Regex::new(r"(?i)<li\b[^>]*>")
        .unwrap()
        .replace_all(&result, "\n- ")
        .to_string();
    result = Regex::new(r"(?i)</li>")
        .unwrap()
        .replace_all(&result, "")
        .to_string();

    // Remove remaining tags
    result = Regex::new(r"<[^>]+>")
        .unwrap()
        .replace_all(&result, "")
        .to_string();

    // Decode entities
    result = result
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");

    // Clean up whitespace
    result = Regex::new(r"\n{3,}")
        .unwrap()
        .replace_all(&result, "\n\n")
        .to_string();
    result = Regex::new(r"[ \t]+")
        .unwrap()
        .replace_all(&result, " ")
        .to_string();

    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn default_docs_dir_exists() {
        let dir = default_docs_dir();
        // Should return a path (may or may not exist)
        assert!(!dir.as_os_str().is_empty());
    }

    #[test]
    fn slug_from_url_path() {
        assert_eq!(slug_from_url("https://docs.rs/clap/latest/clap"), "clap");
    }

    #[test]
    fn slug_from_url_html() {
        assert_eq!(
            slug_from_url("https://example.com/guide/intro.html"),
            "intro"
        );
    }

    #[test]
    fn slug_from_url_domain_only() {
        assert_eq!(slug_from_url("https://example.com/"), "example");
    }

    #[test]
    fn slug_from_url_empty() {
        assert_eq!(slug_from_url(""), "doc");
    }

    #[test]
    fn add_creates_file() {
        let tmp = tempdir().unwrap();
        let path = add("Test Topic", Some(tmp.path()), true).unwrap();

        assert!(path.exists());
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("topic: \"Test Topic\""));
        assert!(content.contains("# Test Topic"));
    }

    #[test]
    fn add_fails_if_exists() {
        let tmp = tempdir().unwrap();
        let _ = add("Duplicate", Some(tmp.path()), true).unwrap();
        let result = add("Duplicate", Some(tmp.path()), true);
        assert!(result.is_err());
    }

    #[test]
    fn list_empty_dir() {
        let tmp = tempdir().unwrap();
        let entries = list(Some(tmp.path())).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn list_with_docs() {
        let tmp = tempdir().unwrap();
        let _ = add("First", Some(tmp.path()), true).unwrap();
        let _ = add("Second", Some(tmp.path()), true).unwrap();

        let entries = list(Some(tmp.path())).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn list_nonexistent_dir() {
        let entries = list(Some(Path::new("/nonexistent/path/docs"))).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn remove_deletes_file() {
        let tmp = tempdir().unwrap();
        let path = add("ToRemove", Some(tmp.path()), true).unwrap();
        assert!(path.exists());

        remove("toremove.md", Some(tmp.path()), true).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn remove_by_slug() {
        let tmp = tempdir().unwrap();
        let path = add("By Slug", Some(tmp.path()), true).unwrap();
        assert!(path.exists());

        remove("by-slug", Some(tmp.path()), true).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn remove_not_found() {
        let tmp = tempdir().unwrap();
        let result = remove("nonexistent.md", Some(tmp.path()), true);
        assert!(result.is_err());
    }

    #[test]
    fn resolve_file_path_absolute() {
        let path = resolve_file_path("/absolute/path.md", Path::new("/base")).unwrap();
        assert_eq!(path, PathBuf::from("/absolute/path.md"));
    }

    #[test]
    fn resolve_file_path_with_ext() {
        let tmp = tempdir().unwrap();
        fs::write(tmp.path().join("test.md"), "# Test").unwrap();

        let path = resolve_file_path("test.md", tmp.path()).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn resolve_file_path_without_ext() {
        let tmp = tempdir().unwrap();
        fs::write(tmp.path().join("test.md"), "# Test").unwrap();

        let path = resolve_file_path("test", tmp.path()).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn parse_doc_entry_basic() {
        let tmp = tempdir().unwrap();
        let file = tmp.path().join("test.md");
        fs::write(&file, "# My Title\n\nContent").unwrap();

        let entry = parse_doc_entry(&file).unwrap();
        assert_eq!(entry.title, "My Title");
        assert!(entry.source.is_none());
    }

    #[test]
    fn parse_doc_entry_with_frontmatter() {
        let tmp = tempdir().unwrap();
        let file = tmp.path().join("test.md");
        fs::write(
            &file,
            "---\nsource: https://example.com\nfetched: 2024-01-01\n---\n# Title",
        )
        .unwrap();

        let entry = parse_doc_entry(&file).unwrap();
        assert_eq!(entry.source, Some("https://example.com".to_string()));
        assert_eq!(entry.fetched, Some("2024-01-01".to_string()));
    }

    #[test]
    fn parse_doc_entry_title_from_topic() {
        let tmp = tempdir().unwrap();
        let file = tmp.path().join("test.md");
        fs::write(&file, "---\ntopic: My Topic\n---\n\nNo heading").unwrap();

        let entry = parse_doc_entry(&file).unwrap();
        assert_eq!(entry.title, "My Topic");
    }

    #[test]
    fn parse_doc_entry_title_from_filename() {
        let tmp = tempdir().unwrap();
        let file = tmp.path().join("my-file.md");
        fs::write(&file, "No frontmatter, no heading").unwrap();

        let entry = parse_doc_entry(&file).unwrap();
        assert_eq!(entry.title, "my-file");
    }

    #[test]
    fn html_to_markdown_headings() {
        let html = "<h1>Title</h1><h2>Subtitle</h2>";
        let md = html_to_markdown(html);
        assert!(md.contains("# Title"));
        assert!(md.contains("## Subtitle"));
    }

    #[test]
    fn html_to_markdown_links() {
        let html = r#"<a href="https://example.com">Link</a>"#;
        let md = html_to_markdown(html);
        assert!(md.contains("[Link](https://example.com)"));
    }

    #[test]
    fn html_to_markdown_emphasis() {
        let html = "<strong>bold</strong> and <em>italic</em>";
        let md = html_to_markdown(html);
        assert!(md.contains("**bold**"));
        assert!(md.contains("*italic*"));
    }

    #[test]
    fn html_to_markdown_code() {
        let html = "<code>inline code</code>";
        let md = html_to_markdown(html);
        assert!(md.contains("`inline code`"));
    }

    #[test]
    fn html_to_markdown_strips_scripts() {
        let html = "<script>alert('x')</script><p>Content</p>";
        let md = html_to_markdown(html);
        assert!(!md.contains("script"));
        assert!(md.contains("Content"));
    }

    #[test]
    fn html_to_markdown_entities() {
        let html = "&amp; &lt; &gt; &quot;";
        let md = html_to_markdown(html);
        assert!(md.contains("& < > \""));
    }

    #[test]
    fn sync_not_git_repo() {
        let tmp = tempdir().unwrap();
        let result = sync(Some(tmp.path()), false, None);
        assert!(result.is_err());
    }

    #[test]
    fn collect_docs_recursive() {
        let tmp = tempdir().unwrap();
        let subdir = tmp.path().join("sub");
        fs::create_dir(&subdir).unwrap();

        fs::write(tmp.path().join("root.md"), "# Root").unwrap();
        fs::write(subdir.join("nested.md"), "# Nested").unwrap();

        let mut entries = Vec::new();
        collect_docs(tmp.path(), &mut entries).unwrap();

        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn collect_docs_ignores_non_md() {
        let tmp = tempdir().unwrap();
        fs::write(tmp.path().join("doc.md"), "# Doc").unwrap();
        fs::write(tmp.path().join("other.txt"), "text").unwrap();

        let mut entries = Vec::new();
        collect_docs(tmp.path(), &mut entries).unwrap();

        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn list_sorted_by_path() {
        let tmp = tempdir().unwrap();
        fs::write(tmp.path().join("z-last.md"), "# Z").unwrap();
        fs::write(tmp.path().join("a-first.md"), "# A").unwrap();

        let entries = list(Some(tmp.path())).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries[0].path.to_string_lossy().contains("a-first"));
    }
}
