# Phase 1: Hello World CLI

## Goal
Minimal CLI that compiles, shows help, passes all checks.

## DOD
- [x] `hu --help` shows usage
- [x] `just check` passes (fmt + clippy)
- [x] `just test` passes (even if no tests yet)
- [x] `just build` succeeds

## Files to Create

### src/main.rs
```rust
use clap::Parser;

#[derive(Parser)]
#[command(name = "hu")]
#[command(about = "Dev workflow CLI", long_about = None)]
#[command(version)]
struct Cli {
    // Future: #[command(subcommand)] cmd: Commands
}

fn main() -> anyhow::Result<()> {
    let _cli = Cli::parse();
    println!("hu - dev workflow CLI");
    Ok(())
}
```

## Verification
```bash
just check    # fmt + clippy pass
just test     # tests pass
just build    # compiles
cargo run -- --help  # shows help
cargo run -- --version  # shows version
```

## Next Phase
Phase 2: Add first subcommand structure (empty `hu jira` and `hu gh`)
