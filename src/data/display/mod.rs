use anyhow::Result;
use comfy_table::presets::UTF8_FULL_CONDENSED;
use comfy_table::{Cell, Color, Table};

use super::pricing::{self, BillingCycle, ValueComparison};
use super::queries::{ModelTokenUsage, PeriodUsage};
use super::types::*;

#[cfg(test)]
mod tests;

// --- Helper formatting ---

pub fn time_ago_ms(ms: i64) -> String {
    let now = chrono::Utc::now().timestamp_millis();
    let diff = now - ms;
    let secs = diff / 1000;

    if secs < 60 {
        "just now".to_string()
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86400 {
        format!("{}h ago", secs / 3600)
    } else {
        format!("{}d ago", secs / 86400)
    }
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else if max <= 3 {
        s[..max].to_string()
    } else {
        format!("{}...", &s[..max - 3])
    }
}

pub fn role_color(role: &str) -> Color {
    match role {
        "user" => Color::Cyan,
        "assistant" => Color::Green,
        _ => Color::White,
    }
}

pub fn status_color(status: &str) -> Color {
    match status {
        "pending" => Color::Yellow,
        "in_progress" => Color::Cyan,
        "completed" => Color::Green,
        _ => Color::White,
    }
}

fn format_tokens(tokens: i64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

// --- Output functions ---

pub fn output_sync(result: &SyncResult, format: &OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(result)?);
        }
        OutputFormat::Table => {
            println!("✓ Sync complete:");
            println!("  Sessions: {} new", result.history);
            println!("  Messages: {} new", result.messages);
            println!("  Todos: {} synced", result.todos);
        }
    }
    Ok(())
}

pub fn output_config(config: &super::config::DataConfig, format: &OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            let json = serde_json::json!({
                "claude_dir": config.claude_dir.display().to_string(),
                "database": config.database.display().to_string(),
                "auto_sync_interval": config.auto_sync_interval,
                "sync_on_start": config.sync_on_start,
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        OutputFormat::Table => {
            println!("Claude dir: {}", config.claude_dir.display());
            println!("Database:   {}", config.database.display());
            println!("Sync interval: {}s", config.auto_sync_interval);
            println!("Sync on start: {}", config.sync_on_start);
        }
    }
    Ok(())
}

pub fn output_sessions(sessions: &[Session], format: &OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(sessions)?);
        }
        OutputFormat::Table => {
            if sessions.is_empty() {
                println!("No sessions found.");
                return Ok(());
            }
            let mut table = Table::new();
            table.load_preset(UTF8_FULL_CONDENSED);
            table.set_header(vec!["ID", "Project", "Display", "Started", "Msgs", "Cost"]);

            for s in sessions {
                table.add_row(vec![
                    Cell::new(truncate(&s.id, 12)),
                    Cell::new(truncate(&s.project, 30)),
                    Cell::new(truncate(s.display.as_deref().unwrap_or("-"), 25)),
                    Cell::new(time_ago_ms(s.started_at)),
                    Cell::new(s.message_count.to_string()),
                    Cell::new(pricing::format_cost(s.total_cost_usd)),
                ]);
            }
            println!("{table}");
        }
    }
    Ok(())
}

pub fn output_session_messages(messages: &[Message], format: &OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(messages)?);
        }
        OutputFormat::Table => {
            if messages.is_empty() {
                println!("No messages found.");
                return Ok(());
            }
            for msg in messages {
                let role = msg.role.as_str();
                let content = msg.content.as_deref().unwrap_or("");
                let preview = truncate(content, 120);
                let model_str = msg.model.as_deref().unwrap_or("");
                let tokens = match (msg.input_tokens, msg.output_tokens) {
                    (Some(i), Some(o)) => format!(" [{}+{}]", format_tokens(i), format_tokens(o)),
                    _ => String::new(),
                };
                let model_suffix = if model_str.is_empty() {
                    String::new()
                } else {
                    format!(" ({model_str})")
                };
                println!("{role}{model_suffix} {preview}{tokens}");
            }
        }
    }
    Ok(())
}

pub fn output_search_results(results: &[SearchResult], format: &OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(results)?);
        }
        OutputFormat::Table => {
            if results.is_empty() {
                println!("No results found.");
                return Ok(());
            }
            let mut table = Table::new();
            table.load_preset(UTF8_FULL_CONDENSED);
            table.set_header(vec!["Role", "Content", "Project", "When"]);

            for r in results {
                let content = r.content.as_deref().unwrap_or("");
                table.add_row(vec![
                    Cell::new(&r.role).fg(role_color(&r.role)),
                    Cell::new(truncate(content, 60)),
                    Cell::new(truncate(&r.project, 25)),
                    Cell::new(time_ago_ms(r.created_at)),
                ]);
            }
            println!("{table}");
        }
    }
    Ok(())
}

