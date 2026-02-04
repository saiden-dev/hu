use super::*;

#[test]
fn time_ago_ms_just_now() {
    let now = chrono::Utc::now().timestamp_millis();
    assert_eq!(time_ago_ms(now), "just now");
}

#[test]
fn time_ago_ms_minutes() {
    let now = chrono::Utc::now().timestamp_millis();
    let five_min_ago = now - 5 * 60 * 1000;
    assert_eq!(time_ago_ms(five_min_ago), "5m ago");
}

#[test]
fn time_ago_ms_hours() {
    let now = chrono::Utc::now().timestamp_millis();
    let two_hours_ago = now - 2 * 60 * 60 * 1000;
    assert_eq!(time_ago_ms(two_hours_ago), "2h ago");
}

#[test]
fn time_ago_ms_days() {
    let now = chrono::Utc::now().timestamp_millis();
    let three_days_ago = now - 3 * 24 * 60 * 60 * 1000;
    assert_eq!(time_ago_ms(three_days_ago), "3d ago");
}

#[test]
fn truncate_short() {
    assert_eq!(truncate("hello", 10), "hello");
}

#[test]
fn truncate_exact() {
    assert_eq!(truncate("hello", 5), "hello");
}

#[test]
fn truncate_long() {
    assert_eq!(truncate("hello world", 8), "hello...");
}

#[test]
fn truncate_tiny_max() {
    assert_eq!(truncate("hello", 2), "he");
}

#[test]
fn role_color_values() {
    assert_eq!(role_color("user"), Color::Cyan);
    assert_eq!(role_color("assistant"), Color::Green);
    assert_eq!(role_color("system"), Color::White);
}

#[test]
fn status_color_values() {
    assert_eq!(status_color("pending"), Color::Yellow);
    assert_eq!(status_color("in_progress"), Color::Cyan);
    assert_eq!(status_color("completed"), Color::Green);
    assert_eq!(status_color("other"), Color::White);
}

#[test]
fn format_tokens_small() {
    assert_eq!(format_tokens(500), "500");
}

#[test]
fn format_tokens_thousands() {
    assert_eq!(format_tokens(1500), "1.5K");
}

#[test]
fn format_tokens_millions() {
    assert_eq!(format_tokens(2_500_000), "2.5M");
}

#[test]
fn output_sync_table() {
    let result = SyncResult {
        history: 5,
        messages: 100,
        todos: 3,
    };
    assert!(output_sync(&result, &OutputFormat::Table).is_ok());
}

#[test]
fn output_sync_json() {
    let result = SyncResult {
        history: 0,
        messages: 0,
        todos: 0,
    };
    assert!(output_sync(&result, &OutputFormat::Json).is_ok());
}

#[test]
fn output_config_table() {
    let config = super::super::config::DataConfig::default();
    assert!(output_config(&config, &OutputFormat::Table).is_ok());
}

#[test]
fn output_config_json() {
    let config = super::super::config::DataConfig::default();
    assert!(output_config(&config, &OutputFormat::Json).is_ok());
}

fn make_session() -> Session {
    Session {
        id: "abc-123-def".to_string(),
        project: "/home/user/project".to_string(),
        display: Some("Test session".to_string()),
        started_at: chrono::Utc::now().timestamp_millis(),
        message_count: 10,
        total_cost_usd: 0.05,
        git_branch: Some("main".to_string()),
    }
}

#[test]
fn output_sessions_empty() {
    assert!(output_sessions(&[], &OutputFormat::Table).is_ok());
}

#[test]
fn output_sessions_table() {
    let sessions = vec![make_session()];
    assert!(output_sessions(&sessions, &OutputFormat::Table).is_ok());
}

#[test]
fn output_sessions_json() {
    let sessions = vec![make_session()];
    assert!(output_sessions(&sessions, &OutputFormat::Json).is_ok());
}

fn make_message(role: &str) -> Message {
    Message {
        id: "msg-1".to_string(),
        session_id: "sess-1".to_string(),
        parent_id: None,
        role: role.to_string(),
        content: Some("Hello world".to_string()),
        model: if role == "assistant" {
            Some("claude-sonnet-4-5-20251101".to_string())
        } else {
            None
        },
        input_tokens: Some(100),
        output_tokens: Some(200),
        cost_usd: Some(0.001),
        duration_ms: Some(500),
        created_at: chrono::Utc::now().timestamp_millis(),
    }
}

#[test]
fn output_session_messages_empty() {
    assert!(output_session_messages(&[], &OutputFormat::Table).is_ok());
}

#[test]
fn output_session_messages_table() {
    let msgs = vec![make_message("user"), make_message("assistant")];
    assert!(output_session_messages(&msgs, &OutputFormat::Table).is_ok());
}

