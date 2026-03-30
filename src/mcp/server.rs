use serde_json::json;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use super::handlers;
use super::tools;
use super::types::{JsonRpcRequest, JsonRpcResponse, ERR_INTERNAL, ERR_METHOD_NOT_FOUND};

/// Server info constants.
const PROTOCOL_VERSION: &str = "2024-11-05";
const SERVER_NAME: &str = "hu";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Run the MCP JSON-RPC server reading from stdin, writing to stdout.
#[cfg(not(tarpaulin_include))]
pub async fn run() -> anyhow::Result<()> {
    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let err_resp =
                    JsonRpcResponse::error(json!(null), -32700, format!("Parse error: {e}"));
                write_response(&mut stdout, &err_resp).await?;
                continue;
            }
        };

        // Notifications have no id — skip them (no response expected)
        if request.id.is_none() {
            continue;
        }

        let response = dispatch(&request).await;
        write_response(&mut stdout, &response).await?;
    }

    Ok(())
}

/// Write a JSON-RPC response as a single line to stdout.
#[cfg(not(tarpaulin_include))]
async fn write_response(
    stdout: &mut tokio::io::Stdout,
    response: &JsonRpcResponse,
) -> anyhow::Result<()> {
    let json = serde_json::to_string(response)?;
    stdout.write_all(json.as_bytes()).await?;
    stdout.write_all(b"\n").await?;
    stdout.flush().await?;
    Ok(())
}

/// Dispatch a JSON-RPC request to the appropriate handler.
pub async fn dispatch(req: &JsonRpcRequest) -> JsonRpcResponse {
    let id = req.id.clone().unwrap_or(json!(null));

    match req.method.as_str() {
        "initialize" => handle_initialize(id),
        "tools/list" => handle_tools_list(id),
        "tools/call" => handle_tools_call(id, &req.params).await,
        _ => JsonRpcResponse::error(id, ERR_METHOD_NOT_FOUND, "Method not found"),
    }
}

/// Handle `initialize` — return server capabilities.
fn handle_initialize(id: serde_json::Value) -> JsonRpcResponse {
    JsonRpcResponse::success(
        id,
        json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": SERVER_NAME,
                "version": SERVER_VERSION,
            }
        }),
    )
}

/// Handle `tools/list` — return all registered tools.
fn handle_tools_list(id: serde_json::Value) -> JsonRpcResponse {
    let tool_defs = tools::all_tools();
    match serde_json::to_value(&tool_defs) {
        Ok(tools_json) => JsonRpcResponse::success(id, json!({ "tools": tools_json })),
        Err(e) => {
            JsonRpcResponse::error(id, ERR_INTERNAL, format!("Failed to serialize tools: {e}"))
        }
    }
}

