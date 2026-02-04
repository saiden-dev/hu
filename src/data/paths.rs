use anyhow::Result;
use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};

#[allow(dead_code)]
pub fn encode_project_path(path: &str) -> String {
    // Order matters: replace "/." first (double dash), then "/" (single dash)
    path.replace("/.", "--").replace('/', "-")
}

pub fn decode_project_path(encoded: &str) -> String {
    // Order matters: replace "--" first (was "/."), then "-" (was "/")
    encoded.replace("--", "/.").replace('-', "/")
}

pub fn history_path(claude_dir: &Path) -> PathBuf {
    claude_dir.join("history.jsonl")
}

pub fn projects_dir(claude_dir: &Path) -> PathBuf {
    claude_dir.join("projects")
}

pub fn todos_dir(claude_dir: &Path) -> PathBuf {
    claude_dir.join("todos")
}

pub fn debug_dir(claude_dir: &Path) -> PathBuf {
    claude_dir.join("debug")
}

pub fn parse_jsonl<T: DeserializeOwned>(content: &str) -> Vec<T> {
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

pub fn list_project_dirs(claude_dir: &Path) -> Result<Vec<ProjectDir>> {
    let dir = projects_dir(claude_dir);
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut projects = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            projects.push(ProjectDir {
                path: decode_project_path(&name),
                encoded: name,
                dir: entry.path(),
            });
        }
    }
    projects.sort_by(|a, b| a.encoded.cmp(&b.encoded));
    Ok(projects)
}

#[derive(Debug, Clone)]
pub struct ProjectDir {
    pub encoded: String,
    pub path: String,
    pub dir: PathBuf,
}

pub fn list_session_files(project_dir: &Path) -> Result<Vec<SessionFile>> {
    if !project_dir.exists() {
        return Ok(vec![]);
    }
    let mut sessions = Vec::new();
    for entry in std::fs::read_dir(project_dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".jsonl") {
            let session_id = name.trim_end_matches(".jsonl").to_string();
            sessions.push(SessionFile {
                session_id,
                path: entry.path(),
            });
        }
    }
    sessions.sort_by(|a, b| a.session_id.cmp(&b.session_id));
    Ok(sessions)
}

#[derive(Debug, Clone)]
pub struct SessionFile {
    pub session_id: String,
    pub path: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_basic_path() {
        assert_eq!(
            encode_project_path("/Users/chi/project"),
            "-Users-chi-project"
        );
    }

    #[test]
    fn encode_dotfile_path() {
        assert_eq!(
            encode_project_path("/Users/chi/.claude"),
            "-Users-chi--claude"
        );
    }

    #[test]
    fn decode_basic_path() {
        assert_eq!(
            decode_project_path("-Users-chi-project"),
            "/Users/chi/project"
        );
    }

    #[test]
    fn decode_dotfile_path() {
        assert_eq!(
            decode_project_path("-Users-chi--claude"),
            "/Users/chi/.claude"
        );
    }

    #[test]
    fn encode_decode_roundtrip() {
        let paths = vec![
            "/Users/chi/Projects/hu",
            "/Users/chi/.claude",
            "/home/user/.config/test",
            "/tmp/a",
        ];
        for path in paths {
            let encoded = encode_project_path(path);
            let decoded = decode_project_path(&encoded);
            assert_eq!(decoded, path, "roundtrip failed for {path}");
        }
    }

    #[test]
    fn encode_root() {
        assert_eq!(encode_project_path("/"), "-");
    }

    #[test]
    fn decode_single_dash() {
        assert_eq!(decode_project_path("-"), "/");
    }

    #[test]
    fn history_path_construction() {
        let p = history_path(Path::new("/home/user/.claude"));
        assert_eq!(p, PathBuf::from("/home/user/.claude/history.jsonl"));
    }

    #[test]
    fn projects_dir_construction() {
        let p = projects_dir(Path::new("/home/user/.claude"));
        assert_eq!(p, PathBuf::from("/home/user/.claude/projects"));
    }

    #[test]
    fn todos_dir_construction() {
        let p = todos_dir(Path::new("/home/user/.claude"));
        assert_eq!(p, PathBuf::from("/home/user/.claude/todos"));
    }

    #[test]
    fn debug_dir_construction() {
        let p = debug_dir(Path::new("/home/user/.claude"));
        assert_eq!(p, PathBuf::from("/home/user/.claude/debug"));
    }

    #[test]
    fn parse_jsonl_valid() {
        let content = r#"{"name":"a","value":1}
{"name":"b","value":2}
"#;
        #[derive(serde::Deserialize)]
        struct Item {
            name: String,
            value: i32,
        }
        let items: Vec<Item> = parse_jsonl(content);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "a");
        assert_eq!(items[1].value, 2);
    }

    #[test]
    fn parse_jsonl_skip_malformed() {
        let content = r#"{"valid":true}
not json at all
{"also_valid":true}
"#;
        #[derive(serde::Deserialize)]
        #[allow(dead_code)]
        struct Item {
            valid: Option<bool>,
            also_valid: Option<bool>,
        }
        let items: Vec<Item> = parse_jsonl(content);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn parse_jsonl_empty() {
        let items: Vec<serde_json::Value> = parse_jsonl("");
        assert!(items.is_empty());
    }

    #[test]
    fn parse_jsonl_blank_lines() {
        let content = "\n\n{\"x\":1}\n\n{\"x\":2}\n\n";
        #[derive(serde::Deserialize)]
        #[allow(dead_code)]
        struct Item {
            x: i32,
        }
        let items: Vec<Item> = parse_jsonl(content);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn list_project_dirs_missing() {
        let dirs = list_project_dirs(Path::new("/nonexistent/path")).unwrap();
        assert!(dirs.is_empty());
    }

    #[test]
    fn list_session_files_missing() {
        let files = list_session_files(Path::new("/nonexistent/path")).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn list_project_dirs_real() {
        let tmp = std::env::temp_dir().join("hu-test-projects");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("projects").join("-Users-chi-proj")).unwrap();
        std::fs::create_dir_all(tmp.join("projects").join("-Users-chi--hidden")).unwrap();

        let dirs = list_project_dirs(&tmp).unwrap();
        assert_eq!(dirs.len(), 2);
        // Sorted by encoded name
        assert_eq!(dirs[0].encoded, "-Users-chi--hidden");
        assert_eq!(dirs[0].path, "/Users/chi/.hidden");
        assert_eq!(dirs[1].encoded, "-Users-chi-proj");
        assert_eq!(dirs[1].path, "/Users/chi/proj");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn list_session_files_real() {
        let tmp = std::env::temp_dir().join("hu-test-sessions");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("abc-123.jsonl"), "").unwrap();
        std::fs::write(tmp.join("def-456.jsonl"), "").unwrap();
        std::fs::write(tmp.join("notes.txt"), "").unwrap();

        let files = list_session_files(&tmp).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].session_id, "abc-123");
        assert_eq!(files[1].session_id, "def-456");

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
