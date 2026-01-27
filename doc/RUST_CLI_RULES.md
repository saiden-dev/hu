# Rust CLI Project Rules

Best practices distilled from project analysis and refactoring experience.

**Style Philosophy:** When patterns diverge, prefer Ruby/Python idioms—readability over cleverness, flat over nested, explicit over implicit.

---

## 1. Project Structure

### File Size Limits
- **Maximum 400 lines per file** - Split larger files into modules
- **Maximum 50 lines per function** - Extract helpers for longer functions
- No file should contain more than 2-3 distinct responsibilities

### Scalable CLI Layout
Pattern: `hu <command> <subcommand>` (e.g., `hu jira list`, `hu gh prs`)

```
src/
  main.rs              # CLI entry, top-level dispatch only (~50 lines)
  lib.rs               # Re-exports all modules
  cli.rs               # Top-level CLI struct with #[command(flatten)]
  errors.rs            # Shared error types

  # Each command is a self-contained module
  jira/
    mod.rs             # pub use, Jira enum with subcommands
    cli.rs             # #[derive(Subcommand)] enum JiraCommand
    list.rs            # `hu jira list` handler
    show.rs            # `hu jira show` handler
    types.rs           # Jira-specific types
    client.rs          # API client

  gh/
    mod.rs
    cli.rs             # #[derive(Subcommand)] enum GhCommand
    prs.rs             # `hu gh prs` handler
    runs.rs            # `hu gh runs` handler
    types.rs
    client.rs

  # Utilities (name by purpose, not "shared")
  util/
    mod.rs
    http.rs            # HTTP client setup, retries
    config.rs          # Config loading/saving
    output.rs          # Output format handling (--json, --table)
    fmt.rs             # Humanization: time_ago, duration, bytes, inflection
    ui/
      mod.rs           # UI re-exports
      table.rs         # Ratatui table helpers
      progress.rs      # Progress bar/spinner helpers
      status.rs        # Status badges, icons, styles
```

### Implementation Workflow

**Never assume a command will be simple.** Always follow this order:

1. **Base infrastructure first** — Create the foundation before any handler
2. **Then subcommand handler** — One file per subcommand, using the base

**For a new command module** (`hu slack`):
```
Step 1: src/slack/mod.rs      # Module definition
Step 2: src/slack/types.rs    # Data structures
Step 3: src/slack/client.rs   # API client (if needed)
Step 4: src/slack/config.rs   # Config loading (if needed)
Step 5: src/slack/display.rs  # Output formatting (if needed)
Step 6: src/slack/list.rs     # First subcommand handler
```

**For a new subcommand** in existing module (`hu jira sprint`):
```
Step 1: src/jira/sprint.rs    # Handler file
Step 2: Update src/jira/mod.rs and CLI enum
```

**Why base-first:**
- Forces you to understand the API/domain before writing handlers
- Base files are reused by all subcommands — get them right first
- Handlers become thin and focused when infrastructure exists
- Avoids refactoring handlers later to extract shared code

**Reusable patterns → `util/` first:**

If implementing something that looks reusable, **start in `util/`**, not in the command module:

| Feature | Put in `util/` | NOT in |
|---------|----------------|--------|
| `--json` / `--table` output flag | `util/output.rs` | `jira/display.rs` |
| Tables (ratatui) | `util/ui/table.rs` | `gh/display.rs` |
| Colored status badges | `util/ui/status.rs` | `gh/display.rs` |
| Progress spinners | `util/ui/progress.rs` | `jira/tickets.rs` |
| Time ago, durations | `util/fmt.rs` | `gh/runs.rs` |
| Byte size formatting | `util/fmt.rs` | `jira/tickets.rs` |
| Pluralize/inflection | `util/fmt.rs` | `jira/display.rs` |
| Config file loading | `util/config.rs` | `jira/config.rs` |
| HTTP client with retries | `util/http.rs` | `jira/client.rs` |

**Rule:** When in doubt, put it in `util/`. Moving from `util/` to module-specific is easy; extracting from module to `util/` later is a refactor.

