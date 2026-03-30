# Refactoring Plan: Interface-Agnostic Architecture

Generated: 2026-02-08 | Last audited: 2026-03-30
Scope: All modules - enabling reuse across CLI, MCP, and HTTP interfaces

## Summary

**Goal**: Extract service layers so the same business logic powers CLI, MCP server, and HTTP API.

**Current state**: 15+ modules. Critical path complete — MCP server is live with 7 tools. All high-priority service extractions done. Traits exist for 7 API clients.

| Pattern | Count | Modules |
|---------|-------|---------|
| Has API trait | 7 | pagerduty, gh, jira, newrelic, sentry, slack, web_search |
| Has service.rs | 14 | pagerduty, sentry, newrelic, slack, jira, gh, read, git, docs, data, cron, context, shell/df, shell/ls |
| Display separated | 10+ | Most modules |
| MCP server | 1 | `hu mcp serve` — 7 tools (data_stats, data_search, data_sessions, data_errors, data_pricing, data_tools, read_file) |

**Progress**: 10 done, 2 partial, 7 not started out of 19 original items.

**Note**: MCP module is live. `#[allow(dead_code)]` annotations on public API functions can now be removed as MCP handlers consume them.

---

## High Priority

### [x] H1: Extract service layer from pagerduty module _(done)_
- `src/pagerduty/service.rs` (314 lines) — `list_oncalls`, `list_incidents`, `list_alerts`, `get_incident`, `get_current_user`
- All functions accept `&impl PagerDutyApi`
- `mod.rs` has thin `cmd_*` handlers behind `#[cfg(not(tarpaulin_include))]`
- Re-exports public API functions for MCP/HTTP use
- Tests use `MockApi`

### [x] H2: Extract service layer from slack module _(done)_
- `src/slack/service/mod.rs` — full service layer with `list_channels`, `get_channel_info`, `get_history`, `send_message`, `search_messages`, `list_users`, `build_user_lookup`, `authenticate`, `whoami`, `run_tidy`, `compute_tidy_summary`
- All functions accept `&impl SlackApi` (M3 trait landed)
- `handlers.rs` reduced from 277 to 191 lines, 0 `println!` calls — all output via display layer
- New types: `AuthInfo`, `AuthResult`, `TidySummary` in `types.rs`
- Service tests in `service/tests.rs`

### [x] H3: Extract service layer from newrelic module _(done)_
- `src/newrelic/service.rs` (219 lines) — `list_issues`, `list_incidents`, `run_nrql`
- All functions accept `&impl NewRelicApi`
- `NewRelicApi` trait at `client/mod.rs:19`
- Tests use `MockApi`

### [x] H4: Extract service layer from sentry module _(done)_
- `src/sentry/service.rs` (386 lines) — `list_issues`, `get_issue`, `list_events`
- All functions accept `&impl SentryApi`
- `SentryApi` trait at `client.rs:18`
- Tests use `MockApi`

### [x] H5: Extract service layer from data module _(done)_
- `src/data/service.rs` — `open_db`, `ensure_synced`, `sync_data`, `get_sessions`, `get_session_messages`, `get_current_session_messages`, `get_stats`, `get_todos`, `get_pending_todos`, `search_messages`, `get_tool_stats`, `get_tool_detail`, `scan_debug_errors`, `compute_pricing`, `get_branch_stats`, `fetch_pr_info`
- `mod.rs` reduced from 474 to 194 lines — thin handlers only
- `PricingData`, `ModelUsageWithCost`, `BranchWithPr`, `PrInfo`, `build_model_costs` moved to `types.rs`
- Service tests cover `scan_debug_errors`, `compute_tidy_summary`, validation functions

### [x] H6: Fix read/service.rs - remove println! _(done)_
- `read/service.rs` returns `ReadOutput` enum with variants `Full`, `Outline`, `Interface`, `Around`, `Diff`
- No `println!` in service layer
- `read/mod.rs` is a thin wrapper: `service::run()` -> `display::format()` -> `print!()`
- Exposes `pub fn read(args) -> Result<ReadOutput>` for programmatic use

### [x] H7: Create MCP server module _(done)_
- `src/mcp/` — 6 files: `mod.rs`, `cli.rs`, `types.rs`, `tools.rs`, `handlers.rs`, `server.rs`
- JSON-RPC 2.0 over stdio with `initialize`, `tools/list`, `tools/call` methods
- 7 tools: `data_stats`, `data_search`, `data_sessions`, `data_errors`, `data_pricing`, `data_tools`, `read_file`
- 72 tests covering types, tools, handlers, and server dispatch
- CLI: `hu mcp serve` (start server), `hu mcp list` (show tools)
- Register: `claude mcp add hu -- hu mcp serve`
- **Future**: Add prompts, resources, and more tools (pagerduty, sentry, slack, jira) as needed

