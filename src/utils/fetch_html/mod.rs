use anyhow::{Context, Result};
use regex::Regex;
use std::fs;

use super::cli::FetchHtmlArgs;

#[cfg(test)]
mod tests;

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
