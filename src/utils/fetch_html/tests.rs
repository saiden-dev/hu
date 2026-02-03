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