pub fn output_stats(
    stats: &UsageStats,
    model_usage: &[ModelUsage],
    format: &OutputFormat,
) -> Result<()> {
    match format {
        OutputFormat::Json => {
            let json = serde_json::json!({
                "stats": stats,
                "model_usage": model_usage,
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        OutputFormat::Table => {
            println!("Usage Statistics:");
            println!("  Sessions: {}", stats.total_sessions);
            println!("  Messages: {}", stats.total_messages);
            println!("  Total cost: {}", pricing::format_cost(stats.total_cost));
            println!(
                "  Input tokens: {}",
                format_tokens(stats.total_input_tokens)
            );
            println!(
                "  Output tokens: {}",
                format_tokens(stats.total_output_tokens)
            );

            if !model_usage.is_empty() {
                println!("\nBy Model:");
                let mut table = Table::new();
                table.load_preset(UTF8_FULL_CONDENSED);
                table.set_header(vec!["Model", "Count", "Cost", "Input", "Output"]);
                for m in model_usage {
                    table.add_row(vec![
                        Cell::new(&m.model),
                        Cell::new(m.count.to_string()),
                        Cell::new(pricing::format_cost(m.cost)),
                        Cell::new(format_tokens(m.input_tokens)),
                        Cell::new(format_tokens(m.output_tokens)),
                    ]);
                }
                println!("{table}");
            }
        }
    }
    Ok(())
}

pub fn output_todos(todos: &[Todo], format: &OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(todos)?);
        }
        OutputFormat::Table => {
            if todos.is_empty() {
                println!("No todos found.");
                return Ok(());
            }
            let mut table = Table::new();
            table.load_preset(UTF8_FULL_CONDENSED);
            table.set_header(vec!["ID", "Status", "Content", "Session"]);

            for t in todos {
                let icon = match t.status.as_str() {
                    "completed" => "✓",
                    "in_progress" => "◐",
                    "pending" => "○",
                    _ => "?",
                };
                table.add_row(vec![
                    Cell::new(t.id.to_string()),
                    Cell::new(format!("{icon} {}", t.status)).fg(status_color(&t.status)),
                    Cell::new(truncate(&t.content, 50)),
                    Cell::new(truncate(&t.session_id, 12)),
                ]);
            }
            println!("{table}");
        }
    }
    Ok(())
}

pub fn output_pending_todos(todos: &[TodoWithProject], format: &OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(todos)?);
        }
        OutputFormat::Table => {
            if todos.is_empty() {
                println!("No pending todos found.");
                return Ok(());
            }
            let mut table = Table::new();
            table.load_preset(UTF8_FULL_CONDENSED);
            table.set_header(vec!["Status", "Content", "Project"]);

            for t in todos {
                let icon = match t.status.as_str() {
                    "in_progress" => "◐",
                    "pending" => "○",
                    _ => "?",
                };
                table.add_row(vec![
                    Cell::new(format!("{icon} {}", t.status)).fg(status_color(&t.status)),
                    Cell::new(truncate(&t.content, 50)),
                    Cell::new(truncate(&t.project, 30)),
                ]);
            }
            println!("{table}");
        }
    }
    Ok(())
}

pub fn output_tool_stats(stats: &[ToolUsageStats], format: &OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(stats)?);
        }
        OutputFormat::Table => {
            if stats.is_empty() {
                println!("No tool usage data.");
                return Ok(());
            }
            let mut table = Table::new();
            table.load_preset(UTF8_FULL_CONDENSED);
            table.set_header(vec!["Tool", "Count", "Last Used"]);

            for s in stats {
                table.add_row(vec![
                    Cell::new(&s.tool_name),
                    Cell::new(s.count.to_string()),
                    Cell::new(time_ago_ms(s.last_used)),
                ]);
            }
            println!("{table}");
        }
    }
    Ok(())
}

pub fn output_tool_detail(detail: &[ToolUsageDetail], format: &OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(detail)?);
        }
        OutputFormat::Table => {
            if detail.is_empty() {
                println!("No usage found for this tool.");
                return Ok(());
            }
            let mut table = Table::new();
            table.load_preset(UTF8_FULL_CONDENSED);
            table.set_header(vec!["Tool", "Session", "Project", "When"]);

            for d in detail {
                table.add_row(vec![
                    Cell::new(&d.tool_name),
                    Cell::new(truncate(&d.session_id, 12)),
                    Cell::new(truncate(&d.project, 30)),
                    Cell::new(time_ago_ms(d.created_at)),
                ]);
            }
            println!("{table}");
        }
    }
    Ok(())
}

pub fn output_errors(errors: &[DebugError], format: &OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(errors)?);
        }
        OutputFormat::Table => {
            if errors.is_empty() {
                println!("No errors found.");
                return Ok(());
            }
            let mut table = Table::new();
            table.load_preset(UTF8_FULL_CONDENSED);
            table.set_header(vec!["File", "Line", "Content"]);

            for e in errors {
                table.add_row(vec![
                    Cell::new(truncate(&e.file, 25)),
                    Cell::new(e.line.to_string()),
                    Cell::new(truncate(&e.content, 60)).fg(Color::Red),
                ]);
            }
            println!("{table}");
        }
    }
    Ok(())
}

// Helper types for pricing display