### Adding a New Command Module

To add `hu slack <subcommand>`:

1. Create `src/slack/mod.rs`:
```rust
mod cli;
mod list;
mod send;

pub use cli::SlackCommand;
```

2. Create `src/slack/cli.rs`:
```rust
use clap::Subcommand;

#[derive(Subcommand)]
pub enum SlackCommand {
    /// List channels
    List,
    /// Send a message
    Send { channel: String, message: String },
}

impl SlackCommand {
    pub async fn run(self) -> anyhow::Result<()> {
        match self {
            Self::List => list::run().await,
            Self::Send { channel, message } => send::run(&channel, &message).await,
        }
    }
}
```

3. Add to `src/cli.rs`:
```rust
use crate::slack::SlackCommand;

#[derive(Subcommand)]
pub enum Command {
    /// Jira operations
    Jira {
        #[command(subcommand)]
        cmd: JiraCommand,
    },
    /// GitHub operations
    Gh {
        #[command(subcommand)]
        cmd: GhCommand,
    },
    /// Slack operations  // <- ADD
    Slack {
        #[command(subcommand)]
        cmd: SlackCommand,
    },
}
```

4. Add match arm in `main.rs`:
```rust
Command::Slack { cmd } => cmd.run().await,
```

### Module Isolation Rules
- Each command module owns its CLI definition, types, and handlers
- Modules only import from `util/` and standard library
- No cross-imports between command modules (jira/ never imports from gh/)
- If two modules need the same code, extract to `util/`

### Internal Module Structure
Separate base infrastructure from subcommand handlers:

```
jira/
  # Base infrastructure (shared within module)
  mod.rs             # Re-exports, CLI enum definition
  client.rs          # API client, HTTP calls
  config.rs          # Module-specific config loading
  types.rs           # Data structures, API responses
  display.rs         # Table formatting, output helpers
  auth.rs            # Authentication flow

  # Subcommand handlers (one file per subcommand)
  sprint.rs          # `hu jira sprint` → uses client, types, display
  tickets.rs         # `hu jira tickets` → uses client, types, display

gh/
  mod.rs
  client.rs          # GitHub API client
  types.rs

  # Handlers
  prs.rs             # `hu gh prs`
  runs.rs            # `hu gh runs`
  failures.rs        # `hu gh failures`

git/
  mod.rs

  # Handlers (no client needed - shells out to git)
  branch.rs          # `hu git branch`
  commit.rs          # `hu git commit`
```

**Base files** (infrastructure):
| File | Purpose |
|------|---------|
| `client.rs` | API client, HTTP requests |
| `config.rs` | Load/save module config |
| `types.rs` | Structs for API responses |
| `display.rs` | Format output, tables |
| `auth.rs` | OAuth, tokens, login flow |

**Handler files** (one per subcommand):
| File | Purpose |
|------|---------|
| `{subcommand}.rs` | Single handler, imports base files |

**Handler file pattern:**
```rust
// src/jira/sprint.rs
use super::{client::JiraClient, display, types::Sprint};

pub async fn run(client: &JiraClient) -> anyhow::Result<()> {
    let sprints = client.get_sprints().await?;
    display::print_sprints(&sprints);
    Ok(())
}
```

**When to split further:**
- Handler file > 200 lines → extract helpers to `{subcommand}/mod.rs` + subfiles
- Shared logic between 2+ handlers → extract to base file or new `helpers.rs`

### Module Organization
- Group by command, not by type
- Each module is self-contained and independently testable
- Use `mod.rs` for clean re-exports
- Keep `main.rs` minimal—just CLI parsing and dispatch

---

## 2. Style Conventions (Ruby/Python Influence)

