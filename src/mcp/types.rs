use serde::{Deserialize, Serialize};

/// JSON-RPC 2.0 request (incoming from MCP client).
///
/// When `id` is `None`, this is a notification and needs no response.
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    #[allow(dead_code)]
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

/// JSON-RPC 2.0 response (outgoing to MCP client).
#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 error object.
#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

/// MCP tool definition returned by `tools/list`.
#[derive(Debug, Clone, Serialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

/// Result payload for `tools/call` responses.
#[derive(Debug, Serialize)]
pub struct ToolResult {
    pub content: Vec<ToolContent>,
    #[serde(rename = "isError", skip_serializing_if = "is_false")]
    pub is_error: bool,
}

/// A single content item in a tool result.
#[derive(Debug, Serialize)]
pub struct ToolContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

fn is_false(v: &bool) -> bool {
    !(*v)
}

// --- Constructors ---

impl JsonRpcResponse {
    /// Build a success response with a JSON result.
    pub fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Build an error response.
    pub fn error(id: serde_json::Value, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
            }),
        }
    }
}

impl ToolResult {
    /// Build a successful tool result with text content.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: text.into(),
            }],
            is_error: false,
        }
    }

    /// Build an error tool result.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: message.into(),
            }],
            is_error: true,
        }
    }
}

// --- JSON-RPC error codes ---

/// Standard JSON-RPC error: method not found.
pub const ERR_METHOD_NOT_FOUND: i32 = -32601;

/// Standard JSON-RPC error: invalid params.
#[allow(dead_code)]
pub const ERR_INVALID_PARAMS: i32 = -32602;

/// Standard JSON-RPC error: internal error.
pub const ERR_INTERNAL: i32 = -32603;

#[cfg(test)]
mod tests {
    use super::*;

    // --- JsonRpcRequest deserialization ---

