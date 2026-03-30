use anyhow::Result;

use super::types::ToolResult;
use crate::data;
use crate::read;

/// Dispatch a tool call to the appropriate handler.
///
/// Each handler opens its own DB/resources, calls the service layer,
/// and serializes the result to a `ToolResult`.
#[cfg(not(tarpaulin_include))]
pub async fn handle_tool_call(name: &str, args: serde_json::Value) -> ToolResult {
    match handle_tool_inner(name, &args).await {
        Ok(result) => result,
        Err(e) => ToolResult::error(format!("{e:#}")),
    }
}

#[cfg(not(tarpaulin_include))]
async fn handle_tool_inner(name: &str, args: &serde_json::Value) -> Result<ToolResult> {
    match name {
        "data_stats" => handle_data_stats(args),
        "data_search" => handle_data_search(args),
        "data_sessions" => handle_data_sessions(args),
        "data_errors" => handle_data_errors(args),
        "data_pricing" => handle_data_pricing(args),
        "data_tools" => handle_data_tools(args),
        "read_file" => handle_read_file(args),
        _ => Ok(ToolResult::error(format!("Unknown tool: {name}"))),
    }
}

// --- Data handlers ---

#[cfg(not(tarpaulin_include))]
fn handle_data_stats(args: &serde_json::Value) -> Result<ToolResult> {
    let today = args.get("today").and_then(|v| v.as_bool()).unwrap_or(false);
    let store = data::service::open_db()?;
    data::service::ensure_synced(&store)?;
    let (stats, model_usage) = data::service::get_stats(&store, today)?;
    let json = serde_json::to_string_pretty(&serde_json::json!({
        "stats": stats,
        "model_usage": model_usage,
    }))?;
    Ok(ToolResult::text(json))
}

#[cfg(not(tarpaulin_include))]
fn handle_data_search(args: &serde_json::Value) -> Result<ToolResult> {
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    if query.is_empty() {
        return Ok(ToolResult::error("Missing required parameter: query"));
    }
    let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(20);
    let store = data::service::open_db()?;
    data::service::ensure_synced(&store)?;
    let results = data::service::search_messages(&store, query, limit)?;
    let json = serde_json::to_string_pretty(&results)?;
    Ok(ToolResult::text(json))
}

#[cfg(not(tarpaulin_include))]
fn handle_data_sessions(args: &serde_json::Value) -> Result<ToolResult> {
    let project = args.get("project").and_then(|v| v.as_str());
    let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(20);
    let store = data::service::open_db()?;
    data::service::ensure_synced(&store)?;
    let sessions = data::service::get_sessions(&store, project, limit)?;
    let json = serde_json::to_string_pretty(&sessions)?;
    Ok(ToolResult::text(json))
}

#[cfg(not(tarpaulin_include))]
fn handle_data_errors(args: &serde_json::Value) -> Result<ToolResult> {
    let recent_days = args
        .get("recent_days")
        .and_then(|v| v.as_u64())
        .unwrap_or(7) as u32;
    let cfg = data::service::get_config()?;
    let errors = data::service::scan_debug_errors(&cfg.claude_dir, recent_days)?;
    let json = serde_json::to_string_pretty(&errors)?;
    Ok(ToolResult::text(json))
}

#[cfg(not(tarpaulin_include))]
fn handle_data_pricing(args: &serde_json::Value) -> Result<ToolResult> {
    let subscription = args
        .get("subscription")
        .and_then(|v| v.as_str())
        .unwrap_or("max5x");
    let billing_day = args
        .get("billing_day")
        .and_then(|v| v.as_u64())
        .unwrap_or(1) as u32;
    let store = data::service::open_db()?;
    data::service::ensure_synced(&store)?;
    let pricing = data::service::compute_pricing(&store, subscription, billing_day)?;
    let json = serde_json::to_string_pretty(&pricing)?;
    Ok(ToolResult::text(json))
}

#[cfg(not(tarpaulin_include))]
fn handle_data_tools(args: &serde_json::Value) -> Result<ToolResult> {
    let tool = args.get("tool").and_then(|v| v.as_str());
    let store = data::service::open_db()?;
    data::service::ensure_synced(&store)?;

    let json = match tool {
        Some(name) => {
            let detail = data::service::get_tool_detail(&store, name)?;
            serde_json::to_string_pretty(&detail)?
        }
        None => {
            let stats = data::service::get_tool_stats(&store)?;
            serde_json::to_string_pretty(&stats)?
        }
    };
    Ok(ToolResult::text(json))
}

// --- Read handler ---

#[cfg(not(tarpaulin_include))]
fn handle_read_file(args: &serde_json::Value) -> Result<ToolResult> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    if path.is_empty() {
        return Ok(ToolResult::error("Missing required parameter: path"));
    }

    let read_args = read::ReadArgs {
        path: path.to_string(),
        outline: args
            .get("outline")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        interface: args
            .get("interface")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        around: args
            .get("around")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize),
        context: args.get("context").and_then(|v| v.as_u64()).unwrap_or(10) as usize,
        diff: args.get("diff").and_then(|v| v.as_bool()).unwrap_or(false),
        commit: args
            .get("commit")
            .and_then(|v| v.as_str())
            .unwrap_or("HEAD")
            .to_string(),
    };

    let output = read::read(read_args)?;
    let text = format!("{output:?}");
    Ok(ToolResult::text(text))
}

/// Extract a tool name from `tools/call` params.
///
/// MCP protocol sends `{"name": "tool_name", "arguments": {...}}`.
pub fn extract_tool_call(params: &serde_json::Value) -> (String, serde_json::Value) {
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or(serde_json::json!({}));
    (name, args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_tool_call_full() {
        let params = serde_json::json!({
            "name": "data_stats",
            "arguments": {"today": true}
        });
        let (name, args) = extract_tool_call(&params);
        assert_eq!(name, "data_stats");
        assert_eq!(args["today"], true);
    }

    #[test]
    fn extract_tool_call_no_arguments() {
        let params = serde_json::json!({"name": "data_stats"});
        let (name, args) = extract_tool_call(&params);
        assert_eq!(name, "data_stats");
        assert!(args.is_object());
        assert!(args.as_object().unwrap().is_empty());
    }

    #[test]
    fn extract_tool_call_empty_params() {
        let params = serde_json::json!({});
        let (name, args) = extract_tool_call(&params);
        assert!(name.is_empty());
        assert!(args.is_object());
    }

    #[test]
    fn extract_tool_call_null_params() {
        let params = serde_json::json!(null);
        let (name, args) = extract_tool_call(&params);
        assert!(name.is_empty());
        assert!(args.is_object());
    }

    #[test]
    fn extract_tool_call_with_nested_args() {
        let params = serde_json::json!({
            "name": "read_file",
            "arguments": {
                "path": "/tmp/test.rs",
                "outline": true,
                "around": 42
            }
        });
        let (name, args) = extract_tool_call(&params);
        assert_eq!(name, "read_file");
        assert_eq!(args["path"], "/tmp/test.rs");
        assert_eq!(args["outline"], true);
        assert_eq!(args["around"], 42);
    }
}
