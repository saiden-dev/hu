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
    http.rs            # HTTP client setup
    table.rs           # Table formatting helpers
    config.rs          # Config loading
    output.rs          # Output format handling (--json, --table)
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
| Colored status badges | `util/table.rs` | `gh/display.rs` |
| Config file loading | `util/config.rs` | `jira/config.rs` |
| HTTP client with retries | `util/http.rs` | `jira/client.rs` |
| Progress spinners | `util/progress.rs` | `jira/tickets.rs` |
| Date/time formatting | `util/time.rs` | `gh/runs.rs` |

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

### Core Crates
- **clap** (derive) - CLI parsing
- **colored** - Terminal colors
- **comfy-table** - Table display
- **indicatif** - Progress spinners
- **anyhow** - Application errors
- **thiserror** - Library errors
- **serde** + **serde_json** - Serialization
- **tokio** - Async runtime (if needed)
- **tracing** - Structured logging

### Dev Dependencies
- **insta** - Snapshot testing
- **criterion** - Benchmarks
- **tempfile** - Test fixtures

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
- [ ] Unit tests inline with `#[cfg(test)]` modules
- [ ] Integration tests in `tests/` directory
- [ ] Snapshot tests for output formatting

**Tooling:**
- [ ] `clippy.toml` and `rustfmt.toml` configured
- [ ] CI runs fmt, clippy, and tests
