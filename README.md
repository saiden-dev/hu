# hu

Dev workflow CLI for Claude Code integration.

## Install

```bash
cargo install --path .
```

## Commands

```
hu dashboard   Dev dashboard (PRs, tasks, alerts, oncall)
hu jira        Jira operations (tickets, sprint, search)
hu gh          GitHub operations (prs, runs, failures)
hu slack       Slack operations (messages, channels)
hu pagerduty   PagerDuty (oncall, alerts)
hu sentry      Sentry (issues, errors)
hu newrelic    NewRelic (incidents, queries)
hu eks         EKS pod access (list, exec, logs)
hu utils       Utility commands (fetch-html, grep, docs)
hu context     Session context tracking
hu read        Smart file reading
```

### Jira

```bash
hu jira auth              # OAuth 2.0 authentication
hu jira tickets           # List tickets in current sprint
hu jira sprint            # Show current sprint info
hu jira search <query>    # Search tickets using JQL
hu jira show <ticket>     # Show ticket details
hu jira update <ticket>   # Update a ticket
```

### GitHub

```bash
hu gh login               # Authenticate with PAT
hu gh prs                 # List your open PRs
hu gh runs                # List workflow runs
hu gh failures            # Extract test failures from CI
hu gh ci                  # Check CI status for current branch
```

### Utils

```bash
hu utils fetch-html <url>           # Fetch URL, convert to markdown
hu utils grep <pattern>             # Smart grep with token-saving options
hu utils web-search <query>         # Web search (Brave Search API)
hu utils docs-index [path]          # Build heading index for markdown
hu utils docs-search <index> <q>    # Search docs index
hu utils docs-section <file> <h>    # Extract section from markdown
```

### Context Tracking

Prevent duplicate file reads in Claude Code sessions:

```bash
hu context track <file>   # Mark file as loaded
hu context check <file>   # Check if already in context
hu context summary        # Show all tracked files
hu context clear          # Reset tracking
```

### Smart File Reading

Token-efficient file reading for AI agents:

```bash
hu read <file> --outline      # Show functions, structs, classes
hu read <file> --interface    # Public API only
hu read <file> --around 50    # Lines around line 50
hu read <file> --diff         # Git diff vs HEAD
```

## Development

```bash
just check    # fmt + clippy (must pass)
just test     # run tests (must pass)
just build    # build release
```

## License

MIT
