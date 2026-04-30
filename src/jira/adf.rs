//! Atlassian Document Format (ADF) helpers.
//!
//! Two pure-functional entry points used by the rest of the Jira module:
//!
//! - [`markdown_to_adf`] converts a Markdown string into an ADF v1
//!   `{type:"doc", version:1, content:[...]}` value. Used when sending
//!   descriptions or comments to Jira.
//! - [`adf_to_plain_text`] flattens an ADF tree into a plain-text string.
//!   Used to render Jira-side rich content (descriptions, comments) in
//!   the terminal.
//!
//! ADF schema reference: <https://developer.atlassian.com/cloud/jira/platform/apidocs/>
//!
//! Tables, panels, mentions, and emoji are deliberately not supported in
//! the writer — they require Jira-side context not available from a CLI.
//! The reader will pass through their text content verbatim.
//!
//! [`markdown_to_adf`]: fn.markdown_to_adf.html

// The writer side of this module is exercised by inline tests but isn't
// wired into a caller until chunk 2.B (update_issue) and 2.C (--body-adf).
// The reader (`adf_to_plain_text`) lands its first caller in chunk 2.B too.
#![allow(dead_code)]

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use serde_json::{json, Value};

/// ADF schema version we emit. Atlassian has only ever shipped v1 in
/// public APIs as of 2026-04 — pinned for predictability.
const ADF_VERSION: u8 = 1;

/// Convert a Markdown string into an ADF v1 document.
///
/// Supported Markdown constructs:
/// - Headings 1–6 → `heading`
/// - Paragraphs → `paragraph`
/// - Bullet/ordered lists with nesting → `bulletList` / `orderedList`
/// - Code blocks (fenced or indented) → `codeBlock` (preserves language)
/// - Inline code → `text` with `code` mark
/// - Bold/italic/strikethrough → `text` with `strong`/`em`/`strike` marks
/// - Links → `text` with `link` mark
/// - Block quotes → `blockquote`
/// - Horizontal rules → `rule`
/// - Hard breaks → `hardBreak`
///
/// Tables are not currently emitted (CLAUDE.md scope cut for v0.2.0).
pub fn markdown_to_adf(md: &str) -> Value {
    let parser = Parser::new_ext(md, Options::ENABLE_STRIKETHROUGH);
    let mut builder = Builder::default();
    for event in parser {
        builder.handle(event);
    }
    json!({
        "type": "doc",
        "version": ADF_VERSION,
        "content": builder.finish(),
    })
}

/// Render an ADF tree (whole document or any sub-node) as plain text.
///
/// Concatenates every `text` node it finds during a depth-first walk.
/// Block-level separation is preserved as newlines between top-level
/// paragraphs, headings, and list items.
pub fn adf_to_plain_text(node: &Value) -> String {
    if let Some(content) = node["content"].as_array() {
        let parts: Vec<String> = content.iter().map(render_block).collect();
        return parts
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
    }
    render_block(node)
}

/// Recursive plain-text renderer for a single ADF node.
fn render_block(node: &Value) -> String {
    if let Some(text) = node["text"].as_str() {
        return text.to_string();
    }
    let Some(content) = node["content"].as_array() else {
        return String::new();
    };
    let separator = match node["type"].as_str().unwrap_or("") {
        "doc" | "bulletList" | "orderedList" => "\n",
        _ => "",
    };
    content
        .iter()
        .map(render_block)
        .collect::<Vec<_>>()
        .join(separator)
}

// ---------------------------------------------------------------------------
// Markdown -> ADF builder
// ---------------------------------------------------------------------------

/// Inline mark types supported by the writer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mark {
    Strong,
    Em,
    Code,
    Strike,
    Link,
}

impl Mark {
    fn as_str(self) -> &'static str {
        match self {
            Self::Strong => "strong",
            Self::Em => "em",
            Self::Code => "code",
            Self::Strike => "strike",
            Self::Link => "link",
        }
    }
}

