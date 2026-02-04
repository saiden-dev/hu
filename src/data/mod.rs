mod cli;
mod config;
mod db;
mod display;
mod paths;
mod pricing;
mod queries;
mod schema;
mod sync;
mod types;

pub use cli::DataCommand;

use anyhow::{bail, Result};
use types::OutputFormat;

pub async fn run_command(cmd: DataCommand) -> Result<()> {
    match cmd {
        DataCommand::Sync { force, quiet } => cmd_sync(force, quiet),
        DataCommand::Config { json } => cmd_config(json),
        DataCommand::Session { cmd } => cmd_session(cmd),
        DataCommand::Stats { json, today } => cmd_stats(json, today),
        DataCommand::Todos { cmd } => cmd_todos(cmd),
        DataCommand::Search { query, limit, json } => cmd_search(&query, limit, json),
        DataCommand::Tools { tool, json } => cmd_tools(tool.as_deref(), json),
        DataCommand::Errors { recent, json } => cmd_errors(recent, json),
        DataCommand::Pricing {
            subscription,
            billing_day,
            json,
        } => cmd_pricing(&subscription, billing_day, json),
        DataCommand::Branches {
            branch,
            limit,
            json,
        } => cmd_branches(branch.as_deref(), limit, json).await,
    }
}

fn get_format(json: bool) -> OutputFormat {
    if json {
        OutputFormat::Json
    } else {
        OutputFormat::Table
    }
}

fn open_db() -> Result<db::SqliteStore> {
    let cfg = config::load_data_config()?;
    let store = db::SqliteStore::open_initialized(&cfg.database)?;
    Ok(store)
}

fn ensure_synced(store: &db::SqliteStore) -> Result<()> {
    let cfg = config::load_data_config()?;
    sync::sync_if_needed(&store.conn, &cfg.claude_dir, cfg.auto_sync_interval)?;
    Ok(())
}

fn cmd_sync(force: bool, quiet: bool) -> Result<()> {
    let cfg = config::load_data_config()?;
    let store = db::SqliteStore::open_initialized(&cfg.database)?;

    let result = if force {
        sync::sync_all(&store.conn, &cfg.claude_dir)?
    } else {
        let synced = sync::sync_if_needed(&store.conn, &cfg.claude_dir, cfg.auto_sync_interval)?;
        if !synced {
            if !quiet {
                println!("Already up to date. Use -f to force.");
            }
            return Ok(());
        }
        sync::sync_all(&store.conn, &cfg.claude_dir)?
    };

    if !quiet {
        display::output_sync(&result, &OutputFormat::Table)?;
    }
    Ok(())
}

fn cmd_config(json: bool) -> Result<()> {
    let cfg = config::load_data_config()?;
    display::output_config(&cfg, &get_format(json))
}

fn cmd_session(cmd: cli::SessionCommand) -> Result<()> {
    let store = open_db()?;
    ensure_synced(&store)?;

    match cmd {
        cli::SessionCommand::List {
            project,
            limit,
            json,
        } => {
            let sessions = queries::get_sessions(&store.conn, project.as_deref(), limit)?;
            display::output_sessions(&sessions, &get_format(json))
        }
        cli::SessionCommand::Read { id, json } => {
            let session = queries::get_session_by_prefix(&store.conn, &id)?
                .or_else(|| queries::get_session_by_id(&store.conn, &id).ok().flatten());

            match session {
                Some(s) => {
                    let messages = queries::get_messages_by_session(&store.conn, &s.id)?;
                    display::output_session_messages(&messages, &get_format(json))
                }
                None => bail!("Session not found: {id}"),
            }
        }
        cli::SessionCommand::Current { json } => {
            let session_id = std::env::var("SESSION_ID").unwrap_or_default();
            if session_id.is_empty() {
                bail!("SESSION_ID environment variable not set");
            }
            let session = queries::get_session_by_id(&store.conn, &session_id)?;
            match session {
                Some(s) => {
                    let messages = queries::get_messages_by_session(&store.conn, &s.id)?;
                    display::output_session_messages(&messages, &get_format(json))
                }
                None => bail!("Current session not found: {session_id}"),
            }
        }
    }
}