### Naming Patterns
```rust
// Predicates: use is_, has_, can_ prefixes (Python style)
fn is_empty(&self) -> bool
fn has_permissions(&self) -> bool
fn can_connect(&self) -> bool

// NOT: empty(), permissions(), connectable()

// Mutating methods: use imperative verbs
fn clear(&mut self)        // not reset_to_empty
fn save(&mut self)         // not persist_to_disk
fn update(&mut self)       // not apply_changes

// Constructors: prefer new(), from_*, parse_*
fn new() -> Self
fn from_path(p: &Path) -> Result<Self>
fn parse(s: &str) -> Result<Self>
```

### Flat Over Nested
```rust
// BAD - deeply nested
fn process(data: Option<Vec<Item>>) -> Result<()> {
    if let Some(items) = data {
        if !items.is_empty() {
            for item in items {
                if item.is_valid() {
                    // finally do something
                }
            }
        }
    }
    Ok(())
}

// GOOD - early returns, flat structure (Python style)
fn process(data: Option<Vec<Item>>) -> Result<()> {
    let items = match data {
        Some(v) if !v.is_empty() => v,
        _ => return Ok(()),
    };

    for item in items.iter().filter(|i| i.is_valid()) {
        // do something
    }
    Ok(())
}
```

### Iterators Over Loops
Prefer functional iterator chains (Ruby/Python comprehension style):
```rust
// BAD - imperative loop
let mut results = Vec::new();
for item in items {
    if item.is_active() {
        results.push(item.name.clone());
    }
}

// GOOD - iterator chain
let results: Vec<_> = items
    .iter()
    .filter(|i| i.is_active())
    .map(|i| i.name.clone())
    .collect();
```

### Method Chaining
Build fluent APIs where appropriate (Ruby style):
```rust
// Builder pattern
let config = Config::new()
    .with_timeout(30)
    .with_retries(3)
    .build()?;

// NOT
let mut config = Config::new();
config.set_timeout(30);
config.set_retries(3);
let config = config.build()?;
```

### Explicit Over Implicit
```rust
// BAD - implicit behavior
fn fetch(url: &str) -> Data  // What if it fails? Panics?

// GOOD - explicit about what can happen
fn fetch(url: &str) -> Result<Data, FetchError>

// BAD - magic boolean
process(data, true, false)

// GOOD - named parameters via struct or enum
process(data, Mode::Async, Validate::Skip)
```

### Sensible Defaults (Convention Over Configuration)
```rust
// Provide good defaults, allow override
#[derive(Default)]
pub struct Options {
    pub timeout: Option<Duration>,  // None = use default
    pub retries: u32,               // Default via Default trait
}

impl Options {
    pub fn timeout_or_default(&self) -> Duration {
        self.timeout.unwrap_or(Duration::from_secs(30))
    }
}
```

### Debug-Friendly Types
Always derive Debug, implement Display for user-facing types:
```rust
#[derive(Debug, Clone)]  // Always derive Debug
pub struct Pod {
    pub name: String,
    pub status: Status,
}

impl std::fmt::Display for Pod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name, self.status)
    }
}
```

---

## 3. Code Quality Rules

### No Magic Numbers
```rust
// BAD
if s.len() > 55 { ... }
Duration::from_millis(100)

// GOOD
const DISPLAY_TITLE_MAX_LEN: usize = 55;
const POLL_INTERVAL_MS: u64 = 100;
```

### No Hardcoded URLs
```rust
// Centralize API base URLs
const GITHUB_API_BASE: &str = "https://api.github.com";
const JIRA_AUTH_URL: &str = "https://auth.atlassian.com";
```

### Function Complexity
- Avoid nesting deeper than 3 levels
- Use early returns to reduce nesting
- Extract nested logic to helper functions

### Parameter Limits
- Maximum 4-5 parameters per function
- Use parameter structs for related data:
```rust
struct Context<'a> {
    env: &'a str,
    namespace: &'a str,
    profile: Option<&'a str>,
}
```

---

## 4. Error Handling

### Use Result Consistently
```rust
// BAD - loses error context
fn run_cmd(cmd: &str) -> Option<String>

// GOOD - preserves error info
fn run_cmd(cmd: &str) -> Result<String>
```