/// Block context being filled. Each variant owns its in-progress ADF children.
#[derive(Debug)]
enum BlockCtx {
    Heading {
        level: u8,
        content: Vec<Value>,
    },
    Paragraph {
        content: Vec<Value>,
    },
    Blockquote {
        content: Vec<Value>,
    },
    BulletList {
        items: Vec<Value>,
    },
    OrderedList {
        start: u32,
        items: Vec<Value>,
    },
    ListItem {
        content: Vec<Value>,
    },
    CodeBlock {
        language: Option<String>,
        text: String,
    },
}

#[derive(Default)]
struct Builder {
    /// Stack of currently-open block contexts. The top of the stack is
    /// where new inline content lands.
    stack: Vec<BlockCtx>,
    /// Inline marks active for the next emitted text node, in push order.
    marks: Vec<Mark>,
    /// Active link href (set when a `Link` mark is on top of `marks`).
    link_href: Option<String>,
    /// Top-level document content emitted so far.
    blocks: Vec<Value>,
}

impl Builder {
    fn handle(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.start_tag(tag),
            Event::End(tag) => self.end_tag(tag),
            Event::Text(s) => self.push_text(&s),
            Event::Code(s) => {
                self.marks.push(Mark::Code);
                self.push_text(&s);
                self.marks.pop();
            }
            Event::SoftBreak => self.push_text(" "),
            Event::HardBreak => self.push_inline(json!({"type": "hardBreak"})),
            Event::Rule => self.push_block(json!({"type": "rule"})),
            // HTML, footnotes, math, tasklist markers — not represented in
            // ADF v1 from the CLI side. Emit text fallback for HTML so
            // content isn't silently dropped.
            Event::Html(s) | Event::InlineHtml(s) => self.push_text(&s),
            Event::FootnoteReference(_)
            | Event::TaskListMarker(_)
            | Event::InlineMath(_)
            | Event::DisplayMath(_) => {}
        }
    }

    fn start_tag(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph => self.stack.push(BlockCtx::Paragraph {
                content: Vec::new(),
            }),
            Tag::Heading { level, .. } => self.stack.push(BlockCtx::Heading {
                level: heading_level(level),
                content: Vec::new(),
            }),
            Tag::BlockQuote(_) => self.stack.push(BlockCtx::Blockquote {
                content: Vec::new(),
            }),
            Tag::CodeBlock(kind) => self.stack.push(BlockCtx::CodeBlock {
                language: code_language(&kind),
                text: String::new(),
            }),
            Tag::List(None) => self.stack.push(BlockCtx::BulletList { items: Vec::new() }),
            Tag::List(Some(start)) => self.stack.push(BlockCtx::OrderedList {
                start: u32::try_from(start).unwrap_or(1),
                items: Vec::new(),
            }),
            Tag::Item => self.stack.push(BlockCtx::ListItem {
                content: Vec::new(),
            }),
            Tag::Strong => self.marks.push(Mark::Strong),
            Tag::Emphasis => self.marks.push(Mark::Em),
            Tag::Strikethrough => self.marks.push(Mark::Strike),
            Tag::Link { dest_url, .. } => {
                self.marks.push(Mark::Link);
                self.link_href = Some(dest_url.into_string());
            }
            // Images, tables, footnote defs, MetadataBlock, etc.: dropped
            // (no ADF mapping in our supported subset).
            _ => {}
        }
    }

    fn end_tag(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph
            | TagEnd::Heading(_)
            | TagEnd::BlockQuote(_)
            | TagEnd::CodeBlock
            | TagEnd::List(_)
            | TagEnd::Item => {
                if let Some(block) = self.stack.pop() {
                    let value = block_to_value(block);
                    self.commit_block(value);
                }
            }
            TagEnd::Strong => {
                pop_mark(&mut self.marks, Mark::Strong);
            }
            TagEnd::Emphasis => {
                pop_mark(&mut self.marks, Mark::Em);
            }
            TagEnd::Strikethrough => {
                pop_mark(&mut self.marks, Mark::Strike);
            }
            TagEnd::Link => {
                pop_mark(&mut self.marks, Mark::Link);
                self.link_href = None;
            }
            _ => {}
        }
    }

    /// Append text to the current context, applying active marks.
    fn push_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        // CodeBlock collects raw text directly.
        if let Some(BlockCtx::CodeBlock { text: buf, .. }) = self.stack.last_mut() {
            buf.push_str(text);
            return;
        }
        let node = build_text_node(text, &self.marks, self.link_href.as_deref());
        self.push_inline(node);
    }

    /// Append an inline node (text or hardBreak) to the current context.
    fn push_inline(&mut self, node: Value) {
        match self.stack.last_mut() {
            Some(BlockCtx::Paragraph { content })
            | Some(BlockCtx::Heading { content, .. })
            | Some(BlockCtx::ListItem { content })
            | Some(BlockCtx::Blockquote { content }) => content.push(node),
            // Inline content with no surrounding block — wrap in a
            // synthetic paragraph at the document level.
            None => self.blocks.push(json!({
                "type": "paragraph",
                "content": [node],
            })),
            _ => {}
        }
    }

    /// Commit a finished block to the nearest enclosing container.
    fn commit_block(&mut self, value: Value) {
        match self.stack.last_mut() {
            Some(BlockCtx::Blockquote { content }) | Some(BlockCtx::ListItem { content }) => {
                content.push(value);
            }
            Some(BlockCtx::BulletList { items }) | Some(BlockCtx::OrderedList { items, .. }) => {
                items.push(value);
            }
            _ => self.blocks.push(value),
        }
    }

    /// Push a leaf block (e.g., `rule`) at the current position.
    fn push_block(&mut self, value: Value) {
        self.commit_block(value);
    }

    fn finish(self) -> Vec<Value> {
        self.blocks
    }
}

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn code_language(kind: &CodeBlockKind<'_>) -> Option<String> {
    match kind {
        CodeBlockKind::Fenced(s) if !s.is_empty() => Some(s.to_string()),
        _ => None,
    }
}