fn cmd_stats(json: bool, today: bool) -> Result<()> {
    let store = open_db()?;
    ensure_synced(&store)?;

    let since = if today {
        Some(start_of_today_ms())
    } else {
        None
    };

    let stats = queries::get_usage_stats(&store.conn, since)?;
    let model_usage = queries::get_model_usage(&store.conn, since)?;
    display::output_stats(&stats, &model_usage, &get_format(json))
}

fn cmd_todos(cmd: cli::TodosCommand) -> Result<()> {
    let store = open_db()?;
    ensure_synced(&store)?;

    match cmd {
        cli::TodosCommand::List { status, json } => {
            let todos = queries::get_todos(&store.conn, status.as_deref())?;
            display::output_todos(&todos, &get_format(json))
        }
        cli::TodosCommand::Pending { project, json } => {
            let todos = queries::get_pending_todos(&store.conn, project.as_deref())?;
            display::output_pending_todos(&todos, &get_format(json))
        }
    }
}

fn cmd_search(query: &str, limit: i64, json: bool) -> Result<()> {
    let store = open_db()?;
    ensure_synced(&store)?;

    let results = queries::search_messages(&store.conn, query, limit)?;
    display::output_search_results(&results, &get_format(json))
}

fn cmd_tools(tool: Option<&str>, json: bool) -> Result<()> {
    let store = open_db()?;
    ensure_synced(&store)?;
    let format = get_format(json);

    match tool {
        Some(name) => {
            let detail = queries::get_tool_detail(&store.conn, name)?;
            display::output_tool_detail(&detail, &format)
        }
        None => {
            let stats = queries::get_tool_stats(&store.conn)?;
            display::output_tool_stats(&stats, &format)
        }
    }
}

fn cmd_errors(recent_days: u32, json: bool) -> Result<()> {
    let cfg = config::load_data_config()?;
    let errors = scan_debug_errors(&cfg.claude_dir, recent_days)?;
    display::output_errors(&errors, &get_format(json))
}

fn scan_debug_errors(
    claude_dir: &std::path::Path,
    recent_days: u32,
) -> Result<Vec<types::DebugError>> {
    let dir = paths::debug_dir(claude_dir);
    if !dir.exists() {
        return Ok(vec![]);
    }

    let cutoff = chrono::Utc::now().timestamp() - (recent_days as i64 * 86400);
    let error_patterns =
        regex::Regex::new(r"(?i)(error|failed|exception|warning|ENOENT|EACCES|EPERM)")?;

    let mut errors = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("txt") {
            continue;
        }

        let metadata = entry.metadata()?;
        let modified = metadata
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        if modified < cutoff {
            continue;
        }

        let content = std::fs::read_to_string(&path)?;
        let filename = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        for (i, line) in content.lines().enumerate() {
            if error_patterns.is_match(line) && seen.insert(line.to_string()) {
                errors.push(types::DebugError {
                    file: filename.clone(),
                    line: i + 1,
                    content: line.to_string(),
                    timestamp: modified,
                });
            }
        }
    }

    errors.truncate(50);
    Ok(errors)
}

fn cmd_pricing(subscription: &str, billing_day: u32, json: bool) -> Result<()> {
    let store = open_db()?;
    ensure_synced(&store)?;

    let now = chrono::Utc::now().timestamp_millis();
    let cycle = pricing::calculate_billing_cycle(billing_day, now);
    let sub_price = pricing::get_subscription_price(subscription);

    let period_usage = queries::get_period_usage(&store.conn, cycle.start_ms)?;
    let model_usage = queries::get_period_model_usage(&store.conn, cycle.start_ms)?;
    let model_costs = display::build_model_costs(&model_usage);
    let total_api_cost: f64 = model_costs.iter().map(|m| m.cost).sum();
    let projected =
        pricing::project_cycle_cost(total_api_cost, cycle.days_elapsed, cycle.total_days);
    let break_even = pricing::calculate_break_even(sub_price);
    let comparisons = pricing::get_value_comparison(total_api_cost);

    let data = display::PricingData {
        subscription: subscription.to_string(),
        subscription_price: sub_price,
        billing_cycle: cycle,
        period_usage,
        model_costs,
        total_api_cost,
        projected_cost: projected,
        break_even,
        value_comparisons: comparisons,
    };

    display::output_pricing(&data, &get_format(json))
}

