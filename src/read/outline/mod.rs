use regex::Regex;
use std::path::Path;

use super::types::{FileOutline, ItemKind, OutlineItem};

#[cfg(test)]
mod tests;

/// Extract outline from file content based on extension
pub fn extract_outline(content: &str, path: &str) -> FileOutline {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let mut outline = FileOutline::new();

    match ext {
        "rs" => extract_rust_outline(content, &mut outline),
        "py" => extract_python_outline(content, &mut outline),
        "js" | "ts" | "jsx" | "tsx" | "mjs" => extract_js_outline(content, &mut outline),
        "rb" => extract_ruby_outline(content, &mut outline),
        "go" => extract_go_outline(content, &mut outline),
        "md" | "markdown" => extract_markdown_outline(content, &mut outline),
        _ => {}
    }

    outline
}

/// Extract Rust outline (functions, structs, enums, traits, impls)
fn extract_rust_outline(content: &str, outline: &mut FileOutline) {
    let fn_re = Regex::new(
        r"^(\s*)(pub\s+)?(async\s+)?fn\s+(\w+)\s*(<[^>]+>)?\s*\([^)]*\)(\s*->\s*[^{]+)?",
    )
    .unwrap();
    let struct_re = Regex::new(r"^(\s*)(pub\s+)?struct\s+(\w+)(<[^>]+>)?").unwrap();
    let enum_re = Regex::new(r"^(\s*)(pub\s+)?enum\s+(\w+)(<[^>]+>)?").unwrap();
    let trait_re = Regex::new(r"^(\s*)(pub\s+)?trait\s+(\w+)(<[^>]+>)?").unwrap();
    let impl_re = Regex::new(r"^(\s*)impl\s*(<[^>]+>)?\s*(\w+)(<[^>]+>)?(\s+for\s+\w+)?").unwrap();
    let mod_re = Regex::new(r"^(\s*)(pub\s+)?mod\s+(\w+)").unwrap();
    let const_re = Regex::new(r"^(\s*)(pub\s+)?const\s+(\w+)").unwrap();
    let type_re = Regex::new(r"^(\s*)(pub\s+)?type\s+(\w+)").unwrap();

    for (line_num, line) in content.lines().enumerate() {
        let line_num = line_num + 1;

        if let Some(caps) = fn_re.captures(line) {
            let indent = caps.get(1).map_or(0, |m| m.as_str().len());
            let sig = caps.get(0).unwrap().as_str().trim_end_matches('{').trim();
            outline.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                indent / 4,
                ItemKind::Function,
            ));
        } else if let Some(caps) = struct_re.captures(line) {
            let indent = caps.get(1).map_or(0, |m| m.as_str().len());
            let sig = caps.get(0).unwrap().as_str().trim();
            outline.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                indent / 4,
                ItemKind::Struct,
            ));
        } else if let Some(caps) = enum_re.captures(line) {
            let indent = caps.get(1).map_or(0, |m| m.as_str().len());
            let sig = caps.get(0).unwrap().as_str().trim();
            outline.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                indent / 4,
                ItemKind::Enum,
            ));
        } else if let Some(caps) = trait_re.captures(line) {
            let indent = caps.get(1).map_or(0, |m| m.as_str().len());
            let sig = caps.get(0).unwrap().as_str().trim();
            outline.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                indent / 4,
                ItemKind::Trait,
            ));
        } else if let Some(caps) = impl_re.captures(line) {
            let indent = caps.get(1).map_or(0, |m| m.as_str().len());
            let sig = caps.get(0).unwrap().as_str().trim();
            outline.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                indent / 4,
                ItemKind::Impl,
            ));
        } else if let Some(caps) = mod_re.captures(line) {
            let indent = caps.get(1).map_or(0, |m| m.as_str().len());
            let sig = caps.get(0).unwrap().as_str().trim();
            outline.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                indent / 4,
                ItemKind::Module,
            ));
        } else if let Some(caps) = const_re.captures(line) {
            let indent = caps.get(1).map_or(0, |m| m.as_str().len());
            let sig = caps.get(0).unwrap().as_str().trim();
            outline.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                indent / 4,
                ItemKind::Const,
            ));
        } else if let Some(caps) = type_re.captures(line) {
            let indent = caps.get(1).map_or(0, |m| m.as_str().len());
            let sig = caps.get(0).unwrap().as_str().trim();
            outline.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                indent / 4,
                ItemKind::Type,
            ));
        }
    }
}

/// Extract Python outline (functions, classes)
fn extract_python_outline(content: &str, outline: &mut FileOutline) {
    let def_re = Regex::new(r"^(\s*)(async\s+)?def\s+(\w+)\s*\([^)]*\)(\s*->\s*[^:]+)?").unwrap();
    let class_re = Regex::new(r"^(\s*)class\s+(\w+)(\([^)]*\))?").unwrap();

    for (line_num, line) in content.lines().enumerate() {
        let line_num = line_num + 1;

        if let Some(caps) = def_re.captures(line) {
            let indent = caps.get(1).map_or(0, |m| m.as_str().len());
            let sig = caps.get(0).unwrap().as_str().trim_end_matches(':').trim();
            outline.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                indent / 4,
                ItemKind::Function,
            ));
        } else if let Some(caps) = class_re.captures(line) {
            let indent = caps.get(1).map_or(0, |m| m.as_str().len());
            let sig = caps.get(0).unwrap().as_str().trim_end_matches(':').trim();
            outline.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                indent / 4,
                ItemKind::Class,
            ));
        }
    }
}

