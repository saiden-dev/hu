# Refactoring Plan: Interface-Agnostic Architecture

Generated: 2026-02-08 (updated from 2026-02-07)
Scope: All modules - enabling reuse across CLI, MCP, and HTTP interfaces

## Summary

**Goal**: Extract service layers so the same business logic powers CLI, MCP server, and HTTP API.

**Current state**: 11 modules with varying levels of separation. Most have `println!()` in orchestration code, coupling output to business logic.

| Pattern | Count | Modules |
|---------|-------|---------|
| Has API trait | 4 | pagerduty, gh, jira, web_search |
| Has service.rs | 3 | read, git, docs |
| Display separated | 9 | Most modules |
| Fully MCP-ready | 0 | None yet |

**Issues found**: 19 total (7 high, 8 medium, 4 low)

---

## High Priority

### [ ] H1: Extract service layer from pagerduty module
- **Location**: `src/pagerduty/mod.rs:65-149`
- **Problem**: `cmd_*` handlers create client, call API, format output, print - all in one
- **Impact**: Cannot reuse for MCP/HTTP without duplicating logic
- **Action**:
  1. Create `src/pagerduty/service.rs` with functions that return data:
     ```rust
     pub async fn list_oncalls(api: &impl PagerDutyApi, opts: OncallOptions) -> Result<Vec<Oncall>>
     pub async fn list_incidents(api: &impl PagerDutyApi, opts: IncidentOptions) -> Result<Vec<Incident>>
     pub async fn get_incident(api: &impl PagerDutyApi, id: &str) -> Result<Incident>
     ```
  2. Move `check_configured()` to service layer
  3. Keep `cmd_*` handlers as thin CLI wrappers that call service + display

### [ ] H2: Extract service layer from slack module
- **Location**: `src/slack/handlers.rs` (334 lines, 21 println!)
- **Problem**: Handlers mix business logic with output formatting
- **Impact**: Slack integration locked to CLI only
- **Action**:
  1. Create `src/slack/service.rs`:
     ```rust
     pub async fn list_channels(client: &impl SlackApi) -> Result<Vec<Channel>>
     pub async fn get_channel_info(client: &impl SlackApi, id: &str) -> Result<ChannelInfo>
     pub async fn send_message(client: &impl SlackApi, channel: &str, text: &str) -> Result<MessageResult>
     pub async fn tidy_channels(client: &impl SlackApi, user: &UserInfo, dry_run: bool) -> Result<TidyReport>
     ```
  2. Create `SlackApi` trait in `src/slack/client.rs`
  3. Move `verify_token()` to client module

### [ ] H3: Extract service layer from newrelic module
- **Location**: `src/newrelic/mod.rs:101-149`
- **Problem**: Same pattern - handlers do everything inline
- **Impact**: Cannot query NewRelic from MCP tools
- **Action**:
  1. Create `src/newrelic/service.rs`:
     ```rust
     pub async fn list_issues(client: &impl NewRelicApi, limit: usize) -> Result<Vec<Issue>>
     pub async fn list_incidents(client: &impl NewRelicApi, limit: usize) -> Result<Vec<Incident>>
     pub async fn run_nrql(client: &impl NewRelicApi, query: &str) -> Result<NrqlResults>
     ```
  2. Add `NewRelicApi` trait to client

### [ ] H4: Extract service layer from sentry module
- **Location**: `src/sentry/mod.rs:117-174`
- **Problem**: Handlers inline all logic
- **Impact**: Cannot use Sentry queries from MCP
- **Action**:
  1. Create `src/sentry/service.rs`:
     ```rust
     pub async fn list_issues(client: &impl SentryApi, opts: IssueOptions) -> Result<Vec<Issue>>
     pub async fn get_issue(client: &impl SentryApi, id: &str) -> Result<Issue>
     pub async fn list_events(client: &impl SentryApi, issue_id: &str, limit: usize) -> Result<Vec<Event>>
     ```
  2. Add `SentryApi` trait to client

### [ ] H5: Extract service layer from data module
- **Location**: `src/data/mod.rs` (473 lines)
- **Problem**: Large file with all handlers, mixes DB access with output
- **Impact**: Cannot expose data queries via MCP
- **Action**:
  1. Create `src/data/service.rs`:
     ```rust
     pub fn sync_data(store: &SqliteStore, claude_dir: &Path, force: bool) -> Result<SyncResult>
     pub fn get_sessions(store: &SqliteStore, project: Option<&str>, limit: i64) -> Result<Vec<Session>>
     pub fn get_stats(store: &SqliteStore, since: Option<i64>) -> Result<UsageStats>
     pub fn search_messages(store: &SqliteStore, query: &str, limit: i64) -> Result<Vec<SearchResult>>
     ```
  2. Move `scan_debug_errors()` to service (already returns data, good pattern)
  3. Keep `open_db()` and `ensure_synced()` as helpers in mod.rs

