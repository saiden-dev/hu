# hu

[![Crates.io](https://img.shields.io/crates/v/hu.svg)](https://crates.io/crates/hu)
[![Downloads](https://img.shields.io/crates/d/hu.svg)](https://crates.io/crates/hu)
[![CI](https://github.com/aladac/hu/actions/workflows/ci.yml/badge.svg)](https://github.com/aladac/hu/actions/workflows/ci.yml)

Token-efficient dev workflow CLI for Claude Code. Optimizes context usage through smart file reading, context tracking, and pre-tool hooks.

## Why hu?

**Claude Code sessions are token-limited.** Every file read, grep search, and web fetch consumes context. Wasted tokens mean hitting limits faster, losing conversation history to compaction, and slower responses.

hu solves this by:

1. **Smart file reading** - Get outlines, interfaces, or specific lines instead of entire files
2. **Context tracking** - Prevent reading the same file twice
3. **Pre-tool hooks** - Automatically warn about wasteful operations
4. **Unified integrations** - Single CLI for Jira, GitHub, Slack, PagerDuty, Sentry, NewRelic, and AWS

See **[SAVINGS.md](SAVINGS.md)** for detailed token usage comparisons.

## Install

```bash
cargo install hu
hu install run  # Install hooks and slash commands to Claude Code
```

## Quick Start

```bash
# Read file outline instead of full content (90% token savings)
hu read -o src/main.rs

# Check if file is already in context
hu context check src/main.rs

# Get only function signatures from grep
hu utils grep "fn main" --signature

# Fetch only main content from webpage
hu utils fetch-html https://docs.rs -c
```

## Documentation

| Document | Description |
|----------|-------------|
| **[CLI.md](CLI.md)** | Complete CLI reference with all flags and options |
| **[COMMANDS.md](COMMANDS.md)** | Slash command reference (`/hu:*` commands) |
| **[SAVINGS.md](SAVINGS.md)** | Token usage analysis and optimization guide |

## Commands Overview

```
hu jira        Jira operations (tickets, sprint, search)
hu gh          GitHub operations (prs, runs, failures, fix)
hu slack       Slack operations (messages, channels)
hu pagerduty   PagerDuty (oncall, alerts, incidents)
hu sentry      Sentry (issues, errors)
hu newrelic    NewRelic (incidents, queries)
hu pipeline    AWS CodePipeline (list, status, history)
hu eks         EKS pod access (list, exec, logs)
hu data        Claude Code session data (sync, stats, search)
hu utils       Utility commands (fetch-html, grep, web-search)
hu context     Session context tracking
hu read        Smart file reading
hu install     Install hooks and commands
```

## Configuration

Credentials: `~/.config/hu/credentials.toml`
Settings: `~/.config/hu/settings.toml`

## Development

```bash
just check    # fmt + clippy (must pass)
just test     # run tests (must pass)
just build    # build release
```

## License

MIT
