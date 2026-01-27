# Testing

## Coverage Requirement

**100% test coverage required.** Run `cargo tarpaulin` to verify.

- Mark unreachable code with `unreachable!()` — don't leave dead `Ok(())`
- Every code path must be exercised by tests

## Write Testable Code

**"Hard to test" or "impossible to test" is not acceptable.** Design for testability from the start.

### Separate Logic from Side Effects

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

### What to Test vs What to Mock

| Test the logic | Mock/stub the boundary |
|----------------|------------------------|
| JSON/data serialization | File system writes |
| Request building | Network calls |
| Response parsing | HTTP client |
| Query construction | Database calls |
| Business rules | External APIs |
| Data transformation | System calls |

### Use Traits for External Dependencies

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

### Test Expectations on Arguments

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

### Rules

- Never write code that "can't be tested"
- If something is hard to test, refactor to make it testable
- Push I/O to the edges, keep core logic pure
- Accept traits/interfaces, not concrete implementations
- Test the logic, mock the boundaries

## Test Location (Rust Convention)

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

## Test Directory Structure

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

## Test Types

- **Unit tests** - Inline `#[cfg(test)]`, test private + public functions
- **Integration tests** - `tests/` directory, test public API, CLI commands
- **Snapshot tests** - Use `insta` for output formatting
- **Benchmarks** - Use `criterion` in `benches/`

## Before Refactoring

1. Ensure existing tests pass
2. Add tests for functions being extracted
3. Consider property-based tests for parsing

## CLI Integration Tests

For CLI apps, test actual binary behavior with `std::process::Command`:

```rust
// tests/cli.rs
use std::process::Command;

fn hu() -> Command {
    Command::new(env!("CARGO_BIN_EXE_hu"))
}

#[test]
fn no_args_shows_help_exits_zero() {
    let output = hu().output().expect("failed to execute");
    assert!(output.status.success());  // exit 0, not 2
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"));  // stdout, not stderr
}

#[test]
fn subcommand_runs() {
    let output = hu()
        .args(["jira", "tickets"])
        .output()
        .expect("failed to execute");
    assert!(output.status.success());
}
```

**CLI behavior rules:**
- Help on no args/subcommand → **exit 0**, output to **stdout**
- Invalid command → exit non-zero
- Test all command paths for 100% coverage

## Test Commands

```bash
just check      # fmt + clippy
just test       # run all tests
cargo tarpaulin # coverage (must be 100%)
cargo insta review  # review snapshot changes
```
