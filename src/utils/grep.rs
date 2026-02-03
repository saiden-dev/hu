use anyhow::{Context, Result};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use super::cli::GrepArgs;

/// A single grep match
#[derive(Debug, Clone)]
pub struct GrepMatch {
    pub file: String,
    pub line_num: usize,
    pub content: String,
    pub match_count: usize,
}

/// Handle the `hu utils grep` command
pub fn run(args: GrepArgs) -> Result<()> {
    let matches = search_files(&args)?;

    if matches.is_empty() {
        eprintln!("No matches found.");
        return Ok(());
    }

    let output = format_matches(&matches, &args);
    println!("{}", output);

    Ok(())
}

/// Search files for pattern
pub fn search_files(args: &GrepArgs) -> Result<Vec<GrepMatch>> {
    let re = if args.ignore_case {
        Regex::new(&format!("(?i){}", &args.pattern))
    } else {
        Regex::new(&args.pattern)
    }
    .with_context(|| format!("Invalid regex pattern: {}", args.pattern))?;

    let glob_pattern = args.glob.as_deref();
    let mut matches = Vec::new();

    collect_matches(&args.path, &re, glob_pattern, args.hidden, &mut matches)?;

    // Apply post-processing
    let mut matches = if args.unique {
        dedupe_matches(matches)
    } else {
        matches
    };

    if args.ranked {
        rank_matches(&mut matches);
    }

    if let Some(limit) = args.limit {
        matches.truncate(limit);
    }

    Ok(matches)
}

/// Recursively collect matches from files
fn collect_matches(
    path: &str,
    re: &Regex,
    glob_pattern: Option<&str>,
    include_hidden: bool,
    matches: &mut Vec<GrepMatch>,
) -> Result<()> {
    let path = Path::new(path);

    if path.is_file() {
        if should_search_file(path, glob_pattern) {
            search_file(path, re, matches)?;
        }
        return Ok(());
    }

    if !path.is_dir() {
        return Ok(());
    }

    let entries =
        fs::read_dir(path).with_context(|| format!("Failed to read directory: {:?}", path))?;

    for entry in entries.flatten() {
        let entry_path = entry.path();
        let file_name = entry_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        // Skip hidden files unless requested
        if !include_hidden && file_name.starts_with('.') {
            continue;
        }

        // Skip common non-code directories
        if entry_path.is_dir() && is_ignored_dir(file_name) {
            continue;
        }

        if entry_path.is_dir() {
            collect_matches(
                entry_path.to_str().unwrap_or(""),
                re,
                glob_pattern,
                include_hidden,
                matches,
            )?;
        } else if should_search_file(&entry_path, glob_pattern) {
            search_file(&entry_path, re, matches)?;
        }
    }

    Ok(())
}

/// Check if a directory should be ignored
fn is_ignored_dir(name: &str) -> bool {
    matches!(
        name,
        "node_modules"
            | "target"
            | ".git"
            | ".svn"
            | ".hg"
            | "__pycache__"
            | ".mypy_cache"
            | ".pytest_cache"
            | "venv"
            | ".venv"
            | "dist"
            | "build"
            | ".next"
            | ".nuxt"
    )
}

/// Check if a file matches the glob pattern
fn should_search_file(path: &Path, glob_pattern: Option<&str>) -> bool {
    // Skip binary files
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    if is_binary_extension(ext) {
        return false;
    }

    // If no glob, search all text files
    let Some(pattern) = glob_pattern else {
        return true;
    };

    // Simple glob matching
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    glob_matches(file_name, pattern)
}

/// Check if extension indicates binary file
fn is_binary_extension(ext: &str) -> bool {
    matches!(
        ext.to_lowercase().as_str(),
        "png"
            | "jpg"
            | "jpeg"
            | "gif"
            | "ico"
            | "webp"
            | "bmp"
            | "svg"
            | "pdf"
            | "zip"
            | "tar"
            | "gz"
            | "bz2"
            | "xz"
            | "7z"
            | "rar"
            | "exe"
            | "dll"
            | "so"
            | "dylib"
            | "a"
            | "o"
            | "obj"
            | "wasm"
            | "class"
            | "jar"
            | "pyc"
            | "pyo"
            | "mp3"
            | "mp4"
            | "avi"
            | "mkv"
            | "mov"
            | "wav"
            | "flac"
            | "ttf"
            | "otf"
            | "woff"
            | "woff2"
            | "eot"
            | "sqlite"
            | "db"
    )
}

