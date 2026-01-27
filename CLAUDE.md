# hu CLI - Rules

## Critical
- READ-ONLY AWS ops, `-e dev` only for EKS
- Base logic first, then handlers
- util/ first for reusable code
- Interface-agnostic: services return data, cli/api format it
- Ask before adding deps

## Structure
```
src/
  main.rs           # CLI entry only
  lib.rs            # Exports
  cli.rs            # Clap commands
  core/dashboard.rs # Data aggregation
  util/
    fmt.rs          # timeago, humantime, Inflector, humansize
    http.rs         # reqwest setup
    config.rs       # Config loading
    ui/table.rs progress.rs status.rs  # ratatui helpers
  {module}/
    mod.rs          # Re-exports + CLI enum
    service.rs      # Business logic (no UI)
    client.rs       # API calls
    types.rs        # Data structs
    {subcommand}.rs # Handler
```

## Stack
CLI: clap, ratatui, crossterm, tui-markdown
Core: anyhow, thiserror, serde, serde_json, tokio, tracing
Fmt: timeago, humantime, Inflector, humansize
API: octocrab(gh), gouqi(jira), slack-rust, aws-sdk-*, graphql_client+reqwest(newrelic), reqwest(sentry,pagerduty)

## Style
- is_/has_/can_ predicates
- Iterators > loops
- Early returns, flat
- Traits for external deps (mockable)
- All types: #[derive(Debug)]

## Testing
- Unit: inline #[cfg(test)]
- Integration: tests/
- Logic separate from I/O
- Mock boundaries, test logic

## Output
- ratatui for tables/progress/status
- Colors: green=success, yellow=progress, red=error
- Icons: ✓ ◐ ○ ✗ ⚠ ⊘

## Commands
```
just check    # fmt + clippy
just test     # tests
just build    # debug
just release  # release
just install  # cargo install
```