### Custom Error Types
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("API request failed: {0}")]
    Api(#[from] reqwest::Error),

    #[error("Config not found: {path}")]
    ConfigNotFound { path: PathBuf },
}
```

### No Silent Failures
- Always propagate errors with `?`
- Log errors with context before handling
- Never return empty collections on error without logging

---

## 5. Avoid Duplication

### Common Patterns to Extract

**Table Creation:**
```rust
pub fn create_table(headers: &[(&str, Color)]) -> Table {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL)
         .apply_modifier(UTF8_ROUND_CORNERS);
    // ... set headers
    table
}
```

**String Truncation:**
```rust
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    } else {
        s.to_string()
    }
}
```

**Config Loading:**
```rust
fn load_json_config<T: Default + DeserializeOwned>(path: &Path) -> Result<T>
fn save_json_config<T: Serialize>(path: &Path, config: &T) -> Result<()>
```

**Status Icons:**
```rust
fn status_icon(status: &str, conclusion: Option<&str>) -> ColoredString
```

---

## 6. Testing

### Write Testable Code

**"Hard to test" or "impossible to test" is not acceptable.** Design for testability from the start.

**Separate logic from side effects:**
```rust
// BAD - logic mixed with I/O, untestable
fn save_config(config: &Config) -> Result<()> {
    let json = serde_json::to_string(config)?;
    std::fs::write("config.json", json)?;  // Can't test without FS
    Ok(())
}

// GOOD - logic separated, I/O at the boundary
fn serialize_config(config: &Config) -> Result<String> {
    serde_json::to_string_pretty(config).map_err(Into::into)
}

fn save_config(config: &Config, path: &Path) -> Result<()> {
    let json = serialize_config(config)?;  // Test this
    std::fs::write(path, json)?;           // Don't test this
    Ok(())
}
```

**What to test vs what to mock:**

| Test the logic | Mock/stub the boundary |
|----------------|------------------------|
| JSON/data serialization | File system writes |
| Request building | Network calls |
| Response parsing | HTTP client |
| Query construction | Database calls |
| Business rules | External APIs |
| Data transformation | System calls |

**Use traits for external dependencies:**
```rust
// Define trait for external dependency
pub trait JiraApi {
    async fn get_tickets(&self) -> Result<Vec<Ticket>>;
}

// Real implementation
pub struct JiraClient { /* ... */ }
impl JiraApi for JiraClient {
    async fn get_tickets(&self) -> Result<Vec<Ticket>> {
        // actual HTTP call
    }
}

// Handler accepts trait, not concrete type
pub async fn list_tickets(api: &impl JiraApi) -> Result<()> {
    let tickets = api.get_tickets().await?;
    // process tickets...
    Ok(())
}

// In tests: mock implementation
#[cfg(test)]
mod tests {
    struct MockJira { tickets: Vec<Ticket> }
    impl JiraApi for MockJira {
        async fn get_tickets(&self) -> Result<Vec<Ticket>> {
            Ok(self.tickets.clone())
        }
    }

    #[test]
    fn test_list_tickets() {
        let mock = MockJira { tickets: vec![/* test data */] };
        assert!(list_tickets(&mock).await.is_ok());
    }
}
```

**Test expectations on arguments:**
```rust
// Verify the request is built correctly, don't send it
#[test]
fn test_build_jira_request() {
    let req = build_ticket_request("PROJ-123", &options);

    assert_eq!(req.url(), "https://jira.example.com/rest/api/3/issue/PROJ-123");
    assert_eq!(req.method(), "GET");
    assert!(req.headers().contains_key("Authorization"));
}
```

**Rules:**
- Never write code that "can't be tested"
- If something is hard to test, refactor to make it testable
- Push I/O to the edges, keep core logic pure
- Accept traits/interfaces, not concrete implementations
- Test the logic, mock the boundaries

### Test Location (Rust Convention)
- **Unit tests**: Inline `#[cfg(test)]` modules (can test private functions)
- **Integration tests**: `tests/` directory (tests public API only)
- **Snapshot tests**: `tests/` with `insta`