/// Extract JavaScript/TypeScript outline
fn extract_js_outline(content: &str, outline: &mut FileOutline) {
    let fn_re =
        Regex::new(r"^(\s*)(export\s+)?(async\s+)?function\s+(\w+)\s*(<[^>]+>)?\s*\([^)]*\)")
            .unwrap();
    let arrow_re =
        Regex::new(r"^(\s*)(export\s+)?(const|let|var)\s+(\w+)\s*=\s*(async\s+)?\([^)]*\)\s*=>")
            .unwrap();
    let class_re = Regex::new(r"^(\s*)(export\s+)?class\s+(\w+)(\s+extends\s+\w+)?").unwrap();
    let method_re = Regex::new(r"^(\s*)(async\s+)?(\w+)\s*\([^)]*\)\s*\{").unwrap();

    for (line_num, line) in content.lines().enumerate() {
        let line_num = line_num + 1;

        if let Some(caps) = fn_re.captures(line) {
            let indent = caps.get(1).map_or(0, |m| m.as_str().len());
            let sig = caps.get(0).unwrap().as_str().trim();
            outline.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                indent / 2,
                ItemKind::Function,
            ));
        } else if let Some(caps) = arrow_re.captures(line) {
            let indent = caps.get(1).map_or(0, |m| m.as_str().len());
            let sig = caps.get(0).unwrap().as_str().trim_end_matches("=>").trim();
            outline.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                indent / 2,
                ItemKind::Function,
            ));
        } else if let Some(caps) = class_re.captures(line) {
            let indent = caps.get(1).map_or(0, |m| m.as_str().len());
            let sig = caps.get(0).unwrap().as_str().trim();
            outline.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                indent / 2,
                ItemKind::Class,
            ));
        } else if let Some(caps) = method_re.captures(line) {
            let indent = caps.get(1).map_or(0, |m| m.as_str().len());
            // Only include methods with some indent (inside class)
            if indent > 0 {
                let sig = caps.get(0).unwrap().as_str().trim_end_matches('{').trim();
                outline.push(OutlineItem::new(
                    line_num,
                    sig.to_string(),
                    indent / 2,
                    ItemKind::Function,
                ));
            }
        }
    }
}

/// Extract Ruby outline
fn extract_ruby_outline(content: &str, outline: &mut FileOutline) {
    let def_re = Regex::new(r"^(\s*)def\s+(\w+[?!=]?)(\([^)]*\))?").unwrap();
    let class_re = Regex::new(r"^(\s*)class\s+(\w+)(\s*<\s*\w+)?").unwrap();
    let module_re = Regex::new(r"^(\s*)module\s+(\w+)").unwrap();

    for (line_num, line) in content.lines().enumerate() {
        let line_num = line_num + 1;

        if let Some(caps) = def_re.captures(line) {
            let indent = caps.get(1).map_or(0, |m| m.as_str().len());
            let sig = caps.get(0).unwrap().as_str().trim();
            outline.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                indent / 2,
                ItemKind::Function,
            ));
        } else if let Some(caps) = class_re.captures(line) {
            let indent = caps.get(1).map_or(0, |m| m.as_str().len());
            let sig = caps.get(0).unwrap().as_str().trim();
            outline.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                indent / 2,
                ItemKind::Class,
            ));
        } else if let Some(caps) = module_re.captures(line) {
            let indent = caps.get(1).map_or(0, |m| m.as_str().len());
            let sig = caps.get(0).unwrap().as_str().trim();
            outline.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                indent / 2,
                ItemKind::Module,
            ));
        }
    }
}

/// Extract Go outline
fn extract_go_outline(content: &str, outline: &mut FileOutline) {
    let func_re =
        Regex::new(r"^func\s+(\([^)]+\)\s+)?(\w+)\s*\([^)]*\)(\s*\([^)]*\)|\s*\w+)?").unwrap();
    let type_struct_re = Regex::new(r"^type\s+(\w+)\s+struct").unwrap();
    let type_interface_re = Regex::new(r"^type\s+(\w+)\s+interface").unwrap();

    for (line_num, line) in content.lines().enumerate() {
        let line_num = line_num + 1;

        if let Some(caps) = func_re.captures(line) {
            let sig = caps.get(0).unwrap().as_str().trim();
            outline.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                0,
                ItemKind::Function,
            ));
        } else if let Some(caps) = type_struct_re.captures(line) {
            let sig = caps.get(0).unwrap().as_str().trim();
            outline.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                0,
                ItemKind::Struct,
            ));
        } else if let Some(caps) = type_interface_re.captures(line) {
            let sig = caps.get(0).unwrap().as_str().trim();
            outline.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                0,
                ItemKind::Trait,
            ));
        }
    }
}

/// Extract Markdown outline (headings)
fn extract_markdown_outline(content: &str, outline: &mut FileOutline) {
    let heading_re = Regex::new(r"^(#{1,6})\s+(.+)$").unwrap();

    for (line_num, line) in content.lines().enumerate() {
        let line_num = line_num + 1;

        if let Some(caps) = heading_re.captures(line) {
            let level = caps.get(1).unwrap().as_str().len() as u8;
            let text = caps.get(2).unwrap().as_str().to_string();
            outline.push(OutlineItem::new(
                line_num,
                text,
                (level - 1) as usize,
                ItemKind::Heading(level),
            ));
        }
    }
}