/// Remove the topmost matching mark. Avoids stack desync on malformed input.
fn pop_mark(marks: &mut Vec<Mark>, mark: Mark) {
    if let Some(pos) = marks.iter().rposition(|m| *m == mark) {
        marks.remove(pos);
    }
}

/// Build an ADF `text` node, applying all active marks. Strips trailing
/// newlines that pulldown-cmark keeps on code-block text.
fn build_text_node(text: &str, marks: &[Mark], link_href: Option<&str>) -> Value {
    let mut node = json!({"type": "text", "text": text});
    if marks.is_empty() {
        return node;
    }
    let mark_values: Vec<Value> = marks
        .iter()
        .map(|m| match m {
            Mark::Link => json!({
                "type": "link",
                "attrs": { "href": link_href.unwrap_or("") },
            }),
            other => json!({ "type": other.as_str() }),
        })
        .collect();
    node["marks"] = Value::Array(mark_values);
    node
}

fn block_to_value(block: BlockCtx) -> Value {
    match block {
        BlockCtx::Paragraph { content } => json!({
            "type": "paragraph",
            "content": content,
        }),
        BlockCtx::Heading { level, content } => json!({
            "type": "heading",
            "attrs": { "level": level },
            "content": content,
        }),
        BlockCtx::Blockquote { content } => json!({
            "type": "blockquote",
            "content": content,
        }),
        BlockCtx::BulletList { items } => json!({
            "type": "bulletList",
            "content": items,
        }),
        BlockCtx::OrderedList { start, items } => json!({
            "type": "orderedList",
            "attrs": { "order": start },
            "content": items,
        }),
        BlockCtx::ListItem { content } => {
            // ADF requires listItem children to be block-level. Wrap loose
            // inline content (the common Markdown case `- text`) in a
            // paragraph.
            let normalized = wrap_loose_inlines(content);
            json!({
                "type": "listItem",
                "content": normalized,
            })
        }
        BlockCtx::CodeBlock { language, text } => {
            let stripped = text.trim_end_matches('\n').to_string();
            let mut node = json!({
                "type": "codeBlock",
                "content": [{ "type": "text", "text": stripped }],
            });
            if let Some(lang) = language {
                node["attrs"] = json!({ "language": lang });
            }
            node
        }
    }
}

