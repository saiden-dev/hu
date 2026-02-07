# hu CLI

## Commands
```bash
just check    # fmt + clippy (MUST PASS)
just test     # tests (MUST PASS)
cargo tarpaulin # coverage (must be 100%)
```

## Critical Rules

### 1. Testing (100% Coverage Required)

**"Hard to test" is NOT acceptable.** Design for testability.

**Separate logic from I/O:**
```rust
// BAD - untestable
fn save_config(config: &Config) -> Result<()> {
    let json = serde_json::to_string(config)?;
    std::fs::write("config.json", json)?;
    Ok(())
}

// GOOD - test serialize_config, don't test fs::write
fn serialize_config(config: &Config) -> Result<String> {
    serde_json::to_string_pretty(config).map_err(Into::into)
}
```

**What to test vs mock:**
| Test the logic | Mock the boundary |
|----------------|-------------------|
| Response parsing | HTTP client |
| Request building | Network calls |
| JSON serialization | File system |
| Business rules | External APIs |

**Use traits for external dependencies:**
```rust
// Trait for API
pub trait GithubApi {
    async fn list_prs(&self) -> Result<Vec<PullRequest>>;
}

// Handler accepts trait
pub async fn show_prs(api: &impl GithubApi) -> Result<()> {
    let prs = api.list_prs().await?;
    // ...
}

// Mock in tests
#[cfg(test)]
struct MockGithub { prs: Vec<PullRequest> }
impl GithubApi for MockGithub {
    async fn list_prs(&self) -> Result<Vec<PullRequest>> {
        Ok(self.prs.clone())
    }
}
```

### 2. Architecture (Interface-Agnostic)

Services return data, interfaces format it. Same logic powers CLI, REST API, MCP.

```
Interfaces (cli/) → Services (service.rs) → Clients (client.rs) → Types (types.rs)
```

**Module structure:**
```
{module}/
  mod.rs           # Re-exports + command dispatch
  cli.rs           # CLI args (clap derive)
  types.rs         # Data structs
  config.rs        # Module-specific config
  client/          # API calls (implements trait)
    mod.rs
    tests.rs
  display/         # Output formatting
    mod.rs
    tests.rs
```

### 3. Structure (Base-First)

**Never assume simple.** Always:
1. Base infrastructure first (types, client, service)
2. Then handlers

**util/ first** for anything reusable:
- `util/fmt.rs` - time_ago, bytes, pluralize
- `util/config.rs` - config loading
- `util/http.rs` - HTTP client setup

### 4. Style

- Predicates: `is_`, `has_`, `can_`
- Iterators over loops
- Early returns, flat structure
- All types: `#[derive(Debug)]`
- Max 400 lines/file, 50 lines/function
- Import order: std → external crates → crate → super/self

**Forbidden:**
- `.unwrap()` in library code (use `?` or `expect()` with context)
- `panic!()` for recoverable errors
- Wildcard imports (`use foo::*`)
- `dbg!()` / `todo!()` in committed code
- Magic numbers (use named constants)
- Silent failures (always propagate with `?`)

### 5. Dependencies

**Ask before adding.** Present options with trade-offs.

API clients: `octocrab` (gh), `gouqi` (jira), `reqwest` (sentry/pagerduty)

### 6. Output

- `comfy_table` with `UTF8_FULL_CONDENSED` preset for tables
- `serde_json::to_string_pretty` for JSON output (via `-j`/`--json` flags)
- Colors: green=success, yellow=progress, red=error
- Icons: ✓ ◐ ○ ✗ ⚠
- No plain `println!` for user-facing output

## AWS Safety
- READ-ONLY operations only
- `-e dev` only for EKS testing