/// Simple glob matching (supports * and ?)
pub fn glob_matches(name: &str, pattern: &str) -> bool {
    let pattern = pattern.trim_start_matches("**/");

    if let Some(ext) = pattern.strip_prefix("*.") {
        // Extension match: *.rs
        name.ends_with(&format!(".{}", ext))
    } else if pattern.contains('*') {
        // Convert glob to regex
        let regex_pattern = pattern
            .replace('.', "\\.")
            .replace('*', ".*")
            .replace('?', ".");
        Regex::new(&format!("^{}$", regex_pattern))
            .map(|re| re.is_match(name))
            .unwrap_or(false)
    } else {
        // Exact match
        name == pattern
    }
}

/// Search a single file for matches
fn search_file(path: &Path, re: &Regex, matches: &mut Vec<GrepMatch>) -> Result<()> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Ok(()), // Skip unreadable files
    };

    let file_str = path.to_str().unwrap_or("");

    for (line_num, line) in content.lines().enumerate() {
        let match_count = re.find_iter(line).count();
        if match_count > 0 {
            matches.push(GrepMatch {
                file: file_str.to_string(),
                line_num: line_num + 1,
                content: line.to_string(),
                match_count,
            });
        }
    }

    Ok(())
}

/// Deduplicate similar matches
fn dedupe_matches(matches: Vec<GrepMatch>) -> Vec<GrepMatch> {
    let mut seen: HashMap<String, GrepMatch> = HashMap::new();

    for m in matches {
        // Normalize content for comparison (trim, collapse whitespace)
        let normalized = m.content.split_whitespace().collect::<Vec<_>>().join(" ");

        seen.entry(normalized)
            .and_modify(|existing| existing.match_count += m.match_count)
            .or_insert(m);
    }

    seen.into_values().collect()
}

/// Rank matches by relevance (match density)
fn rank_matches(matches: &mut [GrepMatch]) {
    matches.sort_by(|a, b| {
        // Higher match count first
        b.match_count
            .cmp(&a.match_count)
            // Then shorter content (more focused)
            .then_with(|| a.content.len().cmp(&b.content.len()))
    });
}

/// Format matches for output
pub fn format_matches(matches: &[GrepMatch], args: &GrepArgs) -> String {
    let mut output = Vec::new();

    for m in matches {
        if args.refs {
            // Just file:line reference
            output.push(format!("{}:{}", m.file, m.line_num));
        } else if args.signature {
            // Try to extract function signature
            if let Some(sig) = extract_signature(&m.content, &m.file) {
                output.push(format!("{}:{}: {}", m.file, m.line_num, sig));
            } else {
                output.push(format!("{}:{}: {}", m.file, m.line_num, m.content.trim()));
            }
        } else {
            // Full match with content
            output.push(format!("{}:{}: {}", m.file, m.line_num, m.content.trim()));
        }
    }

    output.join("\n")
}

/// Try to extract function/method signature from a line
pub fn extract_signature(line: &str, file: &str) -> Option<String> {
    let trimmed = line.trim();
    let ext = Path::new(file)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    match ext {
        "rs" => extract_rust_signature(trimmed),
        "py" => extract_python_signature(trimmed),
        "js" | "ts" | "jsx" | "tsx" => extract_js_signature(trimmed),
        "rb" => extract_ruby_signature(trimmed),
        "go" => extract_go_signature(trimmed),
        _ => None,
    }
}

/// Extract Rust function/struct signature
fn extract_rust_signature(line: &str) -> Option<String> {
    // fn name(...) -> Type
    if let Some(caps) =
        Regex::new(r"^(pub\s+)?(async\s+)?fn\s+(\w+)\s*(<[^>]+>)?\s*\([^)]*\)(\s*->\s*[^{]+)?")
            .ok()?
            .captures(line)
    {
        return Some(
            caps.get(0)?
                .as_str()
                .trim_end_matches('{')
                .trim()
                .to_string(),
        );
    }

    // struct/enum/impl
    if let Some(caps) = Regex::new(r"^(pub\s+)?(struct|enum|impl|trait)\s+(\w+)(<[^>]+>)?")
        .ok()?
        .captures(line)
    {
        return Some(caps.get(0)?.as_str().to_string());
    }

    None
}

