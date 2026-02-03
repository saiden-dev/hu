use regex::Regex;
use std::path::Path;

use super::types::{ItemKind, OutlineItem};

#[cfg(test)]
mod tests;

/// Extract public interface from file content
pub fn extract_interface(content: &str, path: &str) -> Vec<OutlineItem> {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    match ext {
        "rs" => extract_rust_interface(content),
        "py" => extract_python_interface(content),
        "js" | "ts" | "jsx" | "tsx" | "mjs" => extract_js_interface(content),
        "rb" => extract_ruby_interface(content),
        "go" => extract_go_interface(content),
        _ => vec![],
    }
}

/// Extract Rust public interface (pub items only)
fn extract_rust_interface(content: &str) -> Vec<OutlineItem> {
    let mut items = Vec::new();

    let pub_fn_re =
        Regex::new(r"^(\s*)pub\s+(async\s+)?fn\s+(\w+)\s*(<[^>]+>)?\s*\([^)]*\)(\s*->\s*[^{]+)?")
            .unwrap();
    let pub_struct_re = Regex::new(r"^(\s*)pub\s+struct\s+(\w+)(<[^>]+>)?").unwrap();
    let pub_enum_re = Regex::new(r"^(\s*)pub\s+enum\s+(\w+)(<[^>]+>)?").unwrap();
    let pub_trait_re = Regex::new(r"^(\s*)pub\s+trait\s+(\w+)(<[^>]+>)?").unwrap();
    let pub_const_re = Regex::new(r"^(\s*)pub\s+const\s+(\w+)").unwrap();
    let pub_type_re = Regex::new(r"^(\s*)pub\s+type\s+(\w+)").unwrap();
    let pub_mod_re = Regex::new(r"^(\s*)pub\s+mod\s+(\w+)").unwrap();

    for (line_num, line) in content.lines().enumerate() {
        let line_num = line_num + 1;

        if let Some(caps) = pub_fn_re.captures(line) {
            let indent = caps.get(1).map_or(0, |m| m.as_str().len());
            let sig = caps.get(0).unwrap().as_str().trim_end_matches('{').trim();
            items.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                indent / 4,
                ItemKind::Function,
            ));
        } else if let Some(caps) = pub_struct_re.captures(line) {
            let indent = caps.get(1).map_or(0, |m| m.as_str().len());
            let sig = caps.get(0).unwrap().as_str().trim();
            items.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                indent / 4,
                ItemKind::Struct,
            ));
        } else if let Some(caps) = pub_enum_re.captures(line) {
            let indent = caps.get(1).map_or(0, |m| m.as_str().len());
            let sig = caps.get(0).unwrap().as_str().trim();
            items.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                indent / 4,
                ItemKind::Enum,
            ));
        } else if let Some(caps) = pub_trait_re.captures(line) {
            let indent = caps.get(1).map_or(0, |m| m.as_str().len());
            let sig = caps.get(0).unwrap().as_str().trim();
            items.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                indent / 4,
                ItemKind::Trait,
            ));
        } else if let Some(caps) = pub_const_re.captures(line) {
            let indent = caps.get(1).map_or(0, |m| m.as_str().len());
            let sig = caps.get(0).unwrap().as_str().trim();
            items.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                indent / 4,
                ItemKind::Const,
            ));
        } else if let Some(caps) = pub_type_re.captures(line) {
            let indent = caps.get(1).map_or(0, |m| m.as_str().len());
            let sig = caps.get(0).unwrap().as_str().trim();
            items.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                indent / 4,
                ItemKind::Type,
            ));
        } else if let Some(caps) = pub_mod_re.captures(line) {
            let indent = caps.get(1).map_or(0, |m| m.as_str().len());
            let sig = caps.get(0).unwrap().as_str().trim();
            items.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                indent / 4,
                ItemKind::Module,
            ));
        }
    }

    items
}

/// Extract Python public interface (exclude _private items)
fn extract_python_interface(content: &str) -> Vec<OutlineItem> {
    let mut items = Vec::new();

    let def_re = Regex::new(r"^(\s*)(async\s+)?def\s+(\w+)\s*\([^)]*\)(\s*->\s*[^:]+)?").unwrap();
    let class_re = Regex::new(r"^(\s*)class\s+(\w+)(\([^)]*\))?").unwrap();

    for (line_num, line) in content.lines().enumerate() {
        let line_num = line_num + 1;

        if let Some(caps) = def_re.captures(line) {
            let indent = caps.get(1).map_or(0, |m| m.as_str().len());
            let name = caps.get(3).map_or("", |m| m.as_str());

            // Skip private functions (leading underscore) at top level
            if indent == 0 && name.starts_with('_') && !name.starts_with("__") {
                continue;
            }

            // Skip methods (indented)
            if indent > 0 {
                continue;
            }

            let sig = caps.get(0).unwrap().as_str().trim_end_matches(':').trim();
            items.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                0,
                ItemKind::Function,
            ));
        } else if let Some(caps) = class_re.captures(line) {
            let indent = caps.get(1).map_or(0, |m| m.as_str().len());
            let name = caps.get(2).map_or("", |m| m.as_str());

            // Skip private classes
            if name.starts_with('_') && !name.starts_with("__") {
                continue;
            }

            // Skip nested classes
            if indent > 0 {
                continue;
            }

            let sig = caps.get(0).unwrap().as_str().trim_end_matches(':').trim();
            items.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                0,
                ItemKind::Class,
            ));
        }
    }

    items
}