### [ ] H6: Fix read/service.rs - remove println!
- **Location**: `src/read/service.rs:23,37,42,45`
- **Problem**: Service layer has 4 `println!()` calls - defeats the purpose
- **Impact**: Cannot use read operations from MCP without stdout pollution
- **Action**:
  1. Change `run()` to return an enum:
     ```rust
     pub enum ReadOutput {
         LinesAround { lines: Vec<Line>, center: usize, total: usize },
         Diff(String),
         Interface(Vec<OutlineItem>),
         Outline(FileOutline),
         Content(String),
     }
     pub fn run(args: ReadArgs) -> Result<ReadOutput>
     ```
  2. Move formatting to CLI layer in mod.rs

### [ ] H7: Create MCP server module with templates
- **Location**: `src/mcp/` (new module)
- **Problem**: No MCP interface exists yet
- **Impact**: Cannot use hu tools from Claude Code or other MCP clients
- **Action**:
  1. Create module structure:
     ```
     src/mcp/
       mod.rs         # JSON-RPC stdio server, method dispatch
       cli.rs         # clap subcommands (serve, list)
       types.rs       # MCP protocol types (Tool, Prompt, Resource, etc.)
       templates.rs   # All definitions as constants (like install/templates.rs)
       handlers.rs    # tools/call, prompts/get, resources/read handlers
     ```
  2. Define all three MCP primitives in templates.rs:
     - **Tools**: `pagerduty_incidents`, `slack_search`, `sentry_issues`, etc.
     - **Prompts**: `incident_response`, `daily_standup`, `error_analysis`
     - **Resources**: `hu://config`, `hu://stats`, `hu://session/{id}`, `hu://jira/{key}`
  3. Implement JSON-RPC handlers for:
     - `tools/list`, `tools/call`
     - `prompts/list`, `prompts/get`
     - `resources/list`, `resources/read`, `resources/templates/list`
  4. Add CLI subcommand:
     ```rust
     #[derive(Subcommand)]
     pub enum McpCommand {
         /// Start MCP server (JSON-RPC over stdio)
         Serve,
         /// List available tools, prompts, resources
         List {
             #[arg(long)] tools: bool,
             #[arg(long)] prompts: bool,
             #[arg(long)] resources: bool,
         },
     }
     ```
  5. Register: `claude mcp add hu --transport stdio -- hu mcp serve`

---

## Medium Priority

### [ ] M1: Add trait to NewRelic client
- **Location**: `src/newrelic/client/mod.rs`
- **Problem**: No trait defined - cannot mock for testing or swap implementations
- **Impact**: Tests require real API calls or `#[cfg(not(tarpaulin_include))]`
- **Action**: Add `NewRelicApi` trait matching `PagerDutyApi` pattern

### [ ] M2: Add trait to Sentry client
- **Location**: `src/sentry/client.rs`
- **Problem**: No trait - same issue as NewRelic
- **Action**: Add `SentryApi` trait

### [ ] M3: Add trait to Slack client
- **Location**: `src/slack/client.rs` (389 lines)
- **Problem**: Large file, no trait for mockability
- **Action**: Extract `SlackApi` trait, consider splitting client into smaller files

### [ ] M4: Flatten module exports
- **Location**: All modules
- **Problem**: Current: `pagerduty::types::Incident`, should be: `pagerduty::Incident`
- **Impact**: Verbose imports, exposes internal structure
- **Action**: Add `pub use` statements in each `mod.rs`:
  ```rust
  // pagerduty/mod.rs
  pub use types::{Incident, Oncall, User, OutputFormat};
  pub use client::{Client, Api};
  ```

### [ ] M5: Consolidate OutputFormat types
- **Location**: `src/*/types.rs` (each module defines own OutputFormat)
- **Problem**: 6+ identical `OutputFormat { Json, Table }` definitions
- **Impact**: Duplication, inconsistency risk
- **Action**: Create `src/util/output.rs` with shared type:
  ```rust
  #[derive(Debug, Clone, Copy)]
  pub enum OutputFormat { Json, Table }
  ```

