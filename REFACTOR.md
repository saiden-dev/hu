# Refactoring Plan

Generated: 2026-02-07
Scope: Full codebase - enabling reuse for MCP server and HTTP API

## Summary

15 issues found across the codebase. The main architectural gap is that hu is a **binary-only project** with no library exports - all business logic is locked inside the CLI. Good patterns exist in `gh/` and `jira/` (trait-based clients) but other modules mix I/O with logic.

**Breakdown:** 5 High, 6 Medium, 4 Low priority

---

## High Priority

### [ ] Create lib.rs for library exports
- **Location**: `src/lib.rs` (new file)
- **Problem**: No public API - everything is private to the binary
- **Impact**: Cannot reuse any logic in MCP server or HTTP API
- **Action**:
  1. Create `src/lib.rs` with public module exports
  2. Re-export traits: `GithubApi`, `JiraApi`, `PagerDutyApi`, `SlackClient`
  3. Re-export types from each module's `types.rs`
  4. Keep `main.rs` as thin CLI wrapper calling lib functions

### [ ] Extract service layer from handlers that print
- **Location**: `src/slack/handlers.rs:61-334`, `src/sentry/mod.rs:117-175`, `src/newrelic/mod.rs:101-150`, `src/pagerduty/mod.rs:65-149`
- **Problem**: Command handlers fetch data AND print output - can't reuse for MCP/HTTP
- **Impact**: Every new interface requires duplicating business logic
- **Action**:
  1. Create `service.rs` in each module
  2. Move data-fetching logic to service functions that return typed data
  3. Keep handlers as thin wrappers: `service::get_data()` → `display::format()`

### [ ] Centralize duplicate utility functions
- **Location**: `time_ago()` in sentry/pagerduty/newrelic display modules, `truncate()` in 4+ locations
- **Problem**: Same functions duplicated across modules, diverging implementations
- **Impact**: Bug fixes require changes in multiple places
- **Action**:
  1. Create `src/util/fmt.rs`
  2. Move `time_ago()`, `truncate()`, color helpers
  3. Update all display modules to use `crate::util::fmt::*`

### [ ] Standardize configuration check pattern
- **Location**: `src/sentry/mod.rs:93-101`, `src/newrelic/mod.rs:77-85`, `src/pagerduty/mod.rs:40-48`
- **Problem**: Identical `check_configured()` logic duplicated in each module
- **Impact**: Inconsistent error messages, maintenance burden
- **Action**:
  1. Create trait `HasConfig { fn is_configured(&self) -> bool; }`
  2. Create `util::ensure_configured(client: &impl HasConfig) -> Result<()>`
  3. Implement trait for each client type

### [ ] Flatten public API - re-export types at module level
- **Location**: All service modules (sentry, slack, gh, jira, newrelic, pagerduty, data)
- **Problem**: Deep paths like `sentry::config::SentryConfig` - doubly redundant
- **Impact**: Verbose imports, poor library ergonomics
- **Action**:
  1. Rename types to simple names (`Config`, `Client`)
  2. Keep submodules private, re-export at module level
  3. Users get clean paths: `sentry::Config`, `gh::Client`

**Pattern:**
```rust
// sentry/mod.rs
mod config;   // private
mod client;   // private
mod types;    // private

pub use config::Config;           // sentry::Config
pub use client::{Client, Api};    // sentry::Client, sentry::Api (trait)
pub use types::{Issue, Event};    // sentry::Issue
```

**Types to rename and re-export:**

| Current Path | New Public Path |
|--------------|-----------------|
| `sentry::config::SentryConfig` | `sentry::Config` |
| `sentry::client::SentryClient` | `sentry::Client` |
| `slack::config::SlackConfig` | `slack::Config` |
| `slack::client::SlackClient` | `slack::Client` |
| `gh::client::GithubClient` | `gh::Client` |
| `gh::client::GithubApi` | `gh::Api` |
| `jira::client::JiraClient` | `jira::Client` |
| `jira::client::JiraApi` | `jira::Api` |
| `newrelic::config::NewRelicConfig` | `newrelic::Config` |
| `newrelic::client::NewRelicClient` | `newrelic::Client` |
| `pagerduty::config::PagerDutyConfig` | `pagerduty::Config` |
| `pagerduty::client::PagerDutyClient` | `pagerduty::Client` |
| `data::config::DataConfig` | `data::Config` |

---

## Medium Priority

### [ ] Split docs/service.rs - separate I/O from logic
- **Location**: `src/docs/service.rs:17-95` (663 lines)
- **Problem**: File I/O (`fs::write`) and git operations mixed with content formatting
- **Impact**: Cannot test document processing without filesystem
- **Action**:
  1. Extract `parse_frontmatter()`, `format_document()` as pure functions
  2. Create `io.rs` for file operations
  3. Service layer coordinates: fetch → format → io.write