/// Handle `tools/call` — dispatch to the named tool handler.
#[cfg(not(tarpaulin_include))]
async fn handle_tools_call(id: serde_json::Value, params: &serde_json::Value) -> JsonRpcResponse {
    let (name, args) = handlers::extract_tool_call(params);
    if name.is_empty() {
        return JsonRpcResponse::error(id, ERR_INTERNAL, "Missing tool name in params");
    }

    let result = handlers::handle_tool_call(&name, args).await;
    match serde_json::to_value(&result) {
        Ok(val) => JsonRpcResponse::success(id, val),
        Err(e) => JsonRpcResponse::error(
            id,
            ERR_INTERNAL,
            format!("Failed to serialize tool result: {e}"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const EXPECTED_TOOL_COUNT: usize = 6;

    // --- Constants ---

    #[test]
    fn protocol_version_is_set() {
        assert_eq!(PROTOCOL_VERSION, "2024-11-05");
    }

    #[test]
    fn server_name_is_hu() {
        assert_eq!(SERVER_NAME, "hu");
    }

    #[test]
    fn server_version_matches_cargo() {
        assert!(!SERVER_VERSION.is_empty());
    }

    // --- handle_initialize ---

    #[test]
    fn initialize_response_has_protocol_version() {
        let resp = handle_initialize(json!(1));
        let result = resp.result.unwrap();
        assert_eq!(result["protocolVersion"], PROTOCOL_VERSION);
    }

    #[test]
    fn initialize_response_has_capabilities() {
        let resp = handle_initialize(json!(1));
        let result = resp.result.unwrap();
        assert!(result["capabilities"]["tools"].is_object());
    }

    #[test]
    fn initialize_response_has_server_info() {
        let resp = handle_initialize(json!(1));
        let result = resp.result.unwrap();
        assert_eq!(result["serverInfo"]["name"], SERVER_NAME);
        assert_eq!(result["serverInfo"]["version"], SERVER_VERSION);
    }

    #[test]
    fn initialize_preserves_request_id() {
        let resp = handle_initialize(json!("abc"));
        assert_eq!(resp.id, json!("abc"));
    }

    // --- handle_tools_list ---

    #[test]
    fn tools_list_returns_all_tools() {
        let resp = handle_tools_list(json!(2));
        let result = resp.result.unwrap();
        let tools = result["tools"].as_array().unwrap();
        assert_eq!(tools.len(), EXPECTED_TOOL_COUNT);
    }

    #[test]
    fn tools_list_tools_have_required_fields() {
        let resp = handle_tools_list(json!(1));
        let result = resp.result.unwrap();
        for tool in result["tools"].as_array().unwrap() {
            assert!(tool.get("name").is_some(), "tool missing name");
            assert!(
                tool.get("description").is_some(),
                "tool missing description"
            );
            assert!(
                tool.get("inputSchema").is_some(),
                "tool missing inputSchema"
            );
        }
    }

    #[test]
    fn tools_list_preserves_request_id() {
        let resp = handle_tools_list(json!(99));
        assert_eq!(resp.id, json!(99));
    }

    // --- dispatch ---

    #[tokio::test]
    async fn dispatch_initialize() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: json!({}),
        };
        let resp = dispatch(&req).await;
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[tokio::test]
    async fn dispatch_tools_list() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(2)),
            method: "tools/list".to_string(),
            params: json!({}),
        };
        let resp = dispatch(&req).await;
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[tokio::test]
    async fn dispatch_unknown_method() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(3)),
            method: "unknown/method".to_string(),
            params: json!({}),
        };
        let resp = dispatch(&req).await;
        assert!(resp.error.is_some());
        let err = resp.error.unwrap();
        assert_eq!(err.code, ERR_METHOD_NOT_FOUND);
        assert_eq!(err.message, "Method not found");
    }

    #[tokio::test]
    async fn dispatch_notification_still_dispatches() {
        // dispatch() is called after the server loop filters notifications,
        // but if called directly with no id, it should still work
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: "initialize".to_string(),
            params: json!({}),
        };
        let resp = dispatch(&req).await;
        // id defaults to null for notifications
        assert_eq!(resp.id, json!(null));
        assert!(resp.result.is_some());
    }

    #[tokio::test]
    async fn dispatch_preserves_string_id() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!("request-abc")),
            method: "initialize".to_string(),
            params: json!({}),
        };
        let resp = dispatch(&req).await;
        assert_eq!(resp.id, json!("request-abc"));
    }

    // --- Full response serialization ---

    #[tokio::test]
    async fn initialize_response_is_valid_json() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: json!({}),
        };
        let resp = dispatch(&req).await;
        let json_str = serde_json::to_string(&resp).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["id"], 1);
    }

    #[tokio::test]
    async fn tools_list_response_is_valid_json() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(2)),
            method: "tools/list".to_string(),
            params: json!({}),
        };
        let resp = dispatch(&req).await;
        let json_str = serde_json::to_string(&resp).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert!(parsed["result"]["tools"].is_array());
    }
}
