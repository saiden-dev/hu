use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Representation of a documentation file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocEntry {
    /// Path to the file
    pub path: PathBuf,
    /// Document title (from first heading or filename)
    pub title: String,
    /// Source URL if fetched from web
    pub source: Option<String>,
    /// Date when document was fetched/created
    pub fetched: Option<String>,
    /// File size in bytes
    pub size: u64,
}

/// YAML frontmatter structure for documentation files
#[derive(Debug, Clone, Default)]
pub struct Frontmatter {
    /// Primary source URL
    pub source: Option<String>,
    /// Date when document was fetched
    pub fetched: Option<String>,
    /// Topic for scaffold files
    pub topic: Option<String>,
    /// Date when scaffold was created
    pub created: Option<String>,
}

impl Frontmatter {
    /// Parse frontmatter from markdown content (simple key: value parsing)
    pub fn parse(content: &str) -> Option<Self> {
        if !content.starts_with("---") {
            return None;
        }

        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() < 3 {
            return None;
        }

        let yaml = parts[1].trim();
        let mut fm = Frontmatter::default();

        for line in yaml.lines() {
            let line = line.trim();
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim().trim_matches('"').trim_matches('\'');
                match key {
                    "source" => fm.source = Some(value.to_string()),
                    "fetched" => fm.fetched = Some(value.to_string()),
                    "topic" => fm.topic = Some(value.to_string()),
                    "created" => fm.created = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        Some(fm)
    }

    /// Serialize frontmatter to YAML-like string
    pub fn to_yaml(&self) -> String {
        let mut lines = Vec::new();
        if let Some(ref source) = self.source {
            lines.push(format!("source: {}", source));
        }
        if let Some(ref fetched) = self.fetched {
            lines.push(format!("fetched: {}", fetched));
        }
        if let Some(ref topic) = self.topic {
            lines.push(format!("topic: \"{}\"", topic));
        }
        if let Some(ref created) = self.created {
            lines.push(format!("created: {}", created));
        }
        if !lines.is_empty() {
            lines.push(String::new()); // trailing newline
        }
        lines.join("\n")
    }

    /// Create frontmatter block for markdown
    pub fn to_block(&self) -> String {
        format!("---\n{}---\n", self.to_yaml())
    }
}

/// Convert a string to kebab-case slug
pub fn to_slug(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Extract title from markdown content (first heading)
pub fn extract_title(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix('#') {
            let title = rest.trim_start_matches('#').trim();
            if !title.is_empty() {
                return Some(title.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_slug_basic() {
        assert_eq!(to_slug("Hello World"), "hello-world");
    }

    #[test]
    fn to_slug_special_chars() {
        assert_eq!(to_slug("Rust's Best Practices!"), "rust-s-best-practices");
    }

    #[test]
    fn to_slug_multiple_spaces() {
        assert_eq!(to_slug("one   two  three"), "one-two-three");
    }

    #[test]
    fn to_slug_already_slug() {
        assert_eq!(to_slug("already-a-slug"), "already-a-slug");
    }

    #[test]
    fn to_slug_empty() {
        assert_eq!(to_slug(""), "");
    }

    #[test]
    fn to_slug_numbers() {
        assert_eq!(to_slug("Chapter 1: Introduction"), "chapter-1-introduction");
    }

    #[test]
    fn frontmatter_parse_valid() {
        let content = r#"---
source: https://example.com
fetched: 2024-01-01
---
# Content"#;
        let fm = Frontmatter::parse(content).unwrap();
        assert_eq!(fm.source, Some("https://example.com".to_string()));
        assert_eq!(fm.fetched, Some("2024-01-01".to_string()));
    }

    #[test]
    fn frontmatter_parse_no_frontmatter() {
        let content = "# Just Content";
        assert!(Frontmatter::parse(content).is_none());
    }

    #[test]
    fn frontmatter_parse_incomplete() {
        let content = "---\nsource: test";
        assert!(Frontmatter::parse(content).is_none());
    }

    #[test]
    fn frontmatter_to_yaml() {
        let fm = Frontmatter {
            source: Some("https://example.com".to_string()),
            fetched: Some("2024-01-01".to_string()),
            ..Default::default()
        };
        let yaml = fm.to_yaml();
        assert!(yaml.contains("source:"));
        assert!(yaml.contains("https://example.com"));
    }

    #[test]
    fn frontmatter_to_block() {
        let fm = Frontmatter {
            source: Some("https://test.com".to_string()),
            ..Default::default()
        };
        let block = fm.to_block();
        assert!(block.starts_with("---\n"));
        assert!(block.ends_with("---\n"));
    }

    #[test]
    fn frontmatter_skips_none_values() {
        let fm = Frontmatter {
            source: Some("url".to_string()),
            fetched: None,
            topic: None,
            created: None,
        };
        let yaml = fm.to_yaml();
        assert!(!yaml.contains("fetched:"));
        assert!(!yaml.contains("topic:"));
    }

    #[test]
    fn frontmatter_parse_ignores_unknown_keys() {
        let content = r#"---
source: https://example.com
unknown_key: some_value
another_unknown: test
fetched: 2024-01-01
---
# Content"#;
        let fm = Frontmatter::parse(content).unwrap();
        assert_eq!(fm.source, Some("https://example.com".to_string()));
        assert_eq!(fm.fetched, Some("2024-01-01".to_string()));
    }

    #[test]
    fn extract_title_h1() {
        let content = "# My Title\n\nContent here";
        assert_eq!(extract_title(content), Some("My Title".to_string()));
    }

    #[test]
    fn extract_title_h2() {
        let content = "## Second Level\n\nContent";
        assert_eq!(extract_title(content), Some("Second Level".to_string()));
    }

    #[test]
    fn extract_title_with_frontmatter() {
        let content = "---\nsource: url\n---\n\n# Actual Title";
        assert_eq!(extract_title(content), Some("Actual Title".to_string()));
    }

    #[test]
    fn extract_title_no_heading() {
        let content = "Just plain text\nNo headings here";
        assert!(extract_title(content).is_none());
    }

    #[test]
    fn extract_title_empty_heading() {
        let content = "#\n\nNo text after hash";
        assert!(extract_title(content).is_none());
    }

    #[test]
    fn doc_entry_serialize() {
        let entry = DocEntry {
            path: PathBuf::from("/docs/test.md"),
            title: "Test".to_string(),
            source: Some("https://example.com".to_string()),
            fetched: Some("2024-01-01".to_string()),
            size: 1234,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("Test"));
        assert!(json.contains("1234"));
    }

    #[test]
    fn doc_entry_debug() {
        let entry = DocEntry {
            path: PathBuf::from("test.md"),
            title: "Test".to_string(),
            source: None,
            fetched: None,
            size: 0,
        };
        let debug = format!("{:?}", entry);
        assert!(debug.contains("DocEntry"));
    }

    #[test]
    fn frontmatter_parse_with_source() {
        let content = r#"---
source: https://main.com
fetched: 2024-02-01
---
# Content"#;
        let fm = Frontmatter::parse(content).unwrap();
        assert_eq!(fm.source, Some("https://main.com".to_string()));
        assert_eq!(fm.fetched, Some("2024-02-01".to_string()));
    }

    #[test]
    fn frontmatter_parse_topic() {
        let content = r#"---
topic: Rust Error Handling
created: 2024-01-01
---
# Content"#;
        let fm = Frontmatter::parse(content).unwrap();
        assert_eq!(fm.topic, Some("Rust Error Handling".to_string()));
        assert_eq!(fm.created, Some("2024-01-01".to_string()));
    }
}