```rust
// src/parser.rs
fn parse_internal(s: &str) -> Result<Token> { ... }  // private helper

pub fn parse(s: &str) -> Result<Ast> {
    let token = parse_internal(s)?;
    // ...
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_internal() {
        // Can test private function - this is the point of inline tests
        assert!(parse_internal("valid").is_ok());
    }

    #[test]
    fn test_parse() {
        assert!(parse("input").is_ok());
    }
}
```

```rust
// tests/integration.rs - tests public API only
use hu::parse;

#[test]
fn test_parse_end_to_end() {
    // Can only access pub items from hu crate
    assert!(parse("input").is_ok());
}
```

### Test Directory Structure
```
src/
  parser.rs          # Contains #[cfg(test)] mod tests inline
  config.rs          # Contains #[cfg(test)] mod tests inline

tests/
  integration.rs     # Integration tests (public API)
  cli.rs             # CLI end-to-end tests
  fixtures/
    sample.json      # Test data files
  snapshots/         # insta snapshot files (auto-generated)

benches/
  parser_bench.rs    # Benchmarks with criterion
```

### Test Types
- **Unit tests** - Inline `#[cfg(test)]`, test private + public functions
- **Integration tests** - `tests/` directory, test public API, CLI commands
- **Snapshot tests** - Use `insta` for output formatting
- **Benchmarks** - Use `criterion` in `benches/`

### Before Refactoring
1. Ensure existing tests pass
2. Add tests for functions being extracted
3. Consider property-based tests for parsing

### Test Commands
```bash
just check      # fmt + clippy
just test       # run all tests
cargo insta review  # review snapshot changes
```

---

## 7. Dependencies

### Prefer Established Crates

**Don't reinvent the wheel.** Use mature, maintained, popular crates over custom implementations.

| Need | Use crate | Don't implement |
|------|-----------|-----------------|
| HTTP client | `reqwest` | Custom HTTP handling |
| JSON parsing | `serde_json` | Manual parsing |
| CLI parsing | `clap` | Custom arg parsing |
| Date/time | `chrono` | Manual date math |
| Regex | `regex` | Custom pattern matching |
| UUID | `uuid` | Custom ID generation |
| Base64 | `base64` | Manual encoding |
| URL parsing | `url` | String manipulation |
| Retries | `backoff`, `tokio-retry` | Custom retry loops |
| Rate limiting | `governor` | Manual throttling |

**Selection criteria:**
- Downloads: >100k/month on crates.io
- Maintenance: Updated within last 6 months
- Ecosystem: Used by other popular crates
- Documentation: Has examples and API docs