#[derive(Debug, Clone, serde::Serialize)]
pub struct ModelUsageWithCost {
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PricingData {
    pub subscription: String,
    pub subscription_price: f64,
    pub billing_cycle: BillingCycle,
    pub period_usage: PeriodUsage,
    pub model_costs: Vec<ModelUsageWithCost>,
    pub total_api_cost: f64,
    pub projected_cost: f64,
    pub break_even: pricing::BreakEvenAnalysis,
    pub value_comparisons: Vec<ValueComparison>,
}

pub fn output_pricing(data: &PricingData, format: &OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(data)?);
        }
        OutputFormat::Table => {
            println!(
                "Pricing Analysis ({} plan, ${}/mo):",
                data.subscription, data.subscription_price
            );
            println!(
                "  Billing cycle: {} days elapsed, {} remaining (of {})",
                data.billing_cycle.days_elapsed,
                data.billing_cycle.days_remaining,
                data.billing_cycle.total_days
            );
            println!();

            println!("Current Period Usage:");
            println!("  Messages: {}", data.period_usage.messages);
            println!(
                "  Input tokens: {}",
                format_tokens(data.period_usage.input_tokens)
            );
            println!(
                "  Output tokens: {}",
                format_tokens(data.period_usage.output_tokens)
            );
            println!();

            if !data.model_costs.is_empty() {
                println!("API Cost by Model:");
                let mut table = Table::new();
                table.load_preset(UTF8_FULL_CONDENSED);
                table.set_header(vec!["Model", "Input", "Output", "Cost"]);
                for m in &data.model_costs {
                    table.add_row(vec![
                        Cell::new(&m.model),
                        Cell::new(format_tokens(m.input_tokens)),
                        Cell::new(format_tokens(m.output_tokens)),
                        Cell::new(pricing::format_cost(m.cost)),
                    ]);
                }
                println!("{table}");
            }

            println!(
                "Total API-equivalent cost: {}",
                pricing::format_cost(data.total_api_cost)
            );
            println!(
                "Projected cycle cost: {}",
                pricing::format_cost(data.projected_cost)
            );
            println!();

            println!("Break-even Analysis (Opus 4.5 rates):");
            println!(
                "  Output tokens to break even: {}",
                format_tokens(data.break_even.break_even_output_tokens)
            );
            println!(
                "  Input tokens to break even: {}",
                format_tokens(data.break_even.break_even_input_tokens)
            );

            if !data.value_comparisons.is_empty() {
                println!(
                    "\nValue Comparison (vs API cost {}):",
                    pricing::format_cost(data.total_api_cost)
                );
                let mut table = Table::new();
                table.load_preset(UTF8_FULL_CONDENSED);
                table.set_header(vec!["Service", "Plan", "Price", "Savings"]);
                for v in &data.value_comparisons {
                    let savings_str = if v.savings > 0.0 {
                        format!(
                            "{} ({:.0}%)",
                            pricing::format_cost(v.savings),
                            v.savings_percent
                        )
                    } else {
                        format!("-{}", pricing::format_cost(-v.savings))
                    };
                    table.add_row(vec![
                        Cell::new(&v.service),
                        Cell::new(&v.plan),
                        Cell::new(format!("${:.0}/mo", v.price)),
                        Cell::new(savings_str),
                    ]);
                }
                println!("{table}");
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BranchWithPr {
    pub branch: BranchStats,
    pub pr: Option<PrInfo>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PrInfo {
    pub number: i64,
    pub title: String,
    pub state: String,
    pub url: String,
}

pub fn output_branches(branches: &[BranchWithPr], format: &OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(branches)?);
        }
        OutputFormat::Table => {
            if branches.is_empty() {
                println!("No branches found.");
                return Ok(());
            }
            let mut table = Table::new();
            table.load_preset(UTF8_FULL_CONDENSED);
            table.set_header(vec![
                "Branch",
                "Sessions",
                "Msgs",
                "Cost",
                "Last Active",
                "PR",
            ]);

            for b in branches {
                let pr_str = match &b.pr {
                    Some(pr) => format!("#{} ({})", pr.number, pr.state),
                    None => "-".to_string(),
                };
                table.add_row(vec![
                    Cell::new(truncate(&b.branch.git_branch, 30)),
                    Cell::new(b.branch.session_count.to_string()),
                    Cell::new(b.branch.total_messages.to_string()),
                    Cell::new(pricing::format_cost(b.branch.total_cost)),
                    Cell::new(time_ago_ms(b.branch.last_activity)),
                    Cell::new(pr_str),
                ]);
            }
            println!("{table}");
        }
    }
    Ok(())
}

pub fn build_model_costs(model_usage: &[ModelTokenUsage]) -> Vec<ModelUsageWithCost> {
    model_usage
        .iter()
        .map(|m| {
            let cost = pricing::calculate_cost(Some(&m.model), m.input_tokens, m.output_tokens);
            ModelUsageWithCost {
                model: m.model.clone(),
                input_tokens: m.input_tokens,
                output_tokens: m.output_tokens,
                cost,
            }
        })
        .collect()
}
