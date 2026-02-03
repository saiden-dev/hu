use super::*;

#[test]
fn rust_pub_fn() {
    let content = "pub fn test() {}";
    let items = extract_interface(content, "test.rs");
    assert_eq!(items.len(), 1);
    assert!(items[0].text.contains("pub fn test"));
}

#[test]
fn rust_private_fn_excluded() {
    let content = "fn private_test() {}";
    let items = extract_interface(content, "test.rs");
    assert!(items.is_empty());
}

#[test]
fn rust_pub_struct() {
    let content = "pub struct Config {}";
    let items = extract_interface(content, "test.rs");
    assert_eq!(items.len(), 1);
    assert!(items[0].text.contains("pub struct Config"));
}

#[test]
fn rust_pub_enum() {
    let content = "pub enum Status { Ok, Err }";
    let items = extract_interface(content, "test.rs");
    assert_eq!(items.len(), 1);
    assert!(items[0].text.contains("pub enum Status"));
}

#[test]
fn rust_pub_trait() {
    let content = "pub trait Handler {}";
    let items = extract_interface(content, "test.rs");
    assert_eq!(items.len(), 1);
    assert!(items[0].text.contains("pub trait Handler"));
}

#[test]
fn rust_pub_const() {
    let content = "pub const MAX: u32 = 100;";
    let items = extract_interface(content, "test.rs");
    assert_eq!(items.len(), 1);
    assert!(items[0].text.contains("pub const MAX"));
}

#[test]
fn rust_pub_type() {
    let content = "pub type Result<T> = std::result::Result<T, Error>;";
    let items = extract_interface(content, "test.rs");
    assert_eq!(items.len(), 1);
    assert!(items[0].text.contains("pub type Result"));
}

#[test]
fn rust_pub_mod() {
    let content = "pub mod utils;";
    let items = extract_interface(content, "test.rs");
    assert_eq!(items.len(), 1);
    assert!(items[0].text.contains("pub mod utils"));
}

#[test]
fn python_public_function() {
    let content = "def public_fn():";
    let items = extract_interface(content, "test.py");
    assert_eq!(items.len(), 1);
}

#[test]
fn python_private_function_excluded() {
    let content = "def _private_fn():";
    let items = extract_interface(content, "test.py");
    assert!(items.is_empty());
}

#[test]
fn python_dunder_included() {
    let content = "def __init__(self):";
    let items = extract_interface(content, "test.py");
    assert_eq!(items.len(), 1);
}

#[test]
fn python_public_class() {
    let content = "class Handler:";
    let items = extract_interface(content, "test.py");
    assert_eq!(items.len(), 1);
}

#[test]
fn python_private_class_excluded() {
    let content = "class _Private:";
    let items = extract_interface(content, "test.py");
    assert!(items.is_empty());
}

#[test]
fn python_method_excluded() {
    let content = r#"class Test:
    def method(self):
        pass
"#;
    let items = extract_interface(content, "test.py");
    // Only class, not method
    assert_eq!(items.len(), 1);
    assert!(items[0].text.contains("class Test"));
}

#[test]
fn js_export_function() {
    let content = "export function test() {}";
    let items = extract_interface(content, "test.js");
    assert_eq!(items.len(), 1);
    assert!(items[0].text.contains("export function test"));
}

#[test]
fn js_non_export_excluded() {
    let content = "function internal() {}";
    let items = extract_interface(content, "test.js");
    assert!(items.is_empty());
}

#[test]
fn js_export_arrow() {
    let content = "export const handler = (req) =>";
    let items = extract_interface(content, "test.js");
    assert_eq!(items.len(), 1);
}

#[test]
fn js_export_class() {
    let content = "export class Service {}";
    let items = extract_interface(content, "test.js");
    assert_eq!(items.len(), 1);
    assert!(items[0].text.contains("export class Service"));
}

#[test]
fn js_export_default() {
    let content = "export default function";
    let items = extract_interface(content, "test.js");
    assert_eq!(items.len(), 1);
}

