use super::*;

// strip_line_number tests
#[test]
fn strip_line_number_with_line() {
    assert_eq!(strip_line_number("spec/foo_spec.rb:10"), "spec/foo_spec.rb");
}

#[test]
fn strip_line_number_without_line() {
    assert_eq!(strip_line_number("spec/foo_spec.rb"), "spec/foo_spec.rb");
}

#[test]
fn strip_line_number_colon_no_digits() {
    assert_eq!(strip_line_number("foo:bar"), "foo:bar");
}

#[test]
fn strip_line_number_empty() {
    assert_eq!(strip_line_number(""), "");
}

#[test]
fn strip_line_number_multiple_colons() {
    assert_eq!(
        strip_line_number("./spec/foo_spec.rb:10"),
        "./spec/foo_spec.rb"
    );
}

#[test]
fn strip_line_number_trailing_colon() {
    assert_eq!(strip_line_number("foo:"), "foo:");
}

// detect_language tests
#[test]
fn detect_ruby() {
    assert_eq!(detect_language("spec/models/user_spec.rb"), "ruby");
    assert_eq!(detect_language("./spec/models/user_spec.rb:10"), "ruby");
    assert_eq!(detect_language("spec/helpers/foo_spec.rb"), "ruby");
}

#[test]
fn detect_rust() {
    assert_eq!(detect_language("tests/test_sync.rs"), "rust");
    assert_eq!(detect_language("src/data/tests.rs"), "rust");
    assert_eq!(detect_language("src/gh/fix/tests.rs"), "rust");
}

#[test]
fn detect_python() {
    assert_eq!(detect_language("tests/test_utils.py"), "python");
    assert_eq!(detect_language("utils_test.py"), "python");
    assert_eq!(detect_language("test_main.py"), "python");
}

#[test]
fn detect_javascript() {
    assert_eq!(detect_language("Button.test.tsx"), "javascript");
    assert_eq!(detect_language("utils/format.spec.ts"), "javascript");
    assert_eq!(detect_language("app.test.js"), "javascript");
    assert_eq!(detect_language("component.spec.jsx"), "javascript");
}

#[test]
fn detect_unknown() {
    assert_eq!(detect_language("README.md"), "unknown");
    assert_eq!(detect_language("main.go"), "unknown");
    assert_eq!(detect_language(""), "unknown");
}

// map_test_to_source: Ruby
#[test]
fn map_rspec_model() {
    let sources = map_test_to_source("spec/models/user_spec.rb");
    assert!(sources.contains(&"app/models/user.rb".to_string()));
    assert!(sources.contains(&"lib/models/user.rb".to_string()));
}

#[test]
fn map_rspec_helper() {
    let sources = map_test_to_source("spec/helpers/pricing_helper_spec.rb:289");
    assert!(sources.contains(&"app/helpers/pricing_helper.rb".to_string()));
}

#[test]
fn map_rspec_with_dot_prefix() {
    let sources = map_test_to_source("./spec/models/user_spec.rb:10");
    assert!(sources.contains(&"app/models/user.rb".to_string()));
}

#[test]
fn map_rspec_nested_path() {
    let sources = map_test_to_source("spec/features/admin/users/permissions_spec.rb:42");
    assert!(sources.contains(&"app/features/admin/users/permissions.rb".to_string()));
}

// map_test_to_source: Rust
#[test]
fn map_rust_integration_test() {
    let sources = map_test_to_source("tests/test_sync.rs");
    assert!(sources.contains(&"src/sync.rs".to_string()));
    assert!(sources.contains(&"src/sync/mod.rs".to_string()));
}

#[test]
fn map_rust_module_tests() {
    let sources = map_test_to_source("src/data/tests.rs");
    assert_eq!(sources, vec!["src/data/mod.rs"]);
}

#[test]
fn map_rust_integration_test_no_prefix() {
    let sources = map_test_to_source("tests/utils.rs");
    assert!(sources.contains(&"src/utils.rs".to_string()));
}

// map_test_to_source: Python
#[test]
fn map_python_test_file() {
    let sources = map_test_to_source("tests/test_utils.py");
    assert!(sources.contains(&"src/utils.py".to_string()));
    assert!(sources.contains(&"utils.py".to_string()));
}

#[test]
fn map_python_suffix_test() {
    let sources = map_test_to_source("utils_test.py");
    assert!(sources.contains(&"utils.py".to_string()));
    assert!(sources.contains(&"src/utils.py".to_string()));
}

#[test]
fn map_python_bare_test() {
    let sources = map_test_to_source("test_main.py");
    assert!(sources.contains(&"src/main.py".to_string()));
    assert!(sources.contains(&"main.py".to_string()));
}

// map_test_to_source: JavaScript/TypeScript
#[test]
fn map_js_test() {
    let sources = map_test_to_source("components/Button.test.tsx");
    assert_eq!(sources, vec!["components/Button.tsx"]);
}

#[test]
fn map_js_spec() {
    let sources = map_test_to_source("utils/format.spec.ts");
    assert_eq!(sources, vec!["utils/format.ts"]);
}

#[test]
fn map_js_plain() {
    let sources = map_test_to_source("app.test.js");
    assert_eq!(sources, vec!["app.js"]);
}

#[test]
fn map_jsx_spec() {
    let sources = map_test_to_source("component.spec.jsx");
    assert_eq!(sources, vec!["component.jsx"]);
}

// map_test_to_source: Unknown
#[test]
fn map_unknown_returns_empty() {
    let sources = map_test_to_source("README.md");
    assert!(sources.is_empty());
}

#[test]
fn map_empty_returns_empty() {
    let sources = map_test_to_source("");
    assert!(sources.is_empty());
}

// Edge cases
#[test]
fn map_with_line_number_stripped() {
    let sources = map_test_to_source("spec/models/user_spec.rb:42");
    assert!(sources.contains(&"app/models/user.rb".to_string()));
}

#[test]
fn detect_language_with_line_number() {
    assert_eq!(detect_language("tests/test_sync.rs:100"), "rust");
    assert_eq!(detect_language("app.test.js:55"), "javascript");
}

// Direct mapper fallback tests
#[test]
fn map_rust_non_test_file() {
    assert!(map_rust("src/main.rs").is_empty());
}

#[test]
fn map_python_non_test_file() {
    assert!(map_python("main.py").is_empty());
}

#[test]
fn map_javascript_non_test_file() {
    assert!(map_javascript("index.js").is_empty());
}