### [ ] M6: Extract install display logic
- **Location**: `src/install/mod.rs` (25 println!, 443 lines)
- **Problem**: Mixes installation logic with output, file too large
- **Impact**: Cannot test installation logic without capturing stdout
- **Action**:
  1. Create `src/install/display.rs` for `print_status_table()` and output
  2. Create `src/install/service.rs` with:
     ```rust
     pub fn check_statuses(components: &[Component], base: &Path) -> Vec<ComponentStatus>
     pub fn install_components(components: &[Component], base: &Path) -> Result<InstallReport>
     ```

### [ ] M7: Split large client files
- **Location**: `src/gh/client/mod.rs` (455 lines), `src/slack/client.rs` (389 lines)
- **Problem**: Files exceed 300-500 line limit
- **Impact**: Hard to navigate, test, maintain
- **Action**: Split by concern:
  - `gh/client/` → `mod.rs`, `runs.rs`, `prs.rs`, `checks.rs`
  - `slack/client/` → `mod.rs`, `channels.rs`, `messages.rs`, `search.rs`

### [ ] M8: Standardize handler naming
- **Location**: Various modules
- **Problem**: Inconsistent: `run()`, `run_command()`, `cmd_*()` patterns
- **Impact**: Confusion when navigating codebase
- **Action**: Standardize to:
  - `run(cmd: Command)` - public dispatcher in mod.rs
  - `service::*` - business logic functions
  - Keep `cmd_*` as private CLI-specific handlers if needed

---

## Low Priority

### [ ] L1: Remove redundant type prefixes
- **Location**: Various type definitions
- **Problem**: `SentryConfig` in `sentry::config`, `SlackConfig` in `slack::config`
- **Impact**: Verbose: `sentry::config::SentryConfig` vs `sentry::Config`
- **Action**: Rename to just `Config`, re-export at module level

### [ ] L2: Consolidate check_configured patterns
- **Location**: Each module has own `check_configured(&client)` function
- **Problem**: Duplicated error handling pattern
- **Action**: Consider trait method or shared helper:
  ```rust
  pub trait Configured {
      fn ensure_configured(&self) -> Result<()>;
  }
  ```

### [ ] L3: Extract common table formatting
- **Location**: Multiple `display/mod.rs` files
- **Problem**: Repeated `comfy_table` setup boilerplate
- **Action**: Create `src/util/table.rs`:
  ```rust
  pub fn new_table() -> Table {
      let mut t = Table::new();
      t.load_preset(UTF8_FULL_CONDENSED);
      t
  }
  ```

### [ ] L4: Document service layer pattern
- **Location**: `CLAUDE.md` or new `ARCHITECTURE.md`
- **Problem**: Pattern not documented for contributors
- **Action**: Add architecture section explaining:
  - Service layer returns data, never prints
  - Traits for all API clients
  - CLI handlers are thin wrappers

---

## Implementation Order

Suggested sequence for minimal disruption:

1. **H6** (read) - Small, isolated, proves the service pattern
2. **H1** (pagerduty) - Already has trait, closest to done
3. **M5** (OutputFormat) - Unblocks other refactors
4. **H3, H4** (newrelic, sentry) - Similar to pagerduty
5. **M1, M2** (traits) - Enable testing
6. **H7** (mcp) - Create MCP module with tool templates (depends on service layers)
7. **H2** (slack) - Larger but well-structured
8. **H5** (data) - Largest, most complex
9. **M6** (install) - Independent
10. **Medium/Low** - As time permits

---

## Target Architecture

### Before (current)
```
CLI args → cmd_handlers (fetch + print) → stdout
```

### After
```
CLI args ──┐
           ├──→ service.rs (returns data) ──→ client (API calls)
MCP req ───┤         │
           │         ↓
HTTP req ──┘    display (formatting) ← only CLI uses this
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
  → cli::parse() → pagerduty::cmd_incidents()
    → service::list_incidents(api, limit)
      → display::output_incidents(data, Table)
        → println!(table)

# MCP
{"method": "pagerduty/incidents", "params": {"limit": 5}}
  → mcp::handle() → mcp::tools::pagerduty_incidents()
    → service::list_incidents(api, limit)
      → json_rpc_response(data)

# HTTP
GET /api/pagerduty/incidents?limit=5
  → serve::routes::get_incidents()
    → service::list_incidents(api, limit)
      → Json(data)
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
