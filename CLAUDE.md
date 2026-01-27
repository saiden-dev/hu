# hu CLI

**Full rules: `doc/rules/` — READ BEFORE IMPLEMENTING**

Rules split by topic:
- `structure.md` - Project layout, module organization
- `style.md` - Naming, Ruby/Python conventions
- `quality.md` - Error handling, duplication
- `testing.md` - Testability, test structure
- `dependencies.md` - Crate selection, API clients
- `architecture.md` - Interface-agnostic design
- `output.md` - UI/output with ratatui
- `checklist.md` - Quick reference

## Critical Rules (Must Follow)

### Implementation Workflow
1. **Never assume simple** — always follow base-first approach
2. **Base infrastructure first** → then subcommand handlers
3. **util/ first** for anything reusable (don't implement in module)

### Present Interactive Choices For:
- **Adding dependencies** — show options with trade-offs
- **Architecture decisions** — multiple valid approaches
- **Config format** — TOML vs JSON vs YAML
- **API design** — when multiple patterns fit
- **Breaking changes** — confirm before modifying public API

Example:
```
Which HTTP client approach?
1. reqwest (recommended) - full-featured, async
2. ureq - blocking, minimal deps
3. Custom with hyper - more control
```

### Architecture
- **Interface-agnostic logic**: Services return data, cli/api format it
- **Reusable for CLI, REST API, MCP** — if can't add REST endpoint easily, refactor
- **Dependency direction**: Interfaces → Services → Clients → Types

### Structure
```
src/
  main.rs              # CLI entry only (~50 lines)
  lib.rs               # Exports
  cli.rs               # Top-level clap Commands enum
  util/
    fmt.rs             # Humanization (timeago, humantime, Inflector, humansize)
    http.rs            # HTTP client setup
    config.rs          # Config loading
    ui/                # Ratatui helpers
      table.rs
      progress.rs
      status.rs
  {module}/            # e.g., jira/, gh/, slack/
    mod.rs             # Re-exports + CLI subcommand enum
    service.rs         # Business logic (NO UI, returns data)
    client.rs          # API calls
    types.rs           # Data structs
    {subcommand}.rs    # Handler (thin, uses service)
```

### Style (Ruby/Python influence)
- `is_`, `has_`, `can_` predicates
- Iterators over loops
- Early returns, flat structure
- Builder pattern for complex construction
- All types: `#[derive(Debug)]`

### Testing (Critical)
- **100% coverage required** — run `cargo tarpaulin`
- **"Hard to test" is NOT acceptable** — design for testability
- **Separate logic from I/O** — test logic, mock boundaries
- **Traits for external deps** — mockable

**Test locations (Rust idiom):**
- **Unit tests**: inline `#[cfg(test)]` — parsing logic, internal functions
- **Integration tests**: `tests/` — binary behavior via `std::process::Command`

**CLI integration tests pattern:**
```rust
// tests/cli.rs
fn hu() -> Command {
    Command::new(env!("CARGO_BIN_EXE_hu"))
}

#[test]
fn subcommand_shows_help() {
    let output = hu().arg("jira").output().unwrap();
    assert!(output.status.success());  // exit 0, not 2
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Jira operations"));  // stdout, not stderr
}
```

**CLI behavior rules:**
- Help on no args/subcommand → exit 0, stdout (not stderr)
- Mark unreachable code with `unreachable!()` (don't leave dead `Ok(())`)

### Output (Pretty by Default)
- **ratatui** for tables, progress, status
- **tui-markdown** for markdown rendering
- Colors: green=success, yellow=progress, red=error, cyan=info
- Icons: ✓ ◐ ○ ✗ ⚠ ⊘
- No plain `println!` for user-facing output

### Dependencies
- Ask user before adding
- Prefer established crates (see doc for full list)
- API clients: octocrab(gh), gouqi(jira), slack-rust, aws-sdk-*, reqwest(sentry/pagerduty), graphql_client(newrelic)

## Commands
```bash
just check    # fmt + clippy (MUST PASS)
just test     # tests (MUST PASS)
just build    # debug build
just install  # cargo install
```

## AWS Safety
- **READ-ONLY operations only**
- **`-e dev` only** for EKS testing
- Never start/stop/modify anything
