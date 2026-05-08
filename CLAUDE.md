# hu CLI

## Commands
```bash
just check          # fmt + clippy (MUST PASS, no #[allow] without justification)
just test           # tests (MUST PASS)
cargo llvm-cov      # coverage (preferred — LLVM source-based, cross-platform)
cargo tarpaulin     # coverage (legacy, still works on x86_64 Linux)
# target: ≥85% with carve-outs (see Section 1)
```

## Critical Rules

### 1. Testing (≥85% coverage with carve-outs)

**Coverage target: 85%+**, not 100%. The 100% target was vanity — it pushed traits and mocks around code that had no logic, and tarpaulin's branch/async accuracy made the last 15% a fight with the tool, not the bug.

**Carve-outs allowed** — preferred annotations in order:
1. `#[coverage(off)]` (modern, portable)
2. `--ignore-filename-regex` patterns in CI invocation
3. `#[cfg_attr(tarpaulin, skip)]` / `#[cfg(not(tarpaulin_include))]` (legacy, only if using tarpaulin)

Categories:
- Trivial `Display` / `From` / getter impls with no branching
- `main.rs` glue
- `#[cfg(test)]` mock types
- One-line shellout wrappers whose logic lives in the trait caller

**"Hard to test" is still not an excuse — but "not worth testing" is a real category.** Use judgment.

**Separate logic from I/O:**
```rust
// BAD - untestable, no value
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
| Test the logic        | Mock the boundary |
|-----------------------|-------------------|
| Response parsing      | HTTP client       |
| Request building      | Network calls     |
| JSON serialization    | File system       |
| Business rules        | External APIs     |

**Use traits for external dependencies — when there's a reason:**

A trait earns its keep when **(a) ≥2 implementers exist or are likely**, **(b) the I/O is genuinely painful to set up in tests**, or **(c) it's a stable seam between layers (CLI / service / MCP)**. Don't add a trait just so you can mock a `Command::new("git")` call once — a `#[cfg(test)]` injection point or a `Shell` chokepoint covers that.

**Prefer static dispatch:** `&impl Trait` or `<T: Trait>` generics over `&dyn Trait` / `Box<dyn Trait>`. Use `dyn` only when you genuinely need heterogeneous polymorphism (e.g. a `Vec<Box<dyn Installer>>` mixing brew + apt). Static dispatch monomorphizes — zero runtime cost.

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

**Module structure — applies once a module crosses ~400 lines.** Below that, a single file is fine. Don't pre-split.

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
- `util/fmt.rs` — time_ago, bytes, pluralize
- `util/config.rs` — config loading
- `util/http.rs` — HTTP client setup

### 4. Style

- Predicates: `is_`, `has_`, `can_`
- Prefer iterators when they read clearer; `for` is fine for early-return / side-effect loops
- Early returns, flat structure
- All types: `#[derive(Debug)]`
- **Max 400 lines/file, 80 lines/function** (use judgment — clap arg structs and match dispatchers naturally hit 60–80 and read better whole)
- Import order: std → external crates → crate → super/self

**Forbidden:**
- `.unwrap()` in library code (use `?` or `expect("invariant: ...")` with context)
- `panic!()` for recoverable errors
- Wildcard imports (`use foo::*`)
- `dbg!()` / `todo!()` / `unimplemented!()` in committed code
- Magic numbers when meaning isn't local-obvious (name `MAX_RETRIES = 3`, but `vec.len() > 0` doesn't need a constant)
- Silent failures (always propagate with `?`; `.ok()` discard requires a `// reason:` comment)
- `#[allow(...)]` annotations without a `// reason: <why>` comment — fix the lint or justify the suppression
- Free-form `// TODO`: prefer `// TODO(#123): description` linking to a tracked issue

**Optional enforcement** — pin the function size budget via `clippy.toml`:
```toml
too-many-lines-threshold = 80
```

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