async fn cmd_branches(branch: Option<&str>, limit: i64, json: bool) -> Result<()> {
    let store = open_db()?;
    ensure_synced(&store)?;

    let stats = queries::get_branch_stats(&store.conn, branch, limit)?;
    let mut branches = Vec::new();

    for b in stats {
        let pr = fetch_pr_info(&b.git_branch).await;
        branches.push(display::BranchWithPr { branch: b, pr });
    }

    display::output_branches(&branches, &get_format(json))
}

async fn fetch_pr_info(branch: &str) -> Option<display::PrInfo> {
    let output: std::process::Output = tokio::process::Command::new("gh")
        .args([
            "pr",
            "list",
            "--head",
            branch,
            "--json",
            "number,title,state,url",
            "--limit",
            "1",
        ])
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let prs: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).ok()?;
    let pr = prs.first()?;

    Some(display::PrInfo {
        number: pr.get("number")?.as_i64()?,
        title: pr.get("title")?.as_str()?.to_string(),
        state: pr.get("state")?.as_str()?.to_string(),
        url: pr.get("url")?.as_str()?.to_string(),
    })
}

fn start_of_today_ms() -> i64 {
    let now = chrono::Utc::now();
    now.date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_format_json() {
        assert!(matches!(get_format(true), OutputFormat::Json));
    }

    #[test]
    fn get_format_table() {
        assert!(matches!(get_format(false), OutputFormat::Table));
    }

    #[test]
    fn start_of_today_is_past() {
        let ms = start_of_today_ms();
        let now = chrono::Utc::now().timestamp_millis();
        assert!(ms <= now);
        assert!(ms > now - 86_400_000); // Within last 24h
    }

    #[test]
    fn scan_debug_errors_missing_dir() {
        let errors = scan_debug_errors(std::path::Path::new("/nonexistent"), 7).unwrap();
        assert!(errors.is_empty());
    }

    #[test]
    fn scan_debug_errors_with_fixture() {
        let tmp = std::env::temp_dir().join("hu-test-debug-errors");
        let _ = std::fs::remove_dir_all(&tmp);
        let debug = tmp.join("debug");
        std::fs::create_dir_all(&debug).unwrap();

        std::fs::write(
            debug.join("test.txt"),
            "normal line\nError: something broke\nFailed to connect\nanother normal line\n",
        )
        .unwrap();

        let errors = scan_debug_errors(&tmp, 7).unwrap();
        assert_eq!(errors.len(), 2);
        assert!(errors[0].content.contains("Error"));
        assert!(errors[1].content.contains("Failed"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn scan_debug_errors_deduplication() {
        let tmp = std::env::temp_dir().join("hu-test-debug-dedup");
        let _ = std::fs::remove_dir_all(&tmp);
        let debug = tmp.join("debug");
        std::fs::create_dir_all(&debug).unwrap();

        std::fs::write(debug.join("a.txt"), "Error: same message\n").unwrap();
        std::fs::write(debug.join("b.txt"), "Error: same message\n").unwrap();

        let errors = scan_debug_errors(&tmp, 7).unwrap();
        assert_eq!(errors.len(), 1); // Deduplicated

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn scan_debug_errors_skips_non_txt() {
        let tmp = std::env::temp_dir().join("hu-test-debug-skip");
        let _ = std::fs::remove_dir_all(&tmp);
        let debug = tmp.join("debug");
        std::fs::create_dir_all(&debug).unwrap();

        std::fs::write(debug.join("test.log"), "Error: in log file\n").unwrap();

        let errors = scan_debug_errors(&tmp, 7).unwrap();
        assert!(errors.is_empty()); // .log not .txt

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn scan_debug_errors_max_50() {
        let tmp = std::env::temp_dir().join("hu-test-debug-max");
        let _ = std::fs::remove_dir_all(&tmp);
        let debug = tmp.join("debug");
        std::fs::create_dir_all(&debug).unwrap();

        let mut content = String::new();
        for i in 0..60 {
            content.push_str(&format!("Error: unique error {i}\n"));
        }
        std::fs::write(debug.join("many.txt"), &content).unwrap();

        let errors = scan_debug_errors(&tmp, 7).unwrap();
        assert_eq!(errors.len(), 50); // Capped at 50

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