/// Extract Python function/class signature
fn extract_python_signature(line: &str) -> Option<String> {
    // def name(...):
    if let Some(caps) = Regex::new(r"^(async\s+)?def\s+(\w+)\s*\([^)]*\)(\s*->\s*[^:]+)?:")
        .ok()?
        .captures(line)
    {
        return Some(caps.get(0)?.as_str().trim_end_matches(':').to_string());
    }

    // class Name:
    if let Some(caps) = Regex::new(r"^class\s+(\w+)(\([^)]*\))?:")
        .ok()?
        .captures(line)
    {
        return Some(caps.get(0)?.as_str().trim_end_matches(':').to_string());
    }

    None
}

/// Extract JavaScript/TypeScript function signature
fn extract_js_signature(line: &str) -> Option<String> {
    // function name(...)
    if let Some(caps) =
        Regex::new(r"^(export\s+)?(async\s+)?function\s+(\w+)\s*(<[^>]+>)?\s*\([^)]*\)")
            .ok()?
            .captures(line)
    {
        return Some(caps.get(0)?.as_str().to_string());
    }

    // const name = (...) =>
    if let Some(caps) =
        Regex::new(r"^(export\s+)?(const|let|var)\s+(\w+)\s*=\s*(async\s+)?\([^)]*\)\s*=>")
            .ok()?
            .captures(line)
    {
        return Some(
            caps.get(0)?
                .as_str()
                .trim_end_matches("=>")
                .trim()
                .to_string(),
        );
    }

    // class Name
    if let Some(caps) = Regex::new(r"^(export\s+)?class\s+(\w+)(\s+extends\s+\w+)?")
        .ok()?
        .captures(line)
    {
        return Some(caps.get(0)?.as_str().to_string());
    }

    None
}

/// Extract Ruby method/class signature
fn extract_ruby_signature(line: &str) -> Option<String> {
    // def name(...)
    if let Some(caps) = Regex::new(r"^def\s+(\w+[?!=]?)(\([^)]*\))?")
        .ok()?
        .captures(line)
    {
        return Some(caps.get(0)?.as_str().to_string());
    }

    // class Name
    if let Some(caps) = Regex::new(r"^class\s+(\w+)(\s*<\s*\w+)?")
        .ok()?
        .captures(line)
    {
        return Some(caps.get(0)?.as_str().to_string());
    }

    None
}