#[test]
fn ruby_public_method() {
    let content = "def public_method\nend";
    let items = extract_interface(content, "test.rb");
    assert_eq!(items.len(), 1);
}

#[test]
fn ruby_private_method_excluded() {
    let content = r#"
class Test
  def public
  end

  private

  def private_method
  end
end
"#;
    let items = extract_interface(content, "test.rb");
    // Only class and public method
    assert_eq!(items.len(), 2);
}

#[test]
fn ruby_class() {
    let content = "class Handler";
    let items = extract_interface(content, "test.rb");
    assert_eq!(items.len(), 1);
    assert!(items[0].text.contains("class Handler"));
}

#[test]
fn ruby_module() {
    let content = "module Utils";
    let items = extract_interface(content, "test.rb");
    assert_eq!(items.len(), 1);
    assert!(items[0].text.contains("module Utils"));
}

#[test]
fn go_exported_func() {
    let content = "func Handler(w http.ResponseWriter) {}";
    let items = extract_interface(content, "test.go");
    assert_eq!(items.len(), 1);
}

#[test]
fn go_unexported_func_excluded() {
    let content = "func internal() {}";
    let items = extract_interface(content, "test.go");
    assert!(items.is_empty());
}

#[test]
fn go_exported_struct() {
    let content = "type Config struct {}";
    let items = extract_interface(content, "test.go");
    assert_eq!(items.len(), 1);
}

#[test]
fn go_unexported_struct_excluded() {
    let content = "type config struct {}";
    let items = extract_interface(content, "test.go");
    assert!(items.is_empty());
}

#[test]
fn go_exported_interface() {
    let content = "type Handler interface {}";
    let items = extract_interface(content, "test.go");
    assert_eq!(items.len(), 1);
}

#[test]
fn unknown_extension() {
    let content = "some content";
    let items = extract_interface(content, "test.xyz");
    assert!(items.is_empty());
}

#[test]
fn empty_content() {
    let items = extract_interface("", "test.rs");
    assert!(items.is_empty());
}

#[test]
fn python_nested_class_excluded() {
    // Nested classes (indented) should be excluded
    let content = r#"class Outer:
    class Inner:
        pass
"#;
    let items = extract_interface(content, "test.py");
    // Only top-level class
    assert_eq!(items.len(), 1);
    assert!(items[0].text.contains("class Outer"));
}

#[test]
fn ruby_public_after_private() {
    // public keyword should reset private state
    let content = r#"class Test
  private

  def private_method
  end

  public

  def public_again
  end
end
"#;
    let items = extract_interface(content, "test.rb");
    // class + public_again (private_method is excluded)
    assert_eq!(items.len(), 2);
    assert!(items.iter().any(|i| i.text.contains("class Test")));
    assert!(items.iter().any(|i| i.text.contains("def public_again")));
    assert!(!items.iter().any(|i| i.text.contains("private_method")));
}

#[test]
fn ruby_nested_method_excluded() {
    // Deeply nested methods (indent > 2) should be excluded
    let content = r#"class Test
  def outer
      def inner_method
      end
  end
end
"#;
    let items = extract_interface(content, "test.rb");
    // class + outer method, but not inner_method
    assert_eq!(items.len(), 2);
    assert!(items.iter().any(|i| i.text.contains("class Test")));
    assert!(items.iter().any(|i| i.text.contains("def outer")));
    assert!(!items.iter().any(|i| i.text.contains("inner_method")));
}

#[test]
fn ruby_nested_class_excluded() {
    // Nested classes should be excluded
    let content = r#"class Outer
  class Inner
  end
end
"#;
    let items = extract_interface(content, "test.rb");
    // Only top-level class
    assert_eq!(items.len(), 1);
    assert!(items[0].text.contains("class Outer"));
}

#[test]
fn ruby_nested_module_excluded() {
    // Nested modules should be excluded
    let content = r#"module Outer
  module Inner
  end
end
"#;
    let items = extract_interface(content, "test.rb");
    // Only top-level module
    assert_eq!(items.len(), 1);
    assert!(items[0].text.contains("module Outer"));
}