    #[test]
    fn deserialize_request_with_id() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.id, Some(serde_json::json!(1)));
        assert_eq!(req.method, "tools/list");
    }

    #[test]
    fn deserialize_request_string_id() {
        let json = r#"{"jsonrpc":"2.0","id":"abc","method":"initialize","params":{}}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.id, Some(serde_json::json!("abc")));
    }

    #[test]
    fn deserialize_notification_no_id() {
        let json = r#"{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert!(req.id.is_none());
        assert_eq!(req.method, "notifications/initialized");
    }

    #[test]
    fn deserialize_request_no_params() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert!(req.params.is_null());
    }

    #[test]
    fn deserialize_request_with_nested_params() {
        let json = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"data_stats","arguments":{"today":true}}}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.params["name"], "data_stats");
        assert_eq!(req.params["arguments"]["today"], true);
    }

    #[test]
    fn request_debug_format() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"test"}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        let debug = format!("{:?}", req);
        assert!(debug.contains("JsonRpcRequest"));
        assert!(debug.contains("test"));
    }

    // --- JsonRpcResponse serialization ---

    #[test]
    fn serialize_success_response() {
        let resp = JsonRpcResponse::success(serde_json::json!(1), serde_json::json!({"ok": true}));
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains(r#""jsonrpc":"2.0""#));
        assert!(json.contains(r#""id":1"#));
        assert!(json.contains(r#""result":{"ok":true}"#));
        assert!(!json.contains("error"));
    }

    #[test]
    fn serialize_error_response() {
        let resp = JsonRpcResponse::error(
            serde_json::json!(2),
            ERR_METHOD_NOT_FOUND,
            "Method not found",
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains(r#""id":2"#));
        assert!(json.contains(r#""code":-32601"#));
        assert!(json.contains(r#""message":"Method not found""#));
        assert!(!json.contains("result"));
    }

    #[test]
    fn serialize_success_skips_error() {
        let resp = JsonRpcResponse::success(serde_json::json!(1), serde_json::json!("ok"));
        let json = serde_json::to_string(&resp).unwrap();
        assert!(!json.contains("error"));
    }

    #[test]
    fn serialize_error_skips_result() {
        let resp = JsonRpcResponse::error(serde_json::json!(1), -1, "fail");
        let json = serde_json::to_string(&resp).unwrap();
        assert!(!json.contains("result"));
    }

    #[test]
    fn response_debug_format() {
        let resp = JsonRpcResponse::success(serde_json::json!(1), serde_json::json!(null));
        let debug = format!("{:?}", resp);
        assert!(debug.contains("JsonRpcResponse"));
    }

    // --- JsonRpcError ---

    #[test]
    fn error_serialize() {
        let err = JsonRpcError {
            code: -32600,
            message: "Invalid request".to_string(),
        };
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("-32600"));
        assert!(json.contains("Invalid request"));
    }

    #[test]
    fn error_debug_format() {
        let err = JsonRpcError {
            code: -1,
            message: "test".to_string(),
        };
        let debug = format!("{:?}", err);
        assert!(debug.contains("JsonRpcError"));
    }

    // --- ToolDef ---

    #[test]
    fn tool_def_serialize() {
        let tool = ToolDef {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
            }),
        };
        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("test_tool"));
        assert!(json.contains("A test tool"));
        assert!(json.contains("inputSchema"));
        // Verify camelCase rename
        assert!(!json.contains("input_schema"));
    }

    #[test]
    fn tool_def_clone() {
        let tool = ToolDef {
            name: "t".to_string(),
            description: "d".to_string(),
            input_schema: serde_json::json!({}),
        };
        let cloned = tool.clone();
        assert_eq!(cloned.name, "t");
    }

    #[test]
    fn tool_def_debug_format() {
        let tool = ToolDef {
            name: "t".to_string(),
            description: "d".to_string(),
            input_schema: serde_json::json!({}),
        };
        let debug = format!("{:?}", tool);
        assert!(debug.contains("ToolDef"));
    }

    // --- ToolResult ---

    #[test]
    fn tool_result_text() {
        let result = ToolResult::text("hello world");
        assert!(!result.is_error);
        assert_eq!(result.content.len(), 1);
        assert_eq!(result.content[0].content_type, "text");
        assert_eq!(result.content[0].text, "hello world");
    }

    #[test]
    fn tool_result_text_serialize_no_is_error() {
        let result = ToolResult::text("ok");
        let json = serde_json::to_string(&result).unwrap();
        // is_error=false should be skipped
        assert!(!json.contains("isError"));
    }

    #[test]
    fn tool_result_error() {
        let result = ToolResult::error("something broke");
        assert!(result.is_error);
        assert_eq!(result.content[0].text, "something broke");
    }

    #[test]
    fn tool_result_error_serialize_has_is_error() {
        let result = ToolResult::error("fail");
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains(r#""isError":true"#));
    }

    #[test]
    fn tool_result_debug_format() {
        let result = ToolResult::text("test");
        let debug = format!("{:?}", result);
        assert!(debug.contains("ToolResult"));
    }

    // --- ToolContent ---

    #[test]
    fn tool_content_serialize() {
        let content = ToolContent {
            content_type: "text".to_string(),
            text: "hello".to_string(),
        };
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains(r#""type":"text""#));
        assert!(json.contains(r#""text":"hello""#));
        // Verify rename
        assert!(!json.contains("content_type"));
    }

    #[test]
    fn tool_content_debug_format() {
        let content = ToolContent {
            content_type: "text".to_string(),
            text: "test".to_string(),
        };
        let debug = format!("{:?}", content);
        assert!(debug.contains("ToolContent"));
    }

    // --- is_false helper ---

    #[test]
    fn is_false_returns_true_for_false() {
        assert!(is_false(&false));
    }

    #[test]
    fn is_false_returns_false_for_true() {
        assert!(!is_false(&true));
    }

    // --- Error code constants ---

    #[test]
    fn error_codes() {
        assert_eq!(ERR_METHOD_NOT_FOUND, -32601);
        assert_eq!(ERR_INVALID_PARAMS, -32602);
        assert_eq!(ERR_INTERNAL, -32603);
    }

    // --- Round-trip: request -> dispatch -> response ---

    #[test]
    fn response_roundtrip_success() {
        let resp =
            JsonRpcResponse::success(serde_json::json!(42), serde_json::json!({"tools": []}));
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["id"], 42);
        assert!(parsed["result"]["tools"].is_array());
        assert!(parsed.get("error").is_none());
    }

    #[test]
    fn response_roundtrip_error() {
        let resp = JsonRpcResponse::error(
            serde_json::json!("req-1"),
            ERR_INTERNAL,
            "DB connection failed",
        );
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["id"], "req-1");
        assert_eq!(parsed["error"]["code"], ERR_INTERNAL);
        assert_eq!(parsed["error"]["message"], "DB connection failed");
        assert!(parsed.get("result").is_none());
    }
}