---

## Medium Priority

### [x] M1: Add trait to NewRelic client _(done)_
- `pub trait NewRelicApi` at `src/newrelic/client/mod.rs:19`
- Used in service.rs with `&impl NewRelicApi`

### [x] M2: Add trait to Sentry client _(done)_
- `pub trait SentryApi` at `src/sentry/client.rs:18`
- Used in service.rs with `&impl SentryApi`

### [x] M3: Add trait to Slack client _(done)_
- `pub trait SlackApi: Send + Sync` at `src/slack/client.rs:21`
- 5 methods: `get`, `get_with_params`, `get_with_user_token`, `post`, `post_with_user_token`
- Low-level HTTP trait (unlike other modules' domain-level traits) — business logic lives in channels/, messages/, search/ free functions
- All consumers updated to `&impl SlackApi`

### [ ] M4: Flatten module exports _(partial)_
- Types are re-exported in several modules: pagerduty, sentry, newrelic, slack, read, git
- **Not re-exported**: Client types and traits. External consumers still need `pagerduty::client::PagerDutyApi`
- **Not started**: data, install, docs have minimal re-exports (just command enum)
- **Remaining action**: Add `pub use client::{Client, XxxApi}` to each module's mod.rs

### [x] M5: Consolidate OutputFormat types _(done)_
- Single definition in `src/util/output.rs`: `#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]`
- 7 modules updated to re-export via `pub use crate::util::OutputFormat`
- All display/handler imports work unchanged through module-level re-exports

### [ ] M6: Extract install display logic
- `src/install/mod.rs` is 369 lines with 25 `println!` calls (was 443 in original plan)
- No `display.rs` or `service.rs`
- **Action**:
  1. Create `src/install/display.rs` for `print_status_table()` and output
  2. Create `src/install/service.rs` with:
     ```rust
     pub fn check_statuses(components: &[Component], base: &Path) -> Vec<ComponentStatus>
     pub fn install_components(components: &[Component], base: &Path) -> Result<InstallReport>
     ```

### [ ] M7: Split large client files
- `src/gh/client/mod.rs`: 417 lines (was 455 — slightly smaller but still over limit)
- `src/slack/client.rs`: 347 lines (was 389 — slightly smaller but still large)
- **Action**: Split by concern:
  - `gh/client/` -> `mod.rs`, `runs.rs`, `prs.rs`, `checks.rs`
  - `slack/client/` -> `mod.rs`, `channels.rs`, `messages.rs`, `search.rs`

### [ ] M8: Standardize handler naming _(partial)_
- Two conventions in use:
  - `run()`: pagerduty, sentry, newrelic, eks, pipeline, read
  - `run_command()`: data, jira, gh, docs, utils, install, context, cron, shell
- Within modules, private handlers consistently use `cmd_*` pattern
- **Action**: Standardize to `run(cmd: Command)` everywhere

---

## Low Priority

### [ ] L1: Remove redundant type prefixes
- `SentryConfig` in `sentry::config`, `SlackConfig` in `slack::config`, etc.
- **Action**: Rename to just `Config`, re-export at module level

### [ ] L2: Consolidate check_configured patterns _(partial)_
- Renamed to `ensure_configured` consistently across modules (pagerduty, sentry, newrelic, slack)
- Each now lives in its module's `service.rs` (not duplicated in handlers)
- **Remaining**: No shared `Configured` trait — each module implements its own version
- **Action**: Consider trait method or shared helper:
  ```rust
  pub trait Configured {
      fn ensure_configured(&self) -> Result<()>;
  }
  ```

### [ ] L3: Extract common table formatting
- No `src/util/table.rs` or shared `new_table()` function
- Each display module sets up `comfy_table` independently
- **Action**: Create `src/util/table.rs`:
  ```rust
  pub fn new_table() -> Table {
      let mut t = Table::new();
      t.load_preset(UTF8_FULL_CONDENSED);
      t
  }
  ```

### [ ] L4: Document service layer pattern
- `CLAUDE.md` describes the high-level architecture but not the service layer contract
- No `ARCHITECTURE.md` exists
- **Action**: Add architecture section explaining:
  - Service layer returns data, never prints
  - Traits for all API clients
  - CLI handlers are thin wrappers

---

## New Issues (discovered 2026-03-26)

### [ ] N1: Cover new modules in refactoring plan
- Modules added since original plan: `cron`, `shell/df`, `shell/ls`, `context`, `pipeline`, `eks`
- Some already have service.rs (cron, shell/df, shell/ls)
- Others (pipeline, eks, context) may need service extraction
- **Action**: Audit each for service/trait/display separation

### [ ] N2: gh and jira services completed but undocumented
- `src/gh/service.rs` and `src/jira/service.rs` exist with trait-based APIs
- Both follow the correct pattern but were not in the original plan
- **Status**: Done — no action needed, noted for completeness

---

## Implementation Order (updated 2026-03-30)

Critical path complete (M3 → H2 → M5 → H5 → H7). Remaining work:

1. ~~**M3** (slack trait)~~ Done
2. ~~**H2** (slack service)~~ Done
3. ~~**M5** (OutputFormat)~~ Done
4. ~~**H5** (data service)~~ Done
5. ~~**H7** (MCP server)~~ Done
6. **M6** (install) — independent, extract display/service
7. **M4** (flatten exports) — finish re-exporting client/trait types
8. **M7** (split large files) — gh client, slack client
9. **M8** (handler naming) — standardize run vs run_command
10. **N1** (new modules) — audit pipeline, eks, context
11. **Low priority** — as time permits

---

## Target Architecture

### Before (original state)
```
CLI args -> cmd_handlers (fetch + print) -> stdout
```

### Current (MCP live, most modules done)
```
CLI args -> cmd_handler -> service::*(api) -> display::*() -> print
                                ^
MCP req --> mcp::server --> service::*(api) -> json response
                                |
                                | (14 modules have service.rs)
                                | (7 have API traits)
```

### After (target)
```
CLI args ---+
            +---> service.rs (returns data) ---> client (API calls)
MCP req ----+         |
            |         v
HTTP req ---+    display (formatting) <- only CLI uses this
```

### Module Structure Template
```
{module}/
  mod.rs           # Re-exports + command dispatch (thin)
  cli.rs           # clap args (CLI-only)
  types.rs         # Data structs
  config.rs        # Module config
  service.rs       # Business logic - RETURNS DATA, NEVER PRINTS
  client/          # API client
    mod.rs         # Implements trait
    tests.rs
  display/         # Output formatting (CLI-only)
    mod.rs
    tests.rs
```

### Interface Modules (Future)
```
src/
  mcp/             # hu mcp serve - MCP server (stdio)
    mod.rs         # JSON-RPC stdio handler
    tools.rs       # Tool registry (maps to service calls)
    types.rs       # MCP protocol types
    templates.rs   # Tool definitions as constants

  serve/           # hu serve - HTTP API
    mod.rs         # axum server setup
    routes.rs      # Route handlers (maps to service calls)
    types.rs       # API request/response types
```

### MCP Templates (like install/templates.rs)

MCP has three primitives - define all as constants in `src/mcp/templates.rs`:

| Primitive | Control | Use Case | Claude Code UI |
|-----------|---------|----------|----------------|
| **Tools** | Model-controlled | Actions (query, search, create) | Auto-invoked by model |
| **Prompts** | User-controlled | Structured instructions | `/mcp__hu__prompt_name` |
| **Resources** | Application-driven | Read-only data context | `@hu:resource://path` |

```rust
// src/mcp/templates.rs

// ============================================================================
// TOOLS - Model-controlled actions
// ============================================================================

pub struct Tool {
    pub name: &'static str,
    pub description: &'static str,
    pub input_schema: &'static str,
}

pub const PAGERDUTY_INCIDENTS: Tool = Tool {
    name: "pagerduty_incidents",
    description: "List PagerDuty incidents",
    input_schema: r#"{
        "type": "object",
        "properties": {
            "status": { "type": "string", "enum": ["triggered", "acknowledged", "resolved"] },
            "limit": { "type": "integer", "default": 25 }
        }
    }"#,
};

pub const SLACK_SEARCH: Tool = Tool {
    name: "slack_search",
    description: "Search Slack messages",
    input_schema: r#"{
        "type": "object",
        "properties": {
            "query": { "type": "string", "description": "Search query" },
            "count": { "type": "integer", "default": 20 }
        },
        "required": ["query"]
    }"#,
};

pub const SENTRY_ISSUES: Tool = Tool {
    name: "sentry_issues",
    description: "List Sentry issues",
    input_schema: r#"{
        "type": "object",
        "properties": {
            "project": { "type": "string" },
            "query": { "type": "string" },
            "limit": { "type": "integer", "default": 25 }
        }
    }"#,
};

pub const ALL_TOOLS: &[&Tool] = &[
    &PAGERDUTY_INCIDENTS,
    &SLACK_SEARCH,
    &SENTRY_ISSUES,
];

// ============================================================================
// PROMPTS - User-controlled structured messages (like slash commands)
// ============================================================================

pub struct PromptArg {
    pub name: &'static str,
    pub description: &'static str,
    pub required: bool,
}

pub struct Prompt {
    pub name: &'static str,
    pub description: &'static str,
    pub arguments: &'static [PromptArg],
}

pub const PROMPT_INCIDENT_RESPONSE: Prompt = Prompt {
    name: "incident_response",
    description: "Generate incident response checklist from PagerDuty alert",
    arguments: &[
        PromptArg { name: "incident_id", description: "PagerDuty incident ID", required: true },
    ],
};

pub const PROMPT_DAILY_STANDUP: Prompt = Prompt {
    name: "daily_standup",
    description: "Generate standup summary from recent activity",
    arguments: &[
        PromptArg { name: "days", description: "Days to look back (default: 1)", required: false },
    ],
};

pub const PROMPT_ERROR_ANALYSIS: Prompt = Prompt {
    name: "error_analysis",
    description: "Analyze Sentry error and suggest fixes",
    arguments: &[
        PromptArg { name: "issue_id", description: "Sentry issue ID", required: true },
    ],
};

pub const ALL_PROMPTS: &[&Prompt] = &[
    &PROMPT_INCIDENT_RESPONSE,
    &PROMPT_DAILY_STANDUP,
    &PROMPT_ERROR_ANALYSIS,
];

// ============================================================================
// RESOURCES - Read-only data context (@ mentions)
// ============================================================================

pub struct Resource {
    pub uri: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub mime_type: &'static str,
}

pub struct ResourceTemplate {
    pub uri_template: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub mime_type: &'static str,
}

// Static resources
pub const RESOURCE_CONFIG: Resource = Resource {
    uri: "hu://config",
    name: "Configuration",
    description: "Current hu CLI configuration",
    mime_type: "application/json",
};

pub const RESOURCE_STATS: Resource = Resource {
    uri: "hu://stats",
    name: "Usage Statistics",
    description: "Claude Code usage statistics",
    mime_type: "application/json",
};

// Dynamic resource templates
pub const TEMPLATE_SESSION: ResourceTemplate = ResourceTemplate {
    uri_template: "hu://session/{id}",
    name: "Session",
    description: "Claude Code session data",
    mime_type: "application/json",
};

pub const TEMPLATE_JIRA_TICKET: ResourceTemplate = ResourceTemplate {
    uri_template: "hu://jira/{key}",
    name: "Jira Ticket",
    description: "Jira ticket details",
    mime_type: "application/json",
};

pub const ALL_RESOURCES: &[&Resource] = &[
    &RESOURCE_CONFIG,
    &RESOURCE_STATS,
];

pub const ALL_RESOURCE_TEMPLATES: &[&ResourceTemplate] = &[
    &TEMPLATE_SESSION,
    &TEMPLATE_JIRA_TICKET,
];
```

### MCP Handler Pattern

```rust
// src/mcp/handlers.rs

// Tools - invoked by model via tools/call
pub async fn handle_tool_call(name: &str, args: Value) -> Result<ToolResult> {
    match name {
        "pagerduty_incidents" => {
            let client = pagerduty::Client::new()?;
            let data = pagerduty::service::list_incidents(&client, args.into()).await?;
            Ok(ToolResult::json(data))
        }
        "slack_search" => {
            let query = args["query"].as_str().ok_or(anyhow!("query required"))?;
            let count = args["count"].as_u64().unwrap_or(20) as usize;
            let client = slack::Client::new()?;
            let data = slack::service::search(&client, query, count).await?;
            Ok(ToolResult::json(data))
        }
        _ => Err(anyhow!("Unknown tool: {}", name)),
    }
}

// Prompts - invoked by user via prompts/get, returns messages for LLM
pub async fn handle_prompt_get(name: &str, args: Value) -> Result<PromptResult> {
    match name {
        "incident_response" => {
            let id = args["incident_id"].as_str().ok_or(anyhow!("incident_id required"))?;
            let client = pagerduty::Client::new()?;
            let incident = pagerduty::service::get_incident(&client, id).await?;
            Ok(PromptResult::messages(vec![
                Message::user(format!(
                    "Analyze this PagerDuty incident and create a response checklist:\n\n\
                     Title: {}\nStatus: {}\nService: {}\nCreated: {}\n\nDetails: {}",
                    incident.title, incident.status, incident.service,
                    incident.created_at, incident.description
                )),
            ]))
        }
        "daily_standup" => {
            let days = args["days"].as_u64().unwrap_or(1);
            let store = data::open_db()?;
            let stats = data::service::get_stats(&store, Some(days))?;
            Ok(PromptResult::messages(vec![
                Message::user(format!(
                    "Generate a standup summary from this activity:\n\n{}",
                    serde_json::to_string_pretty(&stats)?
                )),
            ]))
        }
        _ => Err(anyhow!("Unknown prompt: {}", name)),
    }
}

// Resources - read via resources/read, returns data
pub async fn handle_resource_read(uri: &str) -> Result<ResourceContents> {
    match uri {
        "hu://config" => {
            let config = util::config::load()?;
            Ok(ResourceContents::json(config))
        }
        "hu://stats" => {
            let store = data::open_db()?;
            let stats = data::service::get_stats(&store, None)?;
            Ok(ResourceContents::json(stats))
        }
        uri if uri.starts_with("hu://session/") => {
            let id = uri.strip_prefix("hu://session/").unwrap();
            let store = data::open_db()?;
            let session = data::service::get_session(&store, id)?;
            Ok(ResourceContents::json(session))
        }
        uri if uri.starts_with("hu://jira/") => {
            let key = uri.strip_prefix("hu://jira/").unwrap();
            let client = jira::Client::new()?;
            let ticket = jira::service::get_ticket(&client, key).await?;
            Ok(ResourceContents::json(ticket))
        }
        _ => Err(anyhow!("Unknown resource: {}", uri)),
    }
}
```

### Usage in Claude Code

```bash
# Start MCP server
hu mcp serve

# Add to Claude Code
claude mcp add hu --transport stdio -- hu mcp serve
```

Or in `.mcp.json` for project sharing:
```json
{
  "mcpServers": {
    "hu": { "command": "hu", "args": ["mcp", "serve"] }
  }
}
```

**Tools** (model auto-invokes):
```
> "Check if there are any open PagerDuty incidents"
# Model calls pagerduty_incidents tool automatically
```

**Prompts** (user invokes via slash command):
```
> /mcp__hu__incident_response P12345
# Returns structured prompt for incident analysis
```

**Resources** (user references via @ mention):
```
> Analyze my usage stats @hu:hu://stats
# Fetches resource and includes in context
```

### Call Flow Example
```
# CLI
hu pagerduty incidents --limit 5
  -> cli::parse() -> pagerduty::cmd_incidents()
    -> service::list_incidents(api, limit)
      -> display::output_incidents(data, Table)
        -> println!(table)

# MCP
{"method": "pagerduty/incidents", "params": {"limit": 5}}
  -> mcp::handle() -> mcp::tools::pagerduty_incidents()
    -> service::list_incidents(api, limit)
      -> json_rpc_response(data)

# HTTP
GET /api/pagerduty/incidents?limit=5
  -> serve::routes::get_incidents()
    -> service::list_incidents(api, limit)
      -> Json(data)
```

---

## Notes

### Cross-Cutting Concerns

- All service functions accept `&impl XxxApi` trait objects
- Service functions return `Result<T>` where T is typed data
- Display functions take data + `OutputFormat`, return `Result<String>` or print
- MCP handlers will call service directly, format as JSON-RPC response
- HTTP handlers will call service directly, format as JSON body

### Testing Strategy

After refactoring, each module should have:
- `service/tests.rs` - Unit tests with mock clients
- `display/tests.rs` - Output formatting tests (already exist)
- `client/tests.rs` - Request/response parsing tests (most exist)

### Future: lib.rs + Subcommands

Once services are extracted, create `src/lib.rs` exposing:
```rust
pub mod pagerduty;
pub mod slack;
pub mod sentry;
pub mod newrelic;
pub mod data;
pub mod gh;
pub mod jira;
```

Add interface subcommands to CLI:
```rust
// src/cli.rs
#[derive(Subcommand)]
pub enum Command {
    // ... existing commands ...

    /// Start MCP server (JSON-RPC over stdio)
    Mcp,

    /// Start HTTP API server
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },
}
```

This enables:
- `hu slack channels` - CLI with table output
- `hu mcp` - MCP server (JSON-RPC over stdio)
- `hu serve` - HTTP API server (JSON responses)
- External crates using `hu` as library

All three interfaces call the same `service::*` functions.
