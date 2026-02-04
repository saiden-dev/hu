use serde::{Deserialize, Serialize};

// --- JSONL source types (read from Claude Code files) ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub display: Option<String>,
    pub timestamp: Option<f64>,
    pub project: Option<String>,
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEntry {
    pub uuid: Option<String>,
    #[serde(rename = "parentUuid")]
    pub parent_uuid: Option<String>,
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
    #[serde(rename = "type")]
    pub entry_type: Option<String>,
    pub timestamp: Option<String>,
    #[serde(rename = "gitBranch")]
    pub git_branch: Option<String>,
    pub cwd: Option<String>,
    pub message: Option<MessageBody>,
    #[serde(rename = "costUSD")]
    pub cost_usd: Option<f64>,
    #[serde(rename = "durationMs")]
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageBody {
    pub role: Option<String>,
    pub content: Option<MessageContent>,
    pub model: Option<String>,
    pub usage: Option<TokenUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub block_type: Option<String>,
    pub text: Option<String>,
    pub name: Option<String>,
    pub input: Option<serde_json::Value>,
    pub id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoEntry {
    pub content: Option<String>,
    pub status: Option<String>,
    #[serde(rename = "activeForm")]
    pub active_form: Option<String>,
}

// --- DB row types ---

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub project: String,
    pub display: Option<String>,
    pub started_at: i64,
    pub message_count: i64,
    pub total_cost_usd: f64,
    pub git_branch: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub session_id: String,
    pub parent_id: Option<String>,
    pub role: String,
    pub content: Option<String>,
    pub model: Option<String>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cost_usd: Option<f64>,
    pub duration_ms: Option<i64>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Todo {
    pub id: i64,
    pub session_id: String,
    pub content: String,
    pub status: String,
    pub active_form: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TodoWithProject {
    pub id: i64,
    pub session_id: String,
    pub content: String,
    pub status: String,
    pub active_form: Option<String>,
    pub project: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageStats {
    pub total_sessions: i64,
    pub total_messages: i64,
    pub total_cost: f64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelUsage {
    pub model: String,
    pub count: i64,
    pub cost: f64,
    pub input_tokens: i64,
    pub output_tokens: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolUsageStats {
    pub tool_name: String,
    pub count: i64,
    pub last_used: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolUsageDetail {
    pub tool_name: String,
    pub session_id: String,
    pub project: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BranchStats {
    pub git_branch: String,
    pub session_count: i64,
    pub session_ids: String,
    pub last_activity: i64,
    pub total_messages: i64,
    pub total_cost: f64,
    pub project: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: Option<String>,
    pub model: Option<String>,
    pub created_at: i64,
    pub project: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugError {
    pub file: String,
    pub line: usize,
    pub content: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncResult {
    pub history: usize,
    pub messages: usize,
    pub todos: usize,
}

#[derive(Debug, Clone, Default)]
pub enum OutputFormat {
    #[default]
    Table,
    Json,
}

// --- Helpers ---

impl MessageContent {
    pub fn as_string(&self) -> String {
        match self {
            MessageContent::Text(s) => s.clone(),
            MessageContent::Blocks(blocks) => serde_json::to_string(blocks).unwrap_or_default(),
        }
    }

    pub fn tool_use_blocks(&self) -> Vec<&ContentBlock> {
        match self {
            MessageContent::Text(_) => vec![],
            MessageContent::Blocks(blocks) => blocks
                .iter()
                .filter(|b| b.block_type.as_deref() == Some("tool_use"))
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_entry_serde_roundtrip() {
        let json = r#"{"display":"test","timestamp":1700000000000,"project":"/home/user","sessionId":"abc-123"}"#;
        let entry: HistoryEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.session_id.as_deref(), Some("abc-123"));
        assert_eq!(entry.display.as_deref(), Some("test"));
        let serialized = serde_json::to_string(&entry).unwrap();
        let re: HistoryEntry = serde_json::from_str(&serialized).unwrap();
        assert_eq!(re.session_id, entry.session_id);
    }

    #[test]
    fn history_entry_partial_fields() {
        let json = r#"{"display":"test"}"#;
        let entry: HistoryEntry = serde_json::from_str(json).unwrap();
        assert!(entry.session_id.is_none());
        assert!(entry.timestamp.is_none());
        assert!(entry.project.is_none());
    }

    #[test]
    fn message_entry_serde_roundtrip() {
        let json = r#"{
            "uuid": "msg-1",
            "parentUuid": "msg-0",
            "type": "user",
            "timestamp": "2024-01-01T00:00:00Z",
            "message": {
                "role": "user",
                "content": "hello",
                "model": null,
                "usage": null
            }
        }"#;
        let entry: MessageEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.uuid.as_deref(), Some("msg-1"));
        assert_eq!(entry.parent_uuid.as_deref(), Some("msg-0"));
        let body = entry.message.as_ref().unwrap();
        assert_eq!(body.role.as_deref(), Some("user"));
    }

    #[test]
    fn message_content_text() {
        let content = MessageContent::Text("hello world".to_string());
        assert_eq!(content.as_string(), "hello world");
        assert!(content.tool_use_blocks().is_empty());
    }

    #[test]
    fn message_content_blocks() {
        let blocks = vec![
            ContentBlock {
                block_type: Some("text".to_string()),
                text: Some("hi".to_string()),
                name: None,
                input: None,
                id: None,
            },
            ContentBlock {
                block_type: Some("tool_use".to_string()),
                text: None,
                name: Some("Read".to_string()),
                input: Some(serde_json::json!({"path": "/tmp/test"})),
                id: Some("tu-1".to_string()),
            },
        ];
        let content = MessageContent::Blocks(blocks);
        let s = content.as_string();
        assert!(s.contains("text"));
        assert!(s.contains("tool_use"));
        let tool_blocks = content.tool_use_blocks();
        assert_eq!(tool_blocks.len(), 1);
        assert_eq!(tool_blocks[0].name.as_deref(), Some("Read"));
    }

    #[test]
    fn message_content_blocks_serde() {
        let json = r#"[{"type":"text","text":"hello"},{"type":"tool_use","name":"Bash","input":{"cmd":"ls"}}]"#;
        let content: MessageContent = serde_json::from_str(json).unwrap();
        assert_eq!(content.tool_use_blocks().len(), 1);
    }

    #[test]
    fn message_content_text_serde() {
        let json = r#""just a string""#;
        let content: MessageContent = serde_json::from_str(json).unwrap();
        assert_eq!(content.as_string(), "just a string");
    }

    #[test]
    fn todo_entry_serde() {
        let json = r#"{"content":"Fix bug","status":"pending","activeForm":"Fixing bug"}"#;
        let entry: TodoEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.content.as_deref(), Some("Fix bug"));
        assert_eq!(entry.status.as_deref(), Some("pending"));
        assert_eq!(entry.active_form.as_deref(), Some("Fixing bug"));
    }

    #[test]
    fn todo_entry_minimal() {
        let json = r#"{}"#;
        let entry: TodoEntry = serde_json::from_str(json).unwrap();
        assert!(entry.content.is_none());
    }

    #[test]
    fn token_usage_serde() {
        let json = r#"{"input_tokens":100,"output_tokens":200}"#;
        let usage: TokenUsage = serde_json::from_str(json).unwrap();
        assert_eq!(usage.input_tokens, Some(100));
        assert_eq!(usage.output_tokens, Some(200));
    }

    #[test]
    fn session_default() {
        let s = Session::default();
        assert_eq!(s.id, "");
        assert_eq!(s.message_count, 0);
        assert_eq!(s.total_cost_usd, 0.0);
        assert!(s.git_branch.is_none());
    }

    #[test]
    fn usage_stats_default() {
        let s = UsageStats::default();
        assert_eq!(s.total_sessions, 0);
        assert_eq!(s.total_messages, 0);
        assert_eq!(s.total_cost, 0.0);
    }

    #[test]
    fn sync_result_default() {
        let r = SyncResult::default();
        assert_eq!(r.history, 0);
        assert_eq!(r.messages, 0);
        assert_eq!(r.todos, 0);
    }

    #[test]
    fn output_format_default() {
        let f = OutputFormat::default();
        assert!(matches!(f, OutputFormat::Table));
    }

    #[test]
    fn debug_error_fields() {
        let e = DebugError {
            file: "test.log".to_string(),
            line: 42,
            content: "error: something failed".to_string(),
            timestamp: 1700000000,
        };
        let json = serde_json::to_string(&e).unwrap();
        let re: DebugError = serde_json::from_str(&json).unwrap();
        assert_eq!(re.file, "test.log");
        assert_eq!(re.line, 42);
    }

    #[test]
    fn branch_stats_default() {
        let b = BranchStats::default();
        assert_eq!(b.git_branch, "");
        assert_eq!(b.session_count, 0);
    }

    #[test]
    fn tool_usage_stats_default() {
        let t = ToolUsageStats::default();
        assert_eq!(t.tool_name, "");
        assert_eq!(t.count, 0);
    }

    #[test]
    fn search_result_serialize() {
        let sr = SearchResult {
            id: "msg-1".to_string(),
            session_id: "sess-1".to_string(),
            role: "user".to_string(),
            content: Some("hello".to_string()),
            model: None,
            created_at: 1700000000,
            project: "/home/user/proj".to_string(),
        };
        let json = serde_json::to_string(&sr).unwrap();
        assert!(json.contains("msg-1"));
    }

    #[test]
    fn message_entry_with_git_branch() {
        let json = r#"{
            "uuid": "msg-1",
            "type": "user",
            "gitBranch": "feature/test",
            "timestamp": "2024-01-01T00:00:00Z"
        }"#;
        let entry: MessageEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.git_branch.as_deref(), Some("feature/test"));
    }

    #[test]
    fn message_entry_with_cost() {
        let json = r#"{
            "uuid": "msg-1",
            "type": "assistant",
            "costUSD": 0.0123,
            "durationMs": 500
        }"#;
        let entry: MessageEntry = serde_json::from_str(json).unwrap();
        assert!((entry.cost_usd.unwrap() - 0.0123).abs() < f64::EPSILON);
        assert_eq!(entry.duration_ms, Some(500));
    }

    #[test]
    fn content_block_serde() {
        let json = r#"{"type":"tool_use","name":"Read","input":{"path":"/tmp"},"id":"tu-1"}"#;
        let block: ContentBlock = serde_json::from_str(json).unwrap();
        assert_eq!(block.block_type.as_deref(), Some("tool_use"));
        assert_eq!(block.name.as_deref(), Some("Read"));
        assert_eq!(block.id.as_deref(), Some("tu-1"));
    }
}