/// Extract Go function signature
fn extract_go_signature(line: &str) -> Option<String> {
    // func name(...)
    if let Some(caps) =
        Regex::new(r"^func\s+(\([^)]+\)\s+)?(\w+)\s*\([^)]*\)(\s*\([^)]*\)|\s*\w+)?")
            .ok()?
            .captures(line)
    {
        return Some(caps.get(0)?.as_str().to_string());
    }

    // type Name struct/interface
    if let Some(caps) = Regex::new(r"^type\s+(\w+)\s+(struct|interface)")
        .ok()?
        .captures(line)
    {
        return Some(caps.get(0)?.as_str().to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_matches_extension() {
        assert!(glob_matches("foo.rs", "*.rs"));
        assert!(glob_matches("bar.rs", "*.rs"));
        assert!(!glob_matches("foo.py", "*.rs"));
    }

    #[test]
    fn glob_matches_recursive() {
        assert!(glob_matches("foo.rs", "**/*.rs"));
    }

    #[test]
    fn glob_matches_exact() {
        assert!(glob_matches("Cargo.toml", "Cargo.toml"));
        assert!(!glob_matches("cargo.toml", "Cargo.toml"));
    }

    #[test]
    fn glob_matches_wildcard() {
        assert!(glob_matches("test_foo.rs", "test_*.rs"));
        assert!(!glob_matches("foo_test.rs", "test_*.rs"));
    }

    #[test]
    fn is_ignored_dir_common() {
        assert!(is_ignored_dir("node_modules"));
        assert!(is_ignored_dir("target"));
        assert!(is_ignored_dir(".git"));
        assert!(!is_ignored_dir("src"));
        assert!(!is_ignored_dir("lib"));
    }

    #[test]
    fn is_binary_extension_images() {
        assert!(is_binary_extension("png"));
        assert!(is_binary_extension("jpg"));
        assert!(is_binary_extension("gif"));
    }

    #[test]
    fn is_binary_extension_archives() {
        assert!(is_binary_extension("zip"));
        assert!(is_binary_extension("tar"));
        assert!(is_binary_extension("gz"));
    }

    #[test]
    fn is_binary_extension_code() {
        assert!(!is_binary_extension("rs"));
        assert!(!is_binary_extension("py"));
        assert!(!is_binary_extension("js"));
    }

    #[test]
    fn extract_rust_fn_signature() {
        let sig = extract_rust_signature("pub fn foo(x: i32) -> String {").unwrap();
        assert_eq!(sig, "pub fn foo(x: i32) -> String");
    }

    #[test]
    fn extract_rust_async_fn_signature() {
        let sig = extract_rust_signature("pub async fn fetch() -> Result<()> {").unwrap();
        assert_eq!(sig, "pub async fn fetch() -> Result<()>");
    }

    #[test]
    fn extract_rust_struct_signature() {
        let sig = extract_rust_signature("pub struct Config<T> {").unwrap();
        assert_eq!(sig, "pub struct Config<T>");
    }

    #[test]
    fn extract_python_def_signature() {
        let sig = extract_python_signature("def process(data: list) -> dict:").unwrap();
        assert_eq!(sig, "def process(data: list) -> dict");
    }

    #[test]
    fn extract_python_class_signature() {
        let sig = extract_python_signature("class Handler(BaseHandler):").unwrap();
        assert_eq!(sig, "class Handler(BaseHandler)");
    }

    #[test]
    fn extract_js_function_signature() {
        let sig = extract_js_signature("export async function fetchData(url) {").unwrap();
        assert_eq!(sig, "export async function fetchData(url)");
    }

    #[test]
    fn extract_js_arrow_signature() {
        let sig = extract_js_signature("const handler = async (req, res) =>").unwrap();
        assert_eq!(sig, "const handler = async (req, res)");
    }

    #[test]
    fn extract_ruby_def_signature() {
        let sig = extract_ruby_signature("def process(data)").unwrap();
        assert_eq!(sig, "def process(data)");
    }

    #[test]
    fn extract_ruby_predicate_signature() {
        let sig = extract_ruby_signature("def valid?").unwrap();
        assert_eq!(sig, "def valid?");
    }

    #[test]
    fn extract_go_func_signature() {
        let sig =
            extract_go_signature("func (s *Server) Handle(w http.ResponseWriter, r *http.Request)")
                .unwrap();
        assert!(sig.contains("func"));
        assert!(sig.contains("Handle"));
    }

    #[test]
    fn extract_signature_by_extension() {
        let sig = extract_signature("pub fn test() {", "foo.rs").unwrap();
        assert!(sig.contains("fn test"));

        let sig = extract_signature("def test():", "foo.py").unwrap();
        assert!(sig.contains("def test"));
    }

    #[test]
    fn format_matches_refs_mode() {
        let matches = vec![GrepMatch {
            file: "src/main.rs".to_string(),
            line_num: 42,
            content: "    let x = 1;".to_string(),
            match_count: 1,
        }];
        let args = GrepArgs {
            pattern: "x".to_string(),
            path: ".".to_string(),
            refs: true,
            unique: false,
            ranked: false,
            limit: None,
            signature: false,
            glob: None,
            ignore_case: false,
            hidden: false,
        };
        let output = format_matches(&matches, &args);
        assert_eq!(output, "src/main.rs:42");
    }

    #[test]
    fn format_matches_full_mode() {
        let matches = vec![GrepMatch {
            file: "src/main.rs".to_string(),
            line_num: 42,
            content: "    let x = 1;".to_string(),
            match_count: 1,
        }];
        let args = GrepArgs {
            pattern: "x".to_string(),
            path: ".".to_string(),
            refs: false,
            unique: false,
            ranked: false,
            limit: None,
            signature: false,
            glob: None,
            ignore_case: false,
            hidden: false,
        };
        let output = format_matches(&matches, &args);
        assert_eq!(output, "src/main.rs:42: let x = 1;");
    }

    #[test]
    fn dedupe_matches_combines_counts() {
        let matches = vec![
            GrepMatch {
                file: "a.rs".to_string(),
                line_num: 1,
                content: "let x = 1;".to_string(),
                match_count: 1,
            },
            GrepMatch {
                file: "b.rs".to_string(),
                line_num: 5,
                content: "let x = 1;".to_string(),
                match_count: 2,
            },
        ];
        let deduped = dedupe_matches(matches);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].match_count, 3); // 1 + 2
    }

    #[test]
    fn rank_matches_by_count() {
        let mut matches = vec![
            GrepMatch {
                file: "a.rs".to_string(),
                line_num: 1,
                content: "one match".to_string(),
                match_count: 1,
            },
            GrepMatch {
                file: "b.rs".to_string(),
                line_num: 2,
                content: "three matches".to_string(),
                match_count: 3,
            },
        ];
        rank_matches(&mut matches);
        assert_eq!(matches[0].match_count, 3); // Higher count first
    }

    #[test]
    fn search_files_respects_limit() {
        // Create a temp directory with test files
        let temp_dir = std::env::temp_dir().join("hu_grep_test");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Create test files
        std::fs::write(temp_dir.join("a.txt"), "test line 1\ntest line 2\n").unwrap();
        std::fs::write(temp_dir.join("b.txt"), "test line 3\n").unwrap();

        let args = GrepArgs {
            pattern: "test".to_string(),
            path: temp_dir.to_str().unwrap().to_string(),
            refs: false,
            unique: false,
            ranked: false,
            limit: Some(2),
            signature: false,
            glob: None,
            ignore_case: false,
            hidden: false,
        };

        let matches = search_files(&args).unwrap();
        assert_eq!(matches.len(), 2);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn search_files_respects_glob() {
        let temp_dir = std::env::temp_dir().join("hu_grep_glob_test");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        std::fs::write(temp_dir.join("foo.rs"), "test\n").unwrap();
        std::fs::write(temp_dir.join("bar.py"), "test\n").unwrap();

        let args = GrepArgs {
            pattern: "test".to_string(),
            path: temp_dir.to_str().unwrap().to_string(),
            refs: false,
            unique: false,
            ranked: false,
            limit: None,
            signature: false,
            glob: Some("*.rs".to_string()),
            ignore_case: false,
            hidden: false,
        };

        let matches = search_files(&args).unwrap();
        assert_eq!(matches.len(), 1);
        assert!(matches[0].file.ends_with("foo.rs"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn format_matches_signature_mode() {
        let matches = vec![GrepMatch {
            file: "src/main.rs".to_string(),
            line_num: 42,
            content: "pub fn process() {".to_string(),
            match_count: 1,
        }];
        let args = GrepArgs {
            pattern: "process".to_string(),
            path: ".".to_string(),
            refs: false,
            unique: false,
            ranked: false,
            limit: None,
            signature: true,
            glob: None,
            ignore_case: false,
            hidden: false,
        };
        let output = format_matches(&matches, &args);
        assert!(output.contains("pub fn process()"));
        assert!(!output.contains("{")); // signature strips the brace
    }

    #[test]
    fn format_matches_signature_no_match() {
        // When line doesn't match signature pattern, falls back to trimmed content
        let matches = vec![GrepMatch {
            file: "src/main.rs".to_string(),
            line_num: 42,
            content: "    let x = 1;".to_string(),
            match_count: 1,
        }];
        let args = GrepArgs {
            pattern: "x".to_string(),
            path: ".".to_string(),
            refs: false,
            unique: false,
            ranked: false,
            limit: None,
            signature: true,
            glob: None,
            ignore_case: false,
            hidden: false,
        };
        let output = format_matches(&matches, &args);
        assert!(output.contains("let x = 1;"));
    }

    #[test]
    fn extract_signature_unknown_extension() {
        let result = extract_signature("some random line", "file.xyz");
        assert!(result.is_none());
    }

    #[test]
    fn extract_js_class_signature() {
        let sig = extract_js_signature("export class UserService extends BaseService {").unwrap();
        assert!(sig.contains("class UserService"));
        assert!(sig.contains("extends BaseService"));
    }

    #[test]
    fn extract_ruby_class_with_inheritance() {
        let sig = extract_ruby_signature("class User < ActiveRecord::Base").unwrap();
        assert!(sig.contains("class User"));
    }

    #[test]
    fn extract_go_type_interface() {
        let sig = extract_go_signature("type Handler interface {").unwrap();
        assert_eq!(sig, "type Handler interface");
    }

    #[test]
    fn extract_python_async_def() {
        let sig = extract_python_signature("async def fetch_data(url: str) -> dict:").unwrap();
        assert!(sig.contains("async def fetch_data"));
    }

    #[test]
    fn should_search_file_binary_extension() {
        let path = std::path::Path::new("image.png");
        assert!(!should_search_file(path, None));
    }

    #[test]
    fn should_search_file_text_no_glob() {
        let path = std::path::Path::new("file.txt");
        assert!(should_search_file(path, None));
    }

    #[test]
    fn grep_match_debug() {
        let m = GrepMatch {
            file: "test.rs".to_string(),
            line_num: 1,
            content: "test".to_string(),
            match_count: 1,
        };
        let debug = format!("{:?}", m);
        assert!(debug.contains("GrepMatch"));
    }

    #[test]
    fn grep_match_clone() {
        let m = GrepMatch {
            file: "test.rs".to_string(),
            line_num: 1,
            content: "test".to_string(),
            match_count: 1,
        };
        let cloned = m.clone();
        assert_eq!(cloned.file, m.file);
        assert_eq!(cloned.line_num, m.line_num);
    }

    #[test]
    fn search_files_with_unique() {
        let temp_dir = std::env::temp_dir().join("hu_grep_unique_test");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Create files with duplicate content
        std::fs::write(temp_dir.join("a.txt"), "let x = 1;\n").unwrap();
        std::fs::write(temp_dir.join("b.txt"), "let x = 1;\n").unwrap();

        let args = GrepArgs {
            pattern: "let".to_string(),
            path: temp_dir.to_str().unwrap().to_string(),
            refs: false,
            unique: true,
            ranked: false,
            limit: None,
            signature: false,
            glob: None,
            ignore_case: false,
            hidden: false,
        };

        let matches = search_files(&args).unwrap();
        // Should dedupe to 1 match with combined count
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].match_count, 2);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn search_files_with_ranked() {
        let temp_dir = std::env::temp_dir().join("hu_grep_ranked_test");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        std::fs::write(temp_dir.join("a.txt"), "test\n").unwrap();
        std::fs::write(temp_dir.join("b.txt"), "test test test\n").unwrap();

        let args = GrepArgs {
            pattern: "test".to_string(),
            path: temp_dir.to_str().unwrap().to_string(),
            refs: false,
            unique: false,
            ranked: true,
            limit: None,
            signature: false,
            glob: None,
            ignore_case: false,
            hidden: false,
        };

        let matches = search_files(&args).unwrap();
        // First match should have higher count
        assert!(matches[0].match_count >= matches[1].match_count);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn collect_matches_skips_hidden() {
        let temp_dir = std::env::temp_dir().join("hu_grep_hidden_test");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::create_dir_all(temp_dir.join(".hidden")).unwrap();

        std::fs::write(temp_dir.join("visible.txt"), "test\n").unwrap();
        std::fs::write(temp_dir.join(".hidden").join("secret.txt"), "test\n").unwrap();

        let args = GrepArgs {
            pattern: "test".to_string(),
            path: temp_dir.to_str().unwrap().to_string(),
            refs: false,
            unique: false,
            ranked: false,
            limit: None,
            signature: false,
            glob: None,
            ignore_case: false,
            hidden: false, // Don't include hidden
        };

        let matches = search_files(&args).unwrap();
        assert_eq!(matches.len(), 1);
        assert!(matches[0].file.contains("visible"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn collect_matches_includes_hidden_when_requested() {
        let temp_dir = std::env::temp_dir().join("hu_grep_hidden_incl_test");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        std::fs::write(temp_dir.join(".hidden_file.txt"), "test\n").unwrap();
        std::fs::write(temp_dir.join("visible.txt"), "test\n").unwrap();

        let args = GrepArgs {
            pattern: "test".to_string(),
            path: temp_dir.to_str().unwrap().to_string(),
            refs: false,
            unique: false,
            ranked: false,
            limit: None,
            signature: false,
            glob: None,
            ignore_case: false,
            hidden: true, // Include hidden
        };

        let matches = search_files(&args).unwrap();
        assert_eq!(matches.len(), 2);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn search_files_skips_ignored_dirs() {
        let temp_dir = std::env::temp_dir().join("hu_grep_ignored_test");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::create_dir_all(temp_dir.join("node_modules")).unwrap();

        std::fs::write(temp_dir.join("app.js"), "test\n").unwrap();
        std::fs::write(temp_dir.join("node_modules").join("dep.js"), "test\n").unwrap();

        let args = GrepArgs {
            pattern: "test".to_string(),
            path: temp_dir.to_str().unwrap().to_string(),
            refs: false,
            unique: false,
            ranked: false,
            limit: None,
            signature: false,
            glob: None,
            ignore_case: false,
            hidden: false,
        };

        let matches = search_files(&args).unwrap();
        assert_eq!(matches.len(), 1);
        assert!(matches[0].file.contains("app.js"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn search_files_single_file_path() {
        let temp_dir = std::env::temp_dir().join("hu_grep_single_file_test");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let file_path = temp_dir.join("single.txt");
        std::fs::write(&file_path, "test line\n").unwrap();

        let args = GrepArgs {
            pattern: "test".to_string(),
            path: file_path.to_str().unwrap().to_string(),
            refs: false,
            unique: false,
            ranked: false,
            limit: None,
            signature: false,
            glob: None,
            ignore_case: false,
            hidden: false,
        };

        let matches = search_files(&args).unwrap();
        assert_eq!(matches.len(), 1);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn search_files_nonexistent_path() {
        let args = GrepArgs {
            pattern: "test".to_string(),
            path: "/nonexistent/path/12345".to_string(),
            refs: false,
            unique: false,
            ranked: false,
            limit: None,
            signature: false,
            glob: None,
            ignore_case: false,
            hidden: false,
        };

        let matches = search_files(&args).unwrap();
        assert!(matches.is_empty());
    }

    #[test]
    fn search_files_invalid_regex() {
        let args = GrepArgs {
            pattern: "[invalid".to_string(), // Invalid regex
            path: ".".to_string(),
            refs: false,
            unique: false,
            ranked: false,
            limit: None,
            signature: false,
            glob: None,
            ignore_case: false,
            hidden: false,
        };

        let result = search_files(&args);
        assert!(result.is_err());
    }

    #[test]
    fn search_files_case_insensitive() {
        let temp_dir = std::env::temp_dir().join("hu_grep_case_test");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        std::fs::write(temp_dir.join("test.txt"), "Hello HELLO hello\n").unwrap();

        // Case sensitive
        let args_sensitive = GrepArgs {
            pattern: "Hello".to_string(),
            path: temp_dir.to_str().unwrap().to_string(),
            refs: false,
            unique: false,
            ranked: false,
            limit: None,
            signature: false,
            glob: None,
            ignore_case: false,
            hidden: false,
        };

        let matches_sensitive = search_files(&args_sensitive).unwrap();
        assert_eq!(matches_sensitive[0].match_count, 1);

        // Case insensitive
        let args_insensitive = GrepArgs {
            pattern: "Hello".to_string(),
            path: temp_dir.to_str().unwrap().to_string(),
            refs: false,
            unique: false,
            ranked: false,
            limit: None,
            signature: false,
            glob: None,
            ignore_case: true,
            hidden: false,
        };

        let matches_insensitive = search_files(&args_insensitive).unwrap();
        assert_eq!(matches_insensitive[0].match_count, 3);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
