/// Strip line number suffix from a path (e.g., "spec/foo_spec.rb:10" -> "spec/foo_spec.rb")
pub fn strip_line_number(path: &str) -> &str {
    // Find last colon followed by digits
    if let Some(idx) = path.rfind(':') {
        if path[idx + 1..].chars().all(|c| c.is_ascii_digit()) && !path[idx + 1..].is_empty() {
            return &path[..idx];
        }
    }
    path
}

/// Detect the language/framework from a test file path
pub fn detect_language(test_file: &str) -> &'static str {
    let path = strip_line_number(test_file);

    if path.ends_with("_spec.rb") || path.starts_with("spec/") || path.starts_with("./spec/") {
        "ruby"
    } else if path.ends_with("_test.py")
        || (path.ends_with(".py") && (path.starts_with("tests/test_") || path.starts_with("test_")))
    {
        "python"
    } else if path.ends_with(".test.js")
        || path.ends_with(".test.ts")
        || path.ends_with(".test.tsx")
        || path.ends_with(".test.jsx")
        || path.ends_with(".spec.js")
        || path.ends_with(".spec.ts")
        || path.ends_with(".spec.tsx")
        || path.ends_with(".spec.jsx")
    {
        "javascript"
    } else if path.ends_with(".rs")
        || (path.starts_with("tests/") && !path.ends_with(".py"))
        || path.contains("/tests.rs")
        || path.contains("/tests/")
    {
        "rust"
    } else {
        "unknown"
    }
}

/// Map a test file path to likely source file paths
pub fn map_test_to_source(test_file: &str) -> Vec<String> {
    let path = strip_line_number(test_file);
    let lang = detect_language(test_file);

    match lang {
        "ruby" => map_rspec(path),
        "rust" => map_rust(path),
        "python" => map_python(path),
        "javascript" => map_javascript(path),
        _ => vec![],
    }
}

/// Map RSpec test file to Ruby source files
/// spec/models/user_spec.rb -> app/models/user.rb
/// spec/helpers/pricing_helper_spec.rb -> app/helpers/pricing_helper.rb
fn map_rspec(path: &str) -> Vec<String> {
    let path = path
        .strip_prefix("./")
        .unwrap_or(path)
        .strip_prefix("spec/")
        .unwrap_or(path);

    let path = path.strip_suffix("_spec.rb").unwrap_or(path);

    vec![format!("app/{}.rb", path), format!("lib/{}.rb", path)]
}

/// Map Rust test file to source files
/// tests/test_sync.rs -> src/sync.rs
/// src/data/tests.rs -> src/data/mod.rs
fn map_rust(path: &str) -> Vec<String> {
    let path = path.strip_prefix("./").unwrap_or(path);

    if let Some(parent) = path.strip_suffix("/tests.rs") {
        return vec![format!("{}/mod.rs", parent)];
    }

    if let Some(rest) = path.strip_prefix("tests/") {
        let without_prefix = rest.strip_prefix("test_").unwrap_or(rest);
        let name = without_prefix.strip_suffix(".rs").unwrap_or(without_prefix);
        return vec![format!("src/{}.rs", name), format!("src/{}/mod.rs", name)];
    }

    vec![]
}

/// Map Python test file to source files
/// tests/test_utils.py -> src/utils.py, utils.py
/// utils_test.py -> utils.py
fn map_python(path: &str) -> Vec<String> {
    let path = path.strip_prefix("./").unwrap_or(path);

    if path.starts_with("tests/test_") || path.starts_with("test_") {
        let name = path
            .strip_prefix("tests/")
            .unwrap_or(path)
            .strip_prefix("test_")
            .unwrap_or(path);
        return vec![format!("src/{}", name), name.to_string()];
    }

    if let Some(base) = path.strip_suffix("_test.py") {
        return vec![format!("{}.py", base), format!("src/{}.py", base)];
    }

    vec![]
}

/// Map JS/TS test file to source files
/// components/Button.test.tsx -> components/Button.tsx
/// utils/format.spec.ts -> utils/format.ts
fn map_javascript(path: &str) -> Vec<String> {
    let path = path.strip_prefix("./").unwrap_or(path);

    for suffix in &[
        ".test.js",
        ".test.ts",
        ".test.tsx",
        ".test.jsx",
        ".spec.js",
        ".spec.ts",
        ".spec.tsx",
        ".spec.jsx",
    ] {
        if let Some(base) = path.strip_suffix(suffix) {
            let ext = &suffix[suffix.rfind('.').unwrap_or(0)..];
            return vec![format!("{}{}", base, ext)];
        }
    }

    vec![]
}

#[cfg(test)]
mod tests;
