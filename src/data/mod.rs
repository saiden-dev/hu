mod cli;
mod config;
mod db;
mod display;
mod paths;
mod pricing;
mod queries;
mod schema;
pub mod service;
mod sync;
mod types;

pub use cli::DataCommand;

use anyhow::Result;
use types::OutputFormat;

#[cfg(not(tarpaulin_include))]
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

#[cfg(not(tarpaulin_include))]
fn cmd_sync(force: bool, quiet: bool) -> Result<()> {
    let store = service::open_db()?;
    match service::sync_data(&store, force)? {
        Some(result) => {
            if !quiet {
                display::output_sync(&result, &OutputFormat::Table)?;
            }
        }
        None => {
            if !quiet {
                println!("Already up to date. Use -f to force.");
            }
        }
    }
    Ok(())
}

#[cfg(not(tarpaulin_include))]
fn cmd_config(json: bool) -> Result<()> {
    let cfg = service::get_config()?;
    display::output_config(&cfg, &get_format(json))
}

#[cfg(not(tarpaulin_include))]
fn cmd_session(cmd: cli::SessionCommand) -> Result<()> {
    let store = service::open_db()?;
    service::ensure_synced(&store)?;

    match cmd {
        cli::SessionCommand::List {
            project,
            limit,
            json,
        } => {
            let sessions = service::get_sessions(&store, project.as_deref(), limit)?;
            display::output_sessions(&sessions, &get_format(json))
        }
        cli::SessionCommand::Read { id, json } => {
            let (_session, messages) = service::get_session_messages(&store, &id)?;
            display::output_session_messages(&messages, &get_format(json))
        }
        cli::SessionCommand::Current { json } => {
            let (_session, messages) = service::get_current_session_messages(&store)?;
            display::output_session_messages(&messages, &get_format(json))
        }
    }
}

#[cfg(not(tarpaulin_include))]
fn cmd_stats(json: bool, today: bool) -> Result<()> {
    let store = service::open_db()?;
    service::ensure_synced(&store)?;
    let (stats, model_usage) = service::get_stats(&store, today)?;
    display::output_stats(&stats, &model_usage, &get_format(json))
}

#[cfg(not(tarpaulin_include))]
fn cmd_todos(cmd: cli::TodosCommand) -> Result<()> {
    let store = service::open_db()?;
    service::ensure_synced(&store)?;

    match cmd {
        cli::TodosCommand::List { status, json } => {
            let todos = service::get_todos(&store, status.as_deref())?;
            display::output_todos(&todos, &get_format(json))
        }
        cli::TodosCommand::Pending { project, json } => {
            let todos = service::get_pending_todos(&store, project.as_deref())?;
            display::output_pending_todos(&todos, &get_format(json))
        }
    }
}

#[cfg(not(tarpaulin_include))]
fn cmd_search(query: &str, limit: i64, json: bool) -> Result<()> {
    let store = service::open_db()?;
    service::ensure_synced(&store)?;
    let results = service::search_messages(&store, query, limit)?;
    display::output_search_results(&results, &get_format(json))
}

#[cfg(not(tarpaulin_include))]
fn cmd_tools(tool: Option<&str>, json: bool) -> Result<()> {
    let store = service::open_db()?;
    service::ensure_synced(&store)?;
    let format = get_format(json);

    match tool {
        Some(name) => {
            let detail = service::get_tool_detail(&store, name)?;
            display::output_tool_detail(&detail, &format)
        }
        None => {
            let stats = service::get_tool_stats(&store)?;
            display::output_tool_stats(&stats, &format)
        }
    }
}

#[cfg(not(tarpaulin_include))]
fn cmd_errors(recent_days: u32, json: bool) -> Result<()> {
    let cfg = service::get_config()?;
    let errors = service::scan_debug_errors(&cfg.claude_dir, recent_days)?;
    display::output_errors(&errors, &get_format(json))
}

#[cfg(not(tarpaulin_include))]
fn cmd_pricing(subscription: &str, billing_day: u32, json: bool) -> Result<()> {
    let store = service::open_db()?;
    service::ensure_synced(&store)?;
    let data = service::compute_pricing(&store, subscription, billing_day)?;
    display::output_pricing(&data, &get_format(json))
}

#[cfg(not(tarpaulin_include))]
async fn cmd_branches(branch: Option<&str>, limit: i64, json: bool) -> Result<()> {
    let store = service::open_db()?;
    service::ensure_synced(&store)?;

    let stats = service::get_branch_stats(&store, branch, limit)?;
    let mut branches = Vec::new();

    for b in stats {
        let pr = service::fetch_pr_info(&b.git_branch).await;
        branches.push(types::BranchWithPr { branch: b, pr });
    }

    display::output_branches(&branches, &get_format(json))
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
}
