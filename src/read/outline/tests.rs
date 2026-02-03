use super::*;

#[test]
fn rust_function() {
    let content = "pub fn test(x: i32) -> String {";
    let outline = extract_outline(content, "test.rs");
    assert_eq!(outline.len(), 1);
    assert!(outline.items[0].text.contains("pub fn test"));
    assert_eq!(outline.items[0].kind, ItemKind::Function);
}

#[test]
fn rust_async_function() {
    let content = "pub async fn fetch() -> Result<()> {";
    let outline = extract_outline(content, "test.rs");
    assert_eq!(outline.len(), 1);
    assert!(outline.items[0].text.contains("async fn fetch"));
}

#[test]
fn rust_struct() {
    let content = "pub struct Config<T> {";
    let outline = extract_outline(content, "test.rs");
    assert_eq!(outline.len(), 1);
    assert!(outline.items[0].text.contains("struct Config"));
    assert_eq!(outline.items[0].kind, ItemKind::Struct);
}

#[test]
fn rust_enum() {
    let content = "pub enum Status {";
    let outline = extract_outline(content, "test.rs");
    assert_eq!(outline.len(), 1);
    assert!(outline.items[0].text.contains("enum Status"));
    assert_eq!(outline.items[0].kind, ItemKind::Enum);
}

#[test]
fn rust_trait() {
    let content = "pub trait Handler {";
    let outline = extract_outline(content, "test.rs");
    assert_eq!(outline.len(), 1);
    assert!(outline.items[0].text.contains("trait Handler"));
    assert_eq!(outline.items[0].kind, ItemKind::Trait);
}

#[test]
fn rust_impl() {
    let content = "impl Config {";
    let outline = extract_outline(content, "test.rs");
    assert_eq!(outline.len(), 1);
    assert!(outline.items[0].text.contains("impl Config"));
    assert_eq!(outline.items[0].kind, ItemKind::Impl);
}

#[test]
fn rust_impl_for() {
    let content = "impl Handler for Config {";
    let outline = extract_outline(content, "test.rs");
    assert_eq!(outline.len(), 1);
    assert!(outline.items[0].text.contains("impl Handler for Config"));
}

#[test]
fn rust_mod() {
    let content = "pub mod utils;";
    let outline = extract_outline(content, "test.rs");
    assert_eq!(outline.len(), 1);
    assert!(outline.items[0].text.contains("mod utils"));
    assert_eq!(outline.items[0].kind, ItemKind::Module);
}

#[test]
fn rust_const() {
    let content = "pub const MAX_SIZE: usize = 1024;";
    let outline = extract_outline(content, "test.rs");
    assert_eq!(outline.len(), 1);
    assert!(outline.items[0].text.contains("const MAX_SIZE"));
    assert_eq!(outline.items[0].kind, ItemKind::Const);
}

#[test]
fn rust_type() {
    let content = "pub type Result<T> = std::result::Result<T, Error>;";
    let outline = extract_outline(content, "test.rs");
    assert_eq!(outline.len(), 1);
    assert!(outline.items[0].text.contains("type Result"));
    assert_eq!(outline.items[0].kind, ItemKind::Type);
}

#[test]
fn rust_nested() {
    let content = r#"
impl Config {
    pub fn new() -> Self {
    }
}
"#;
    let outline = extract_outline(content, "test.rs");
    assert_eq!(outline.len(), 2);
    assert_eq!(outline.items[0].level, 0);
    assert_eq!(outline.items[1].level, 1);
}

#[test]
fn python_function() {
    let content = "def process(data: list) -> dict:";
    let outline = extract_outline(content, "test.py");
    assert_eq!(outline.len(), 1);
    assert!(outline.items[0].text.contains("def process"));
    assert_eq!(outline.items[0].kind, ItemKind::Function);
}

#[test]
fn python_async_function() {
    let content = "async def fetch(url):";
    let outline = extract_outline(content, "test.py");
    assert_eq!(outline.len(), 1);
    assert!(outline.items[0].text.contains("async def fetch"));
}

#[test]
fn python_class() {
    let content = "class Handler(BaseHandler):";
    let outline = extract_outline(content, "test.py");
    assert_eq!(outline.len(), 1);
    assert!(outline.items[0].text.contains("class Handler"));
    assert_eq!(outline.items[0].kind, ItemKind::Class);
}

#[test]
fn python_nested() {
    let content = r#"
class Handler:
    def process(self):
        pass
"#;
    let outline = extract_outline(content, "test.py");
    assert_eq!(outline.len(), 2);
    assert_eq!(outline.items[0].level, 0);
    assert_eq!(outline.items[1].level, 1);
}

#[test]
fn js_function() {
    let content = "export async function fetchData(url) {";
    let outline = extract_outline(content, "test.js");
    assert_eq!(outline.len(), 1);
    assert!(outline.items[0].text.contains("function fetchData"));
    assert_eq!(outline.items[0].kind, ItemKind::Function);
}

#[test]
fn js_arrow_function() {
    let content = "const handler = async (req, res) =>";
    let outline = extract_outline(content, "test.js");
    assert_eq!(outline.len(), 1);
    assert!(outline.items[0].text.contains("const handler"));
}