### [ ] Split data/sync.rs - separate parsing from DB ops
- **Location**: `src/data/sync.rs:39-100` (671 lines)
- **Problem**: JSONL reading mixed with SQLite inserts
- **Impact**: Can't test parsing logic without database
- **Action**:
  1. Extract `parse_jsonl()` → returns `Vec<Entry>`
  2. Extract `sync_entries(conn, entries)` for DB operations
  3. Main function coordinates the pipeline

### [ ] Reduce nesting in slack handlers
- **Location**: `src/slack/handlers.rs:61-93` (cmd_auth), `src/slack/handlers.rs:273-325` (cmd_tidy)
- **Problem**: 4+ levels of if-let nesting makes logic hard to follow
- **Impact**: Difficult to test individual branches, error-prone modifications
- **Action**:
  1. Extract nested logic into small helper functions
  2. Use early returns to flatten structure
  3. Consider Result combinators over nested matches

### [ ] Remove print statements from business logic
- **Location**: `src/gh/failures/mod.rs:32,97` (`eprintln!`, `println!`)
- **Problem**: Progress output embedded in data processing
- **Impact**: MCP/HTTP can't suppress or redirect output
- **Action**:
  1. Return structured progress info instead of printing
  2. Let caller decide whether/how to display progress

### [ ] Create module-level service.rs files
- **Location**: `src/slack/`, `src/sentry/`, `src/newrelic/`, `src/pagerduty/`, `src/pipeline/`
- **Problem**: These modules lack dedicated service layer - logic lives in mod.rs
- **Impact**: Inconsistent structure, harder to find business logic
- **Action**:
  1. Create `service.rs` in each module
  2. Move `cmd_*` function bodies to service layer
  3. Keep `mod.rs` for dispatch only

### [ ] Standardize gh/runs display - use comfy_table
- **Location**: `src/gh/runs/mod.rs:139-192`
- **Problem**: Manual table drawing with `println!` instead of comfy_table
- **Impact**: Inconsistent table style, harder to maintain
- **Action**:
  1. Move to `display/mod.rs`
  2. Use `comfy_table` with `UTF8_FULL_CONDENSED` preset
  3. Match pattern used in other display modules

---

## Low Priority

### [ ] Consider trait-based status colors
- **Location**: `src/sentry/display/mod.rs:41-57`, `src/pagerduty/display/mod.rs:13-18`, `src/pipeline/display/mod.rs:12-30`
- **Problem**: Each module has its own `status_color()` with similar logic
- **Impact**: Minor duplication, acceptable if enums differ
- **Action**: Create `trait Colorable { fn color(&self) -> &str; }` if patterns converge

### [ ] Extract gh/runs table constants
- **Location**: `src/gh/runs/mod.rs:11-16`
- **Problem**: ANSI color codes defined locally
- **Impact**: Minor - could share with other modules
- **Action**: Move to `util/fmt.rs` as `pub const GREEN/YELLOW/RED/RESET`

### [ ] Document public API in lib.rs
- **Location**: `src/lib.rs` (after creation)
- **Problem**: No rustdoc for library consumers
- **Impact**: Harder for MCP/HTTP developers to use the library
- **Action**: Add `//!` module docs and `///` for public items

### [ ] Consider config-driven command dispatch
- **Location**: `src/main.rs:38-121`
- **Problem**: Large match statement for command routing
- **Impact**: Minor - current approach is clear enough
- **Action**: Only if command count grows significantly

---

## Notes

### Dependencies Between Items
1. **lib.rs must come first** - other refactors depend on having a library structure
2. **util/fmt.rs before display refactors** - centralize helpers before updating consumers
3. **service.rs extraction is independent per module** - can be done incrementally

### Good Patterns to Preserve
- `GithubApi`, `JiraApi` traits with `run_with_client()` pattern
- `display/mod.rs` + `display/tests.rs` separation
- `queries.rs` in data module returning typed data

### Test File Organization (enforce consistently)
| Location | Use For |
|----------|---------|
| Inline `#[cfg(test)]` | Testing private functions only |
| `tests/` mirroring `src/` | Module tests (preferred) |

```
src/sentry/display/mod.rs  →  tests/sentry/display.rs
src/gh/client/mod.rs       →  tests/gh/client.rs
src/data/sync.rs           →  tests/data/sync.rs
tests/cli.rs               # CLI integration tests
```

### Module Quality Ranking
| Module | Reusability | Notes |
|--------|-------------|-------|
| gh | Good | Trait-based, testable |
| jira | Good | Same pattern as gh |
| data | Good | queries.rs returns data, display is separate |
| read | Good | Has service.rs |
| context | Good | Has service/ subdirectory |
| slack | Needs work | handlers.rs mixes fetch + print |
| sentry | Needs work | mod.rs does everything |
| newrelic | Needs work | Same as sentry |
| pagerduty | Needs work | Same as sentry |
| docs | Needs work | I/O mixed with logic |

### Suggested Starting Order
1. `lib.rs` + `util/fmt.rs` (unblocks everything)
2. Pick one module (suggest: `sentry` - smallest) as template
3. Apply pattern to remaining modules
