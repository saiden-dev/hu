use serde_json::json;

use super::types::ToolDef;

/// Return all MCP tool definitions.
pub fn all_tools() -> Vec<ToolDef> {
    vec![
        data_stats(),
        data_search(),
        data_sessions(),
        data_errors(),
        data_tools(),
        read_file(),
    ]
}

fn data_stats() -> ToolDef {
    ToolDef {
        name: "data_stats".to_string(),
        description: "Get Claude Code usage statistics (sessions, messages, cost, tokens)"
            .to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "today": {
                    "type": "boolean",
                    "description": "If true, only show stats from today. Default: false"
                }
            }
        }),
    }
}

fn data_search() -> ToolDef {
    ToolDef {
        name: "data_search".to_string(),
        description: "Search through Claude Code session messages".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query string"
                },
                "limit": {
                    "type": "integer",
                    "description": "Max results to return (default: 20)"
                }
            },
            "required": ["query"]
        }),
    }
}

fn data_sessions() -> ToolDef {
    ToolDef {
        name: "data_sessions".to_string(),
        description: "List Claude Code sessions, optionally filtered by project".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "project": {
                    "type": "string",
                    "description": "Filter sessions by project path substring"
                },
                "limit": {
                    "type": "integer",
                    "description": "Max sessions to return (default: 20)"
                }
            }
        }),
    }
}

fn data_errors() -> ToolDef {
    ToolDef {
        name: "data_errors".to_string(),
        description: "Scan Claude Code debug logs for errors".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "recent_days": {
                    "type": "integer",
                    "description": "Only scan files modified within this many days (default: 7)"
                }
            }
        }),
    }
}

fn data_tools() -> ToolDef {
    ToolDef {
        name: "data_tools".to_string(),
        description: "Get tool usage statistics across Claude Code sessions".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "tool": {
                    "type": "string",
                    "description": "Filter by specific tool name (e.g., Read, Edit, Bash)"
                }
            }
        }),
    }
}

fn read_file() -> ToolDef {
    ToolDef {
        name: "read_file".to_string(),
        description: "Smart file reading with outline, interface, around-line, and diff modes"
            .to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "File path to read"
                },
                "outline": {
                    "type": "boolean",
                    "description": "Show file outline (functions, structs, classes)"
                },
                "interface": {
                    "type": "boolean",
                    "description": "Show public interface only"
                },
                "around": {
                    "type": "integer",
                    "description": "Show lines around a specific line number"
                },
                "context": {
                    "type": "integer",
                    "description": "Number of context lines for --around (default: 10)"
                },
                "diff": {
                    "type": "boolean",
                    "description": "Show git diff"
                },
                "commit": {
                    "type": "string",
                    "description": "Commit to diff against (default: HEAD)"
                }
            },
            "required": ["path"]
        }),
    }
}

/// Number of tools expected in the registry (for tests).
#[cfg(test)]
pub const EXPECTED_TOOL_COUNT: usize = 6;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_tools_returns_expected_count() {
        let tools = all_tools();
        assert_eq!(tools.len(), EXPECTED_TOOL_COUNT);
    }

    #[test]
    fn all_tool_names_are_unique() {
        let tools = all_tools();
        let mut names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), EXPECTED_TOOL_COUNT);
    }

    #[test]
    fn all_tools_have_descriptions() {
        for tool in all_tools() {
            assert!(
                !tool.description.is_empty(),
                "{} has empty description",
                tool.name
            );
        }
    }

    #[test]
    fn all_schemas_are_objects() {
        for tool in all_tools() {
            assert_eq!(
                tool.input_schema["type"], "object",
                "{} schema is not an object",
                tool.name
            );
        }
    }

    #[test]
    fn all_schemas_have_properties() {
        for tool in all_tools() {
            assert!(
                tool.input_schema.get("properties").is_some(),
                "{} schema missing properties",
                tool.name
            );
        }
    }

    #[test]
    fn data_stats_schema() {
        let tool = data_stats();
        assert_eq!(tool.name, "data_stats");
        let props = &tool.input_schema["properties"];
        assert!(props.get("today").is_some());
    }

    #[test]
    fn data_search_requires_query() {
        let tool = data_search();
        assert_eq!(tool.name, "data_search");
        let required = tool.input_schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("query")));
    }

    #[test]
    fn data_sessions_schema() {
        let tool = data_sessions();
        assert_eq!(tool.name, "data_sessions");
        let props = &tool.input_schema["properties"];
        assert!(props.get("project").is_some());
        assert!(props.get("limit").is_some());
    }

    #[test]
    fn data_errors_schema() {
        let tool = data_errors();
        assert_eq!(tool.name, "data_errors");
        let props = &tool.input_schema["properties"];
        assert!(props.get("recent_days").is_some());
    }

    #[test]
    fn data_tools_schema() {
        let tool = data_tools();
        assert_eq!(tool.name, "data_tools");
        let props = &tool.input_schema["properties"];
        assert!(props.get("tool").is_some());
    }

    #[test]
    fn read_file_requires_path() {
        let tool = read_file();
        assert_eq!(tool.name, "read_file");
        let required = tool.input_schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("path")));
    }

    #[test]
    fn read_file_has_all_mode_params() {
        let tool = read_file();
        let props = &tool.input_schema["properties"];
        for key in &[
            "path",
            "outline",
            "interface",
            "around",
            "context",
            "diff",
            "commit",
        ] {
            assert!(
                props.get(key).is_some(),
                "read_file missing property: {key}"
            );
        }
    }

    #[test]
    fn tool_schemas_are_valid_json() {
        for tool in all_tools() {
            // Each schema should round-trip through to_string/from_str
            let json_str = serde_json::to_string(&tool.input_schema).unwrap();
            let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
            assert_eq!(parsed["type"], "object");
        }
    }

    #[test]
    fn tool_defs_serialize_correctly() {
        for tool in all_tools() {
            let json_str = serde_json::to_string(&tool).unwrap();
            // Must contain camelCase inputSchema, not snake_case
            assert!(
                json_str.contains("inputSchema"),
                "{} missing inputSchema",
                tool.name
            );
            assert!(
                !json_str.contains("input_schema"),
                "{} has snake_case input_schema",
                tool.name
            );
        }
    }
}