#[test]
fn output_session_messages_json() {
    let msgs = vec![make_message("user")];
    assert!(output_session_messages(&msgs, &OutputFormat::Json).is_ok());
}

#[test]
fn output_session_messages_no_tokens() {
    let msg = Message {
        input_tokens: None,
        output_tokens: None,
        model: None,
        ..make_message("user")
    };
    assert!(output_session_messages(&[msg], &OutputFormat::Table).is_ok());
}

#[test]
fn output_search_results_empty() {
    assert!(output_search_results(&[], &OutputFormat::Table).is_ok());
}

#[test]
fn output_search_results_table() {
    let results = vec![SearchResult {
        id: "m1".to_string(),
        session_id: "s1".to_string(),
        role: "user".to_string(),
        content: Some("test query result".to_string()),
        model: None,
        created_at: chrono::Utc::now().timestamp_millis(),
        project: "/home/user/proj".to_string(),
    }];
    assert!(output_search_results(&results, &OutputFormat::Table).is_ok());
}

#[test]
fn output_search_results_json() {
    assert!(output_search_results(&[], &OutputFormat::Json).is_ok());
}

#[test]
fn output_stats_table() {
    let stats = UsageStats {
        total_sessions: 10,
        total_messages: 100,
        total_cost: 5.0,
        total_input_tokens: 1_000_000,
        total_output_tokens: 500_000,
    };
    let model_usage = vec![ModelUsage {
        model: "claude-sonnet-4-5-20251101".to_string(),
        count: 50,
        cost: 3.0,
        input_tokens: 800_000,
        output_tokens: 400_000,
    }];
    assert!(output_stats(&stats, &model_usage, &OutputFormat::Table).is_ok());
}

#[test]
fn output_stats_json() {
    let stats = UsageStats::default();
    assert!(output_stats(&stats, &[], &OutputFormat::Json).is_ok());
}

#[test]
fn output_stats_empty_models() {
    let stats = UsageStats::default();
    assert!(output_stats(&stats, &[], &OutputFormat::Table).is_ok());
}

fn make_todo(status: &str) -> Todo {
    Todo {
        id: 1,
        session_id: "sess-1".to_string(),
        content: "Fix the bug".to_string(),
        status: status.to_string(),
        active_form: Some("Fixing bug".to_string()),
    }
}

#[test]
fn output_todos_empty() {
    assert!(output_todos(&[], &OutputFormat::Table).is_ok());
}

#[test]
fn output_todos_table() {
    let todos = vec![
        make_todo("pending"),
        make_todo("in_progress"),
        make_todo("completed"),
    ];
    assert!(output_todos(&todos, &OutputFormat::Table).is_ok());
}

#[test]
fn output_todos_json() {
    assert!(output_todos(&[make_todo("pending")], &OutputFormat::Json).is_ok());
}

#[test]
fn output_pending_todos_empty() {
    assert!(output_pending_todos(&[], &OutputFormat::Table).is_ok());
}

#[test]
fn output_pending_todos_table() {
    let todos = vec![TodoWithProject {
        id: 1,
        session_id: "s1".to_string(),
        content: "Review PR".to_string(),
        status: "pending".to_string(),
        active_form: None,
        project: "/home/user/proj".to_string(),
    }];
    assert!(output_pending_todos(&todos, &OutputFormat::Table).is_ok());
}

#[test]
fn output_pending_todos_json() {
    assert!(output_pending_todos(&[], &OutputFormat::Json).is_ok());
}

#[test]
fn output_tool_stats_empty() {
    assert!(output_tool_stats(&[], &OutputFormat::Table).is_ok());
}

#[test]
fn output_tool_stats_table() {
    let stats = vec![ToolUsageStats {
        tool_name: "Read".to_string(),
        count: 42,
        last_used: chrono::Utc::now().timestamp_millis(),
    }];
    assert!(output_tool_stats(&stats, &OutputFormat::Table).is_ok());
}

#[test]
fn output_tool_stats_json() {
    assert!(output_tool_stats(&[], &OutputFormat::Json).is_ok());
}

#[test]
fn output_tool_detail_empty() {
    assert!(output_tool_detail(&[], &OutputFormat::Table).is_ok());
}

#[test]
fn output_tool_detail_table() {
    let detail = vec![ToolUsageDetail {
        tool_name: "Edit".to_string(),
        session_id: "sess-1".to_string(),
        project: "/home/user/proj".to_string(),
        created_at: chrono::Utc::now().timestamp_millis(),
    }];
    assert!(output_tool_detail(&detail, &OutputFormat::Table).is_ok());
}

#[test]
fn output_tool_detail_json() {
    assert!(output_tool_detail(&[], &OutputFormat::Json).is_ok());
}