#[test]
fn js_class() {
    let content = "export class UserService extends Service {";
    let outline = extract_outline(content, "test.js");
    assert_eq!(outline.len(), 1);
    assert!(outline.items[0].text.contains("class UserService"));
    assert_eq!(outline.items[0].kind, ItemKind::Class);
}

#[test]
fn ts_function() {
    let content = "export function process<T>(data: T[]): T[] {";
    let outline = extract_outline(content, "test.ts");
    assert_eq!(outline.len(), 1);
    assert!(outline.items[0].text.contains("function process"));
}

#[test]
fn ruby_def() {
    let content = "def process(data)";
    let outline = extract_outline(content, "test.rb");
    assert_eq!(outline.len(), 1);
    assert!(outline.items[0].text.contains("def process"));
    assert_eq!(outline.items[0].kind, ItemKind::Function);
}

#[test]
fn ruby_predicate() {
    let content = "def valid?";
    let outline = extract_outline(content, "test.rb");
    assert_eq!(outline.len(), 1);
    assert!(outline.items[0].text.contains("def valid?"));
}

#[test]
fn ruby_class() {
    let content = "class Handler < BaseHandler";
    let outline = extract_outline(content, "test.rb");
    assert_eq!(outline.len(), 1);
    assert!(outline.items[0].text.contains("class Handler"));
    assert_eq!(outline.items[0].kind, ItemKind::Class);
}

#[test]
fn ruby_module() {
    let content = "module Utils";
    let outline = extract_outline(content, "test.rb");
    assert_eq!(outline.len(), 1);
    assert!(outline.items[0].text.contains("module Utils"));
    assert_eq!(outline.items[0].kind, ItemKind::Module);
}

#[test]
fn go_func() {
    let content = "func (s *Server) Handle(w http.ResponseWriter, r *http.Request) {";
    let outline = extract_outline(content, "test.go");
    assert_eq!(outline.len(), 1);
    assert!(outline.items[0].text.contains("func"));
    assert!(outline.items[0].text.contains("Handle"));
    assert_eq!(outline.items[0].kind, ItemKind::Function);
}

#[test]
fn go_struct() {
    let content = "type Config struct {";
    let outline = extract_outline(content, "test.go");
    assert_eq!(outline.len(), 1);
    assert!(outline.items[0].text.contains("type Config struct"));
    assert_eq!(outline.items[0].kind, ItemKind::Struct);
}

#[test]
fn go_interface() {
    let content = "type Handler interface {";
    let outline = extract_outline(content, "test.go");
    assert_eq!(outline.len(), 1);
    assert!(outline.items[0].text.contains("type Handler interface"));
    assert_eq!(outline.items[0].kind, ItemKind::Trait);
}

#[test]
fn markdown_headings() {
    let content = r#"
# Title
## Section 1
### Subsection
## Section 2
"#;
    let outline = extract_outline(content, "test.md");
    assert_eq!(outline.len(), 4);
    assert_eq!(outline.items[0].text, "Title");
    assert_eq!(outline.items[0].kind, ItemKind::Heading(1));
    assert_eq!(outline.items[0].level, 0);
    assert_eq!(outline.items[1].text, "Section 1");
    assert_eq!(outline.items[1].kind, ItemKind::Heading(2));
    assert_eq!(outline.items[1].level, 1);
    assert_eq!(outline.items[2].text, "Subsection");
    assert_eq!(outline.items[2].kind, ItemKind::Heading(3));
    assert_eq!(outline.items[2].level, 2);
}

#[test]
fn unknown_extension() {
    let content = "some content";
    let outline = extract_outline(content, "test.xyz");
    assert!(outline.is_empty());
}

#[test]
fn empty_content() {
    let outline = extract_outline("", "test.rs");
    assert!(outline.is_empty());
}

#[test]
fn line_numbers_correct() {
    let content = r#"
pub fn first() {}
pub fn second() {}
"#;
    let outline = extract_outline(content, "test.rs");
    assert_eq!(outline.len(), 2);
    assert_eq!(outline.items[0].line, 2);
    assert_eq!(outline.items[1].line, 3);
}

#[test]
fn js_class_methods() {
    // Test that methods inside JavaScript classes are detected
    let content = r#"class UserService {
  constructor(db) {
    this.db = db;
  }

  async findById(id) {
    return this.db.find(id);
  }

  delete(id) {
  }
}
"#;
    let outline = extract_outline(content, "test.js");
    // class + 3 methods (constructor, findById, delete)
    assert_eq!(outline.len(), 4);
    assert!(outline.items[0].text.contains("class UserService"));
    assert!(outline.items[1].text.contains("constructor"));
    assert!(outline.items[2].text.contains("findById"));
    assert!(outline.items[3].text.contains("delete"));
}

#[test]
fn js_method_async() {
    // Test async methods inside class
    let content = r#"class Api {
  async fetch(url) {
  }
}
"#;
    let outline = extract_outline(content, "test.js");
    assert_eq!(outline.len(), 2);
    assert!(outline.items[1].text.contains("async fetch"));
}
