use anyhow::{Context, Result};
use regex::Regex;
use std::fs;

use super::cli::FetchHtmlArgs;

/// Handle the `hu utils fetch-html` command
pub async fn run(args: FetchHtmlArgs) -> Result<()> {
    let html = fetch_url(&args.url).await?;

    let output = if args.raw {
        html_to_markdown(&html)
    } else if args.links {
        extract_links(&html)
    } else if args.headings {
        extract_headings(&html)
    } else if args.summary {
        extract_summary(&html)
    } else if args.content || args.selector.is_some() {
        let selector = args.selector.as_deref();
        extract_content(&html, selector)
    } else {
        // Default: content extraction
        extract_content(&html, None)
    };

    if let Some(path) = args.output {
        fs::write(&path, &output).with_context(|| format!("Failed to write to {}", path))?;
        eprintln!("Written to {}", path);
    } else {
        println!("{}", output);
    }

    Ok(())
}

/// Fetch URL content
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

/// Convert HTML to markdown (basic conversion)
pub fn html_to_markdown(html: &str) -> String {
    let mut result = html.to_string();

    // Remove script and style tags with content
    result = remove_tag_with_content(&result, "script");
    result = remove_tag_with_content(&result, "style");
    result = remove_tag_with_content(&result, "noscript");

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

    // Convert emphasis (strong/b -> **, em/i -> *)
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

    // Convert pre/code blocks
    let pre_re = Regex::new(r"(?is)<pre\b[^>]*><code\b[^>]*>(.+?)</code></pre>").unwrap();
    result = pre_re.replace_all(&result, "\n```\n$1\n```\n").to_string();

    // Convert lists
    result = Regex::new(r"(?i)<li\b[^>]*>")
        .unwrap()
        .replace_all(&result, "\n- ")
        .to_string();
    result = Regex::new(r"(?i)</li>")
        .unwrap()
        .replace_all(&result, "")
        .to_string();

    // Convert paragraphs and line breaks
    result = Regex::new(r"(?i)<p\b[^>]*>")
        .unwrap()
        .replace_all(&result, "\n\n")
        .to_string();
    result = Regex::new(r"(?i)</p>")
        .unwrap()
        .replace_all(&result, "\n")
        .to_string();
    result = Regex::new(r"(?i)<br\s*/?>")
        .unwrap()
        .replace_all(&result, "\n")
        .to_string();

    // Remove remaining HTML tags
    result = Regex::new(r"<[^>]+>")
        .unwrap()
        .replace_all(&result, "")
        .to_string();

    // Decode common HTML entities
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

/// Remove HTML tag and its content
fn remove_tag_with_content(html: &str, tag: &str) -> String {
    let re = Regex::new(&format!(r"(?is)<{}\b[^>]*>.*?</{}>", tag, tag)).unwrap();
    re.replace_all(html, "").to_string()
}

/// Extract main content only (strip nav, footer, scripts, ads)
pub fn extract_content(html: &str, selector: Option<&str>) -> String {
    let mut result = html.to_string();

    // If selector provided, try to extract just that
    if let Some(sel) = selector {
        if let Some(content) = extract_by_selector(&result, sel) {
            return html_to_markdown(&content);
        }
    }

    // Remove noise elements
    for tag in &[
        "script", "style", "noscript", "nav", "footer", "header", "aside", "iframe", "svg",
    ] {
        result = remove_tag_with_content(&result, tag);
    }

    // Remove common ad/noise classes
    let noise_patterns = [
        r#"<[^>]+class="[^"]*(?:ad|advertisement|sidebar|menu|nav|footer|header|cookie|popup|modal|banner)[^"]*"[^>]*>.*?</[^>]+>"#,
        r#"<[^>]+id="[^"]*(?:ad|advertisement|sidebar|menu|nav|footer|header|cookie|popup|modal|banner)[^"]*"[^>]*>.*?</[^>]+>"#,
    ];

    for pattern in &noise_patterns {
        if let Ok(re) = Regex::new(&format!("(?is){}", pattern)) {
            result = re.replace_all(&result, "").to_string();
        }
    }

    // Try to find main content area
    if let Some(main) = extract_by_selector(&result, "main") {
        return html_to_markdown(&main);
    }
    if let Some(article) = extract_by_selector(&result, "article") {
        return html_to_markdown(&article);
    }
    if let Some(content) = extract_by_selector(&result, ".content") {
        return html_to_markdown(&content);
    }

    html_to_markdown(&result)
}

/// Try to extract content by CSS-like selector (simplified)
fn extract_by_selector(html: &str, selector: &str) -> Option<String> {
    let pattern = if let Some(class) = selector.strip_prefix('.') {
        // Class selector
        format!(
            r#"(?is)<[^>]+class="[^"]*\b{}\b[^"]*"[^>]*>(.*?)</[^>]+>"#,
            regex::escape(class)
        )
    } else if let Some(id) = selector.strip_prefix('#') {
        // ID selector
        format!(
            r#"(?is)<[^>]+id="{}"[^>]*>(.*?)</[^>]+>"#,
            regex::escape(id)
        )
    } else {
        // Tag selector
        format!(
            r"(?is)<{}\b[^>]*>(.*?)</{}>",
            regex::escape(selector),
            regex::escape(selector)
        )
    };

    Regex::new(&pattern)
        .ok()
        .and_then(|re| re.captures(html))
        .map(|caps| caps.get(1).map_or("", |m| m.as_str()).to_string())
}

/// Extract links only
pub fn extract_links(html: &str) -> String {
    let link_re = Regex::new(r#"(?i)<a\s+[^>]*href=["']([^"']+)["'][^>]*>([^<]*)</a>"#).unwrap();

    let mut links = Vec::new();
    for cap in link_re.captures_iter(html) {
        let url = cap.get(1).map_or("", |m| m.as_str());
        let text = cap.get(2).map_or("", |m| m.as_str()).trim();

        // Skip empty links, anchors, javascript
        if url.is_empty()
            || url.starts_with('#')
            || url.starts_with("javascript:")
            || text.is_empty()
        {
            continue;
        }

        links.push(format!("- [{}]({})", text, url));
    }

    links.join("\n")
}

/// Extract headings only (document outline)
pub fn extract_headings(html: &str) -> String {
    let mut headings = Vec::new();
    let strip_tags_re = Regex::new(r"<[^>]+>").unwrap();

    for level in 1..=6 {
        let re = Regex::new(&format!(r"(?is)<h{}\b[^>]*>(.*?)</h{}>", level, level)).unwrap();

        for cap in re.captures_iter(html) {
            let text = cap.get(1).map_or("", |m| m.as_str());
            // Strip any nested tags
            let clean = strip_tags_re.replace_all(text, "").trim().to_string();

            if !clean.is_empty() {
                let indent = "  ".repeat(level - 1);
                headings.push(format!("{}{} {}", indent, "#".repeat(level), clean));
            }
        }
    }

    headings.join("\n")
}

/// Extract summary (first N paragraphs + all headings)
pub fn extract_summary(html: &str) -> String {
    let content = extract_content(html, None);
    let lines: Vec<&str> = content.lines().collect();

    let mut result = Vec::new();
    let mut para_count = 0;
    let max_paras = 3;

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Always include headings
        if trimmed.starts_with('#') {
            result.push(line.to_string());
            continue;
        }

        // Include first N paragraphs
        if para_count < max_paras {
            result.push(line.to_string());
            if !trimmed.starts_with('-') && !trimmed.starts_with('*') {
                para_count += 1;
            }
        }
    }

    result.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_to_markdown_headings() {
        let html = "<h1>Title</h1><h2>Subtitle</h2>";
        let md = html_to_markdown(html);
        assert!(md.contains("# Title"));
        assert!(md.contains("## Subtitle"));
    }

    #[test]
    fn html_to_markdown_links() {
        let html = r#"<a href="https://example.com">Click here</a>"#;
        let md = html_to_markdown(html);
        assert!(md.contains("[Click here](https://example.com)"));
    }

    #[test]
    fn html_to_markdown_emphasis() {
        let html = "<strong>bold</strong> and <em>italic</em>";
        let md = html_to_markdown(html);
        assert!(md.contains("**bold**"));
        assert!(md.contains("*italic*"));
    }

    #[test]
    fn html_to_markdown_strips_scripts() {
        let html = "<p>Text</p><script>alert('x')</script><p>More</p>";
        let md = html_to_markdown(html);
        assert!(!md.contains("alert"));
        assert!(md.contains("Text"));
        assert!(md.contains("More"));
    }

    #[test]
    fn extract_links_basic() {
        let html = r##"
            <a href="https://a.com">Link A</a>
            <a href="https://b.com">Link B</a>
            <a href="#">Skip</a>
        "##;
        let links = extract_links(html);
        assert!(links.contains("[Link A](https://a.com)"));
        assert!(links.contains("[Link B](https://b.com)"));
        assert!(!links.contains("Skip"));
    }

    #[test]
    fn extract_headings_basic() {
        let html = "<h1>Main</h1><h2>Sub</h2><h3>Deep</h3>";
        let headings = extract_headings(html);
        assert!(headings.contains("# Main"));
        assert!(headings.contains("## Sub"));
        assert!(headings.contains("### Deep"));
    }

    #[test]
    fn extract_by_selector_tag() {
        let html = "<main><p>Content</p></main>";
        let content = extract_by_selector(html, "main");
        assert!(content.is_some());
        assert!(content.unwrap().contains("Content"));
    }

    #[test]
    fn extract_by_selector_class() {
        let html = r#"<div class="content"><p>Inner</p></div>"#;
        let content = extract_by_selector(html, ".content");
        assert!(content.is_some());
        assert!(content.unwrap().contains("Inner"));
    }

    #[test]
    fn remove_tag_with_content_basic() {
        let html = "<p>Keep</p><nav>Remove</nav><p>Also keep</p>";
        let result = remove_tag_with_content(html, "nav");
        assert!(result.contains("Keep"));
        assert!(result.contains("Also keep"));
        assert!(!result.contains("Remove"));
    }

    #[test]
    fn extract_summary_limits_paragraphs() {
        let html = "<p>Para 1</p><p>Para 2</p><p>Para 3</p><p>Para 4</p><p>Para 5</p>";
        let summary = extract_summary(html);
        assert!(summary.contains("Para 1"));
        assert!(summary.contains("Para 2"));
        assert!(summary.contains("Para 3"));
        // Should be limited
    }

    #[test]
    fn html_to_markdown_inline_code() {
        let html = "<p>Use <code>foo()</code> method</p>";
        let md = html_to_markdown(html);
        assert!(md.contains("`foo()`"));
    }

    #[test]
    fn html_to_markdown_lists() {
        let html = "<ul><li>Item 1</li><li>Item 2</li></ul>";
        let md = html_to_markdown(html);
        assert!(md.contains("- Item 1"));
        assert!(md.contains("- Item 2"));
    }

    #[test]
    fn html_to_markdown_paragraphs() {
        let html = "<p>First paragraph</p><p>Second paragraph</p>";
        let md = html_to_markdown(html);
        assert!(md.contains("First paragraph"));
        assert!(md.contains("Second paragraph"));
    }

    #[test]
    fn html_to_markdown_br_tags() {
        let html = "Line 1<br/>Line 2<br>Line 3";
        let md = html_to_markdown(html);
        assert!(md.contains("Line 1"));
        assert!(md.contains("Line 2"));
        assert!(md.contains("Line 3"));
    }

    #[test]
    fn html_to_markdown_entities() {
        let html = "5 &lt; 10 &amp; 10 &gt; 5";
        let md = html_to_markdown(html);
        assert!(md.contains("5 < 10 & 10 > 5"));
    }

    #[test]
    fn html_to_markdown_b_and_i_tags() {
        let html = "<b>bold</b> and <i>italic</i>";
        let md = html_to_markdown(html);
        assert!(md.contains("**bold**"));
        assert!(md.contains("*italic*"));
    }

    #[test]
    fn html_to_markdown_noscript() {
        let html = "<p>Content</p><noscript>Enable JS</noscript>";
        let md = html_to_markdown(html);
        assert!(md.contains("Content"));
        assert!(!md.contains("Enable JS"));
    }

    #[test]
    fn html_to_markdown_style() {
        let html = "<style>body { color: red; }</style><p>Text</p>";
        let md = html_to_markdown(html);
        assert!(!md.contains("color"));
        assert!(md.contains("Text"));
    }

    #[test]
    fn extract_links_skips_javascript() {
        let html = r#"<a href="javascript:void(0)">JS Link</a><a href="https://x.com">Real</a>"#;
        let links = extract_links(html);
        assert!(!links.contains("JS Link"));
        assert!(links.contains("Real"));
    }

    #[test]
    fn extract_links_skips_empty_text() {
        let html = r#"<a href="https://x.com"></a><a href="https://y.com">Valid</a>"#;
        let links = extract_links(html);
        assert!(!links.contains("https://x.com")); // skipped - empty text
        assert!(links.contains("Valid"));
    }

    #[test]
    fn extract_by_selector_id() {
        let html = r#"<div id="main"><p>Main content</p></div>"#;
        let content = extract_by_selector(html, "#main");
        assert!(content.is_some());
        assert!(content.unwrap().contains("Main content"));
    }

    #[test]
    fn extract_by_selector_not_found() {
        let html = "<p>Just text</p>";
        let content = extract_by_selector(html, "#nonexistent");
        assert!(content.is_none());
    }

    #[test]
    fn extract_content_with_selector() {
        let html = r#"<nav>Skip</nav><div class="content"><p>Keep</p></div>"#;
        let content = extract_content(html, Some(".content"));
        assert!(content.contains("Keep"));
        assert!(!content.contains("Skip"));
    }

    #[test]
    fn extract_content_strips_noise_elements() {
        let html = "<nav>Nav</nav><script>alert()</script><main><p>Main</p></main>";
        let content = extract_content(html, None);
        assert!(content.contains("Main"));
        assert!(!content.contains("Nav"));
        assert!(!content.contains("alert"));
    }

    #[test]
    fn extract_content_finds_article() {
        let html = "<header>Header</header><article><p>Article</p></article>";
        let content = extract_content(html, None);
        assert!(content.contains("Article"));
    }

    #[test]
    fn extract_content_finds_content_class() {
        let html = r#"<aside>Side</aside><div class="content"><p>Main</p></div>"#;
        let content = extract_content(html, None);
        assert!(content.contains("Main"));
    }

    #[test]
    fn extract_headings_strips_nested_tags() {
        let html = "<h1><span class='icon'>*</span> Title</h1>";
        let headings = extract_headings(html);
        assert!(headings.contains("# * Title") || headings.contains("# Title"));
    }

    #[test]
    fn extract_headings_empty() {
        let html = "<p>No headings</p>";
        let headings = extract_headings(html);
        assert!(headings.is_empty());
    }

    #[test]
    fn extract_summary_includes_headings() {
        let html = "<h1>Title</h1><p>Para 1</p><h2>Section</h2><p>Para 2</p>";
        let summary = extract_summary(html);
        assert!(summary.contains("Title"));
        assert!(summary.contains("Section"));
    }

    #[test]
    fn extract_summary_skips_empty_lines() {
        let html = "<p>Para 1</p><p></p><p>Para 2</p>";
        let summary = extract_summary(html);
        assert!(summary.contains("Para 1"));
        assert!(summary.contains("Para 2"));
    }

    #[test]
    fn extract_summary_handles_lists() {
        let html = "<p>Intro</p><ul><li>Item 1</li><li>Item 2</li></ul><p>Para 2</p>";
        let summary = extract_summary(html);
        // List items shouldn't count toward para limit
        assert!(summary.contains("Intro"));
        assert!(summary.contains("Item 1"));
    }

    #[test]
    fn html_to_markdown_h1_to_h6() {
        let html = "<h1>H1</h1><h2>H2</h2><h3>H3</h3><h4>H4</h4><h5>H5</h5><h6>H6</h6>";
        let md = html_to_markdown(html);
        assert!(md.contains("# H1"));
        assert!(md.contains("## H2"));
        assert!(md.contains("### H3"));
        assert!(md.contains("#### H4"));
        assert!(md.contains("##### H5"));
        assert!(md.contains("###### H6"));
    }

    #[test]
    fn html_to_markdown_cleans_whitespace() {
        let html = "<p>Text</p>\n\n\n\n<p>More</p>";
        let md = html_to_markdown(html);
        // Should not have excessive newlines
        assert!(!md.contains("\n\n\n"));
    }

    #[test]
    fn html_to_markdown_nbsp_entity() {
        let html = "word&nbsp;word";
        let md = html_to_markdown(html);
        assert!(md.contains("word word"));
    }

    #[test]
    fn html_to_markdown_quot_entity() {
        let html = "&quot;quoted&quot;";
        let md = html_to_markdown(html);
        assert!(md.contains("\"quoted\""));
    }

    #[test]
    fn html_to_markdown_apos_entity() {
        let html = "it&#39;s";
        let md = html_to_markdown(html);
        assert!(md.contains("it's"));
    }
}