#[test]
fn output_errors_empty() {
    assert!(output_errors(&[], &OutputFormat::Table).is_ok());
}

#[test]
fn output_errors_table() {
    let errors = vec![DebugError {
        file: "debug.log".to_string(),
        line: 10,
        content: "Error: something failed".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    }];
    assert!(output_errors(&errors, &OutputFormat::Table).is_ok());
}

#[test]
fn output_errors_json() {
    assert!(output_errors(&[], &OutputFormat::Json).is_ok());
}

fn make_pricing_data() -> PricingData {
    PricingData {
        subscription: "max20x".to_string(),
        subscription_price: 200.0,
        billing_cycle: pricing::calculate_billing_cycle(6, chrono::Utc::now().timestamp_millis()),
        period_usage: PeriodUsage {
            messages: 100,
            input_tokens: 500_000,
            output_tokens: 200_000,
        },
        model_costs: vec![ModelUsageWithCost {
            model: "claude-sonnet-4-5-20251101".to_string(),
            input_tokens: 500_000,
            output_tokens: 200_000,
            cost: 4.5,
        }],
        total_api_cost: 4.5,
        projected_cost: 9.0,
        break_even: pricing::calculate_break_even(200.0),
        value_comparisons: pricing::get_value_comparison(4.5),
    }
}

#[test]
fn output_pricing_table() {
    assert!(output_pricing(&make_pricing_data(), &OutputFormat::Table).is_ok());
}

#[test]
fn output_pricing_json() {
    assert!(output_pricing(&make_pricing_data(), &OutputFormat::Json).is_ok());
}

#[test]
fn output_pricing_empty_models() {
    let mut data = make_pricing_data();
    data.model_costs = vec![];
    data.value_comparisons = vec![];
    assert!(output_pricing(&data, &OutputFormat::Table).is_ok());
}

#[test]
fn output_branches_empty() {
    assert!(output_branches(&[], &OutputFormat::Table).is_ok());
}

#[test]
fn output_branches_table() {
    let branches = vec![
        BranchWithPr {
            branch: BranchStats {
                git_branch: "main".to_string(),
                session_count: 5,
                session_ids: "s1,s2,s3".to_string(),
                last_activity: chrono::Utc::now().timestamp_millis(),
                total_messages: 50,
                total_cost: 1.5,
                project: "/home/user/proj".to_string(),
            },
            pr: Some(PrInfo {
                number: 42,
                title: "Add feature".to_string(),
                state: "OPEN".to_string(),
                url: "https://github.com/org/repo/pull/42".to_string(),
            }),
        },
        BranchWithPr {
            branch: BranchStats {
                git_branch: "feature/x".to_string(),
                session_count: 1,
                session_ids: "s4".to_string(),
                last_activity: chrono::Utc::now().timestamp_millis(),
                total_messages: 10,
                total_cost: 0.3,
                project: "/home/user/proj".to_string(),
            },
            pr: None,
        },
    ];
    assert!(output_branches(&branches, &OutputFormat::Table).is_ok());
}

#[test]
fn output_branches_json() {
    assert!(output_branches(&[], &OutputFormat::Json).is_ok());
}

#[test]
fn build_model_costs_empty() {
    let costs = build_model_costs(&[]);
    assert!(costs.is_empty());
}

#[test]
fn build_model_costs_calculates() {
    let usage = vec![ModelTokenUsage {
        model: "claude-sonnet-4-5-20251101".to_string(),
        input_tokens: 1_000_000,
        output_tokens: 1_000_000,
    }];
    let costs = build_model_costs(&usage);
    assert_eq!(costs.len(), 1);
    assert!((costs[0].cost - 18.0).abs() < 0.01);
}

#[test]
fn output_pricing_negative_savings() {
    let mut data = make_pricing_data();
    data.total_api_cost = 5.0;
    data.value_comparisons = vec![ValueComparison {
        service: "Test".to_string(),
        plan: "Premium".to_string(),
        price: 200.0,
        savings: -195.0,
        savings_percent: -3900.0,
    }];
    assert!(output_pricing(&data, &OutputFormat::Table).is_ok());
}

#[test]
fn output_todos_unknown_status() {
    let todo = Todo {
        id: 1,
        session_id: "s1".to_string(),
        content: "test".to_string(),
        status: "unknown_status".to_string(),
        active_form: None,
    };
    assert!(output_todos(&[todo], &OutputFormat::Table).is_ok());
}

#[test]
fn output_pending_in_progress() {
    let todo = TodoWithProject {
        id: 1,
        session_id: "s1".to_string(),
        content: "task".to_string(),
        status: "in_progress".to_string(),
        active_form: None,
        project: "/proj".to_string(),
    };
    assert!(output_pending_todos(&[todo], &OutputFormat::Table).is_ok());
}