/// Extract JavaScript/TypeScript public interface (exports only)
fn extract_js_interface(content: &str) -> Vec<OutlineItem> {
    let mut items = Vec::new();

    let export_fn_re =
        Regex::new(r"^(\s*)export\s+(async\s+)?function\s+(\w+)\s*(<[^>]+>)?\s*\([^)]*\)").unwrap();
    let export_const_re =
        Regex::new(r"^(\s*)export\s+(const|let|var)\s+(\w+)\s*=\s*(async\s+)?\([^)]*\)\s*=>")
            .unwrap();
    let export_class_re = Regex::new(r"^(\s*)export\s+class\s+(\w+)(\s+extends\s+\w+)?").unwrap();
    let export_default_re =
        Regex::new(r"^(\s*)export\s+default\s+(class|function)?\s*(\w+)?").unwrap();

    for (line_num, line) in content.lines().enumerate() {
        let line_num = line_num + 1;

        if let Some(caps) = export_fn_re.captures(line) {
            let sig = caps.get(0).unwrap().as_str().trim();
            items.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                0,
                ItemKind::Function,
            ));
        } else if let Some(caps) = export_const_re.captures(line) {
            let sig = caps.get(0).unwrap().as_str().trim_end_matches("=>").trim();
            items.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                0,
                ItemKind::Function,
            ));
        } else if let Some(caps) = export_class_re.captures(line) {
            let sig = caps.get(0).unwrap().as_str().trim();
            items.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                0,
                ItemKind::Class,
            ));
        } else if let Some(caps) = export_default_re.captures(line) {
            let sig = caps.get(0).unwrap().as_str().trim();
            items.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                0,
                ItemKind::Other,
            ));
        }
    }

    items
}

/// Extract Ruby public interface (exclude private methods)
fn extract_ruby_interface(content: &str) -> Vec<OutlineItem> {
    let mut items = Vec::new();
    let mut in_private = false;

    let def_re = Regex::new(r"^(\s*)def\s+(\w+[?!=]?)(\([^)]*\))?").unwrap();
    let class_re = Regex::new(r"^(\s*)class\s+(\w+)(\s*<\s*\w+)?").unwrap();
    let module_re = Regex::new(r"^(\s*)module\s+(\w+)").unwrap();
    let private_re = Regex::new(r"^\s*private\s*$").unwrap();
    let public_re = Regex::new(r"^\s*public\s*$").unwrap();

    for (line_num, line) in content.lines().enumerate() {
        let line_num = line_num + 1;

        // Track private/public sections
        if private_re.is_match(line) {
            in_private = true;
            continue;
        }
        if public_re.is_match(line) {
            in_private = false;
            continue;
        }

        // Reset private flag on new class/module
        if class_re.is_match(line) || module_re.is_match(line) {
            in_private = false;
        }

        if let Some(caps) = def_re.captures(line) {
            let indent = caps.get(1).map_or(0, |m| m.as_str().len());

            // Skip private methods
            if in_private {
                continue;
            }

            // Skip nested methods
            if indent > 2 {
                continue;
            }

            let sig = caps.get(0).unwrap().as_str().trim();
            items.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                indent / 2,
                ItemKind::Function,
            ));
        } else if let Some(caps) = class_re.captures(line) {
            let indent = caps.get(1).map_or(0, |m| m.as_str().len());

            // Skip nested classes
            if indent > 0 {
                continue;
            }

            let sig = caps.get(0).unwrap().as_str().trim();
            items.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                0,
                ItemKind::Class,
            ));
        } else if let Some(caps) = module_re.captures(line) {
            let indent = caps.get(1).map_or(0, |m| m.as_str().len());

            // Skip nested modules
            if indent > 0 {
                continue;
            }

            let sig = caps.get(0).unwrap().as_str().trim();
            items.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                0,
                ItemKind::Module,
            ));
        }
    }

    items
}

/// Extract Go public interface (exported items - capitalized)
fn extract_go_interface(content: &str) -> Vec<OutlineItem> {
    let mut items = Vec::new();

    let func_re =
        Regex::new(r"^func\s+(\([^)]+\)\s+)?([A-Z]\w*)\s*\([^)]*\)(\s*\([^)]*\)|\s*\w+)?").unwrap();
    let type_struct_re = Regex::new(r"^type\s+([A-Z]\w*)\s+struct").unwrap();
    let type_interface_re = Regex::new(r"^type\s+([A-Z]\w*)\s+interface").unwrap();

    for (line_num, line) in content.lines().enumerate() {
        let line_num = line_num + 1;

        if let Some(caps) = func_re.captures(line) {
            let sig = caps.get(0).unwrap().as_str().trim();
            items.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                0,
                ItemKind::Function,
            ));
        } else if let Some(caps) = type_struct_re.captures(line) {
            let sig = caps.get(0).unwrap().as_str().trim();
            items.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                0,
                ItemKind::Struct,
            ));
        } else if let Some(caps) = type_interface_re.captures(line) {
            let sig = caps.get(0).unwrap().as_str().trim();
            items.push(OutlineItem::new(
                line_num,
                sig.to_string(),
                0,
                ItemKind::Trait,
            ));
        }
    }

    items
}