/// Wrap any inline-level nodes in `content` into a synthetic paragraph,
/// so the result satisfies ADF block-content rules. Block-level entries
/// (paragraph, list, etc.) are passed through unchanged.
fn wrap_loose_inlines(content: Vec<Value>) -> Vec<Value> {
    let mut output = Vec::new();
    let mut buffer: Vec<Value> = Vec::new();
    for node in content {
        if is_block(&node) {
            if !buffer.is_empty() {
                output.push(json!({
                    "type": "paragraph",
                    "content": std::mem::take(&mut buffer),
                }));
            }
            output.push(node);
        } else {
            buffer.push(node);
        }
    }
    if !buffer.is_empty() {
        output.push(json!({"type": "paragraph", "content": buffer}));
    }
    output
}

fn is_block(node: &Value) -> bool {
    matches!(
        node["type"].as_str(),
        Some(
            "paragraph"
                | "heading"
                | "blockquote"
                | "bulletList"
                | "orderedList"
                | "codeBlock"
                | "rule"
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(blocks: Vec<Value>) -> Value {
        json!({"type": "doc", "version": 1, "content": blocks})
    }

    #[test]
    fn empty_input_produces_empty_doc() {
        assert_eq!(markdown_to_adf(""), doc(vec![]));
    }

    #[test]
    fn plain_paragraph() {
        let adf = markdown_to_adf("Hello world");
        assert_eq!(
            adf,
            doc(vec![json!({
                "type": "paragraph",
                "content": [{"type": "text", "text": "Hello world"}],
            })])
        );
    }

    #[test]
    fn heading_levels() {
        for (md, level) in [
            ("# h1", 1),
            ("## h2", 2),
            ("### h3", 3),
            ("#### h4", 4),
            ("##### h5", 5),
            ("###### h6", 6),
        ] {
            let adf = markdown_to_adf(md);
            assert_eq!(adf["content"][0]["type"], "heading");
            assert_eq!(adf["content"][0]["attrs"]["level"], level);
        }
    }

    #[test]
    fn bold_italic_strike_inline() {
        let adf = markdown_to_adf("**bold** *em* ~~gone~~");
        let para = &adf["content"][0]["content"];
        assert_eq!(para[0]["text"], "bold");
        assert_eq!(para[0]["marks"][0]["type"], "strong");
        assert_eq!(para[2]["marks"][0]["type"], "em");
        assert_eq!(para[4]["marks"][0]["type"], "strike");
    }

    #[test]
    fn nested_marks_stack_on_text_node() {
        let adf = markdown_to_adf("**bold *and* em**");
        // Output sequence: "bold " (strong), "and" (strong+em), " em" (strong)
        let para = &adf["content"][0]["content"];
        let strong_marks: Vec<&str> = para[0]["marks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["type"].as_str().unwrap())
            .collect();
        assert!(strong_marks.contains(&"strong"));

        let mid_marks: Vec<&str> = para[1]["marks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["type"].as_str().unwrap())
            .collect();
        assert!(mid_marks.contains(&"strong"));
        assert!(mid_marks.contains(&"em"));
    }

    #[test]
    fn inline_code_gets_code_mark() {
        let adf = markdown_to_adf("use `cargo build`");
        let para = &adf["content"][0]["content"];
        assert_eq!(para[1]["text"], "cargo build");
        assert_eq!(para[1]["marks"][0]["type"], "code");
    }

    #[test]
    fn link_emits_link_mark_with_href() {
        let adf = markdown_to_adf("[home](https://example.com)");
        let text = &adf["content"][0]["content"][0];
        assert_eq!(text["text"], "home");
        assert_eq!(text["marks"][0]["type"], "link");
        assert_eq!(text["marks"][0]["attrs"]["href"], "https://example.com");
    }

    #[test]
    fn bullet_list_with_paragraph_items() {
        let adf = markdown_to_adf("- one\n- two");
        let list = &adf["content"][0];
        assert_eq!(list["type"], "bulletList");
        assert_eq!(list["content"][0]["type"], "listItem");
        // listItem -> paragraph -> text
        assert_eq!(
            list["content"][0]["content"][0]["content"][0]["text"],
            "one"
        );
        assert_eq!(
            list["content"][1]["content"][0]["content"][0]["text"],
            "two"
        );
    }

    #[test]
    fn ordered_list_preserves_start() {
        let adf = markdown_to_adf("3. first\n4. second");
        let list = &adf["content"][0];
        assert_eq!(list["type"], "orderedList");
        assert_eq!(list["attrs"]["order"], 3);
        assert_eq!(list["content"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn nested_list_produces_nested_lists() {
        let adf = markdown_to_adf("- a\n  - nested\n- b");
        let outer = &adf["content"][0];
        assert_eq!(outer["type"], "bulletList");
        let first_item = &outer["content"][0];
        // First item should contain a paragraph plus a nested bulletList.
        let item_content = first_item["content"].as_array().unwrap();
        assert!(item_content.iter().any(|n| n["type"] == "paragraph"));
        assert!(item_content.iter().any(|n| n["type"] == "bulletList"));
    }

    #[test]
    fn code_block_with_language() {
        let adf = markdown_to_adf("```rust\nfn main() {}\n```");
        let cb = &adf["content"][0];
        assert_eq!(cb["type"], "codeBlock");
        assert_eq!(cb["attrs"]["language"], "rust");
        assert_eq!(cb["content"][0]["text"], "fn main() {}");
    }

    #[test]
    fn code_block_without_language() {
        let adf = markdown_to_adf("    indented\n    code");
        let cb = &adf["content"][0];
        assert_eq!(cb["type"], "codeBlock");
        assert!(cb["attrs"].is_null() || cb["attrs"]["language"].is_null());
    }

    #[test]
    fn blockquote_wraps_paragraph() {
        let adf = markdown_to_adf("> quoted");
        let bq = &adf["content"][0];
        assert_eq!(bq["type"], "blockquote");
        assert_eq!(bq["content"][0]["type"], "paragraph");
        assert_eq!(bq["content"][0]["content"][0]["text"], "quoted");
    }

    #[test]
    fn horizontal_rule() {
        let adf = markdown_to_adf("---");
        assert_eq!(adf["content"][0]["type"], "rule");
    }

    #[test]
    fn hard_break_emits_hardbreak_node() {
        let adf = markdown_to_adf("line1  \nline2");
        let para = &adf["content"][0]["content"];
        assert!(para
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["type"] == "hardBreak"));
    }

    #[test]
    fn soft_break_becomes_space() {
        let adf = markdown_to_adf("line1\nline2");
        // Soft break inside a single paragraph collapses to one space.
        let para = &adf["content"][0]["content"];
        let joined: String = para
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|n| n["text"].as_str())
            .collect();
        assert!(joined.contains("line1 line2"));
    }

    #[test]
    fn adf_to_plain_text_renders_paragraph() {
        let node = json!({
            "type": "doc",
            "content": [{
                "type": "paragraph",
                "content": [
                    {"type": "text", "text": "Hello "},
                    {"type": "text", "text": "world"},
                ],
            }],
        });
        assert_eq!(adf_to_plain_text(&node), "Hello world");
    }

    #[test]
    fn adf_to_plain_text_joins_blocks_with_newline() {
        let node = json!({
            "type": "doc",
            "content": [
                {"type": "paragraph", "content": [{"type": "text", "text": "one"}]},
                {"type": "paragraph", "content": [{"type": "text", "text": "two"}]},
            ],
        });
        assert_eq!(adf_to_plain_text(&node), "one\ntwo");
    }

    #[test]
    fn adf_to_plain_text_handles_text_node_directly() {
        let node = json!({"type": "text", "text": "naked"});
        assert_eq!(adf_to_plain_text(&node), "naked");
    }

    #[test]
    fn adf_to_plain_text_returns_empty_for_unknown_shape() {
        assert_eq!(adf_to_plain_text(&json!({"type": "unknown"})), "");
        assert_eq!(adf_to_plain_text(&Value::Null), "");
    }

    #[test]
    fn roundtrip_through_plain_text_preserves_visible_content() {
        let adf = markdown_to_adf("# Heading\n\nA paragraph with **bold**.");
        let text = adf_to_plain_text(&adf);
        assert!(text.contains("Heading"));
        assert!(text.contains("A paragraph with bold."));
    }

    #[test]
    fn html_passes_through_as_text() {
        let adf = markdown_to_adf("<custom>tag</custom>");
        // No silent drop; raw HTML appears as text content.
        let text = adf_to_plain_text(&adf);
        assert!(text.contains("custom"));
    }
}