**When to implement custom:**
- Trivial one-liner (don't add dep for 3 lines of code)
- Domain-specific logic unique to your app
- When existing crates don't fit and wrapping is harder than implementing

### Ask Before Adding Dependencies

**Always ask the user** before adding new crates. Present choices:

**When feature is simple:**
> "This feature is simple. We only need one method from `chrono`.
> Do you want me to:
> 1. Add `chrono` (recommended - handles edge cases)
> 2. Implement manually (~10 lines)"

**When multiple solutions exist:**
> "There are multiple crates for HTTP retries:
> 1. `backoff` - simple, sync-focused
> 2. `tokio-retry` - async-native, minimal
> 3. `reqwest-retry` - reqwest middleware
>
> Which do you prefer?"

**When crate seems heavy:**
> "For UUID generation, options are:
> 1. `uuid` crate (full UUID support, 50kb)
> 2. Manual with `rand` (v4 only, already have rand)
>
> We only need v4 UUIDs. Preference?"

**Don't silently add dependencies** — the user should know what's being added and why.

### Core Crates
- **clap** (derive) - CLI parsing
- **ratatui** - Terminal UI (tables, progress, colors, layouts)
- **crossterm** - Terminal backend for ratatui
- **anyhow** - Application errors
- **thiserror** - Library errors
- **serde** + **serde_json** - Serialization
- **tokio** - Async runtime
- **tracing** - Structured logging

### Humanization Crates (ActiveSupport-like)
- **timeago** - "2 hours ago", "in 3 days"
- **humantime** - Duration ↔ "1h 30m 32s" (parse & format)
- **Inflector** - pluralize, singularize, case conversion
- **humansize** - Bytes → "1.43 MiB"

Put wrappers in `util/fmt.rs`:

```rust
// util/fmt.rs - Humanization helpers
use std::time::Duration;
use timeago::Formatter;
use humantime::format_duration;
use inflector::Inflector;
use humansize::{format_size, BINARY};

/// "2 hours ago", "in 3 days"
pub fn time_ago(duration: Duration) -> String {
    Formatter::new().convert(duration)
}

/// "1h 30m 32s"
pub fn duration(d: Duration) -> String {
    format_duration(d).to_string()
}

/// 1500000 → "1.43 MiB"
pub fn bytes(size: u64) -> String {
    format_size(size, BINARY)
}

/// "user" → "users"
pub fn pluralize(s: &str) -> String {
    s.to_plural()
}

/// "posts" → "post"
pub fn singularize(s: &str) -> String {
    s.to_singular()
}

/// 1 → "1st", 2 → "2nd"
pub fn ordinalize(n: i64) -> String {
    n.ordinalize()
}
```

Usage in handlers:
```rust
use crate::util::fmt;

println!("Updated {}", fmt::time_ago(last_modified));
println!("Size: {}", fmt::bytes(file_size));
println!("Found {} {}", count, fmt::pluralize("ticket"));
```

### API Client Crates

Use established client libraries instead of raw HTTP:

| Service | Crate | Notes |
|---------|-------|-------|
| **GitHub** | `octocrab` | Typed API, extensible, 1.2k+ stars |
| **Jira** | `gouqi` | Async, V3 API, ADF support |
| **Slack** | `slack-rust` | SocketMode, Event API, Web API |
| **AWS** | `aws-sdk-*` | Official SDK, one crate per service |

```toml
# API Clients
octocrab = "0.44"
gouqi = "0.10"
slack-rust = "0.1"

# AWS (add only what you need)
aws-config = "1"
aws-sdk-eks = "1"
aws-sdk-codepipeline = "1"
aws-sdk-ec2 = "1"
aws-sdk-s3 = "1"
```

**AWS SDK pattern:**
```rust
use aws_config::BehaviorVersion;
use aws_sdk_eks::Client as EksClient;

let config = aws_config::defaults(BehaviorVersion::latest())
    .profile_name("my-profile")
    .load()
    .await;

let eks = EksClient::new(&config);
let clusters = eks.list_clusters().send().await?;
```

### Dev Dependencies
- **insta** - Snapshot testing
- **criterion** - Benchmarks
- **tempfile** - Test fixtures

### UI with Ratatui

Ratatui provides unified UI components. Put wrappers in `util/ui/`:

```
util/
  ui/
    mod.rs           # Re-exports
    table.rs         # Table helpers
    progress.rs      # Progress bar/spinner helpers
    status.rs        # Status badges, icons
    prompt.rs        # User prompts (pair with dialoguer)
```

**Table pattern:**
```rust
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Table, Row, Cell},
};

pub fn tickets_table(tickets: &[Ticket]) -> Table<'_> {
    let header = Row::new(vec!["Key", "Summary", "Status"])
        .style(Style::default().bold());

    let rows = tickets.iter().map(|t| {
        Row::new(vec![
            Cell::from(t.key.as_str()),
            Cell::from(t.summary.as_str()),
            Cell::from(t.status.as_str()).style(status_style(&t.status)),
        ])
    });

    Table::new(rows, [Constraint::Length(12), Constraint::Min(30), Constraint::Length(15)])
        .header(header)
        .block(Block::default().borders(Borders::ALL).title("Tickets"))
}
```

**Progress pattern:**
```rust
use ratatui::widgets::{Gauge, Block, Borders};

pub fn progress_gauge(percent: u16, label: &str) -> Gauge<'_> {
    Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(label))
        .gauge_style(Style::default().fg(Color::Cyan))
        .percent(percent)
}
```

**Status badge pattern:**
```rust
pub fn status_style(status: &str) -> Style {
    match status {
        "success" | "done" => Style::default().fg(Color::Green),
        "failure" | "error" => Style::default().fg(Color::Red),
        "pending" | "in_progress" => Style::default().fg(Color::Yellow),
        _ => Style::default(),
    }
}

pub fn status_icon(status: &str) -> &'static str {
    match status {
        "success" | "done" => "✓",
        "failure" | "error" => "✗",
        "pending" => "○",
        "in_progress" => "◐",
        _ => "•",
    }
}
```

**Simple output (non-interactive):**
```rust
use std::io::{self, stdout};
use ratatui::{prelude::*, Terminal};
use crossterm::{execute, terminal::*};

pub fn print_table(table: Table) -> io::Result<()> {
    // For non-interactive output, render once to stdout
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.draw(|frame| {
        frame.render_widget(table, frame.area());
    })?;
    Ok(())
}
```

**When to use ratatui vs plain print:**
| Use ratatui | Use `println!` |
|-------------|----------------|
| Tables | Simple single-line output |
| Progress bars | Error messages |
| Colored status | Debug info |
| Interactive TUI | JSON output (`--json`) |

---

## 8. Configuration

### Linting Setup
Create `clippy.toml`:
```toml
cognitive-complexity-threshold = 15
too-many-arguments-threshold = 5
```

Create `rustfmt.toml`:
```toml
edition = "2021"
max_width = 100
```

### CI Setup
- Run `cargo fmt --check`
- Run `cargo clippy -- -D warnings`
- Run `cargo test`
- Consider `cargo deny` for dependency auditing

---

## 9. Documentation

### Required Docs
- Complex algorithms
- Public API functions
- Non-obvious behavior
- ARN/URL format expectations

### Variable Naming
```rust
// BAD
let s = parse_timestamp(input);
let cli_profile = args.profile;

// GOOD
let timestamp_str = parse_timestamp(input);
let aws_profile_override = args.profile;
```

---

## 10. Architecture Principles

### Separation of Concerns
- CLI parsing in `main.rs`
- Business logic in feature modules
- Display/formatting separate from data fetching

### Dependency Direction
- Commands depend on services
- Services depend on types
- Types depend on nothing

### Consider Service Layer
```rust
pub struct EksService { /* ... */ }
impl EksService {
    pub async fn connect(&self, env: &str, pod: usize) -> Result<()>;
    pub async fn tail_logs(&self, env: &str, pattern: &str) -> Result<()>;
}
```

---

## 11. Metrics Targets

| Metric | Target |
|--------|--------|
| Max file size | <400 lines |
| Max function size | <50 lines |
| Max nesting depth | 3 levels |
| Max parameters | 5 |
| Magic numbers | 0 (use constants) |
| Duplicate code blocks | <3 patterns |

---

## Quick Checklist

**Structure:**
- [ ] Files under 400 lines
- [ ] Functions under 50 lines
- [ ] Nesting depth ≤ 3 levels

**Style (Ruby/Python):**
- [ ] Predicates use `is_`, `has_`, `can_` prefixes
- [ ] Iterator chains over imperative loops
- [ ] Early returns to flatten logic
- [ ] Builder pattern for complex construction
- [ ] All types derive `Debug`

**Quality:**
- [ ] No magic numbers (use constants)
- [ ] No hardcoded URLs
- [ ] All errors propagated with context
- [ ] Common patterns extracted to helpers

**Testing:**
- [ ] Code is testable (logic separated from I/O)
- [ ] External deps use traits (mockable)
- [ ] Unit tests inline with `#[cfg(test)]` modules
- [ ] Integration tests in `tests/` directory
- [ ] No "hard to test" code accepted

**Tooling:**
- [ ] `clippy.toml` and `rustfmt.toml` configured
- [ ] CI runs fmt, clippy, and tests
