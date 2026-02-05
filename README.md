# hu

[![Crates.io](https://img.shields.io/crates/v/hu.svg)](https://crates.io/crates/hu)
[![Downloads](https://img.shields.io/crates/d/hu.svg)](https://crates.io/crates/hu)
[![CI](https://github.com/aladac/hu/actions/workflows/ci.yml/badge.svg)](https://github.com/aladac/hu/actions/workflows/ci.yml)

Dev workflow CLI for Claude Code integration. Unifies access to Jira, GitHub, Slack, PagerDuty, Sentry, NewRelic, AWS CodePipeline, and EKS from a single tool, with built-in support for Claude Code session analytics.

## Install

```bash
cargo install hu
```

Or from source:

```bash
cargo install --path .
```

## Commands

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
hu utils       Utility commands (fetch-html, grep, web-search, docs)
hu context     Session context tracking
hu read        Smart file reading
```

---

### Jira

Manage Jira tickets without leaving the terminal. OAuth 2.0 authentication, sprint views, JQL search, and ticket updates.

```bash
hu jira auth                   # OAuth 2.0 authentication
hu jira tickets                # List my tickets in current sprint
hu jira sprint                 # Show all issues in current sprint
hu jira search <query>         # Search tickets using JQL
hu jira show <ticket>          # Show ticket details
hu jira update <ticket>        # Update a ticket
  --summary <text>             #   New summary/title
  --status <status>            #   New status (transition)
  --assign <user>              #   Assign to user (or "me")
```

### GitHub

GitHub workflow integration. List PRs, monitor CI runs, extract test failures, and get AI-ready investigation context for fixing broken builds.

```bash
hu gh login -t <PAT>           # Authenticate with PAT
hu gh prs                      # List your open PRs
hu gh runs [ticket]            # List workflow runs
  -s, --status <status>        #   Filter: queued, in_progress, completed, success, failure
  -b, --branch <name>          #   Filter by branch
  -r, --repo <owner/repo>      #   Repository
  -n, --limit <n>              #   Max results (default: 20)
  -j, --json                   #   Output as JSON
hu gh failures                 # Extract test failures from CI
  --pr <number>                #   PR number (default: current branch)
  -r, --repo <owner/repo>      #   Repository
hu gh fix                      # Analyze CI failures, output investigation context
  --pr <number>                #   PR number
  --run <id>                   #   Workflow run ID
  -b, --branch <name>          #   Branch name
  -r, --repo <owner/repo>      #   Repository
  -j, --json                   #   Output as JSON
```

### Slack

Slack workspace access. Send messages, search history, list channels, and bulk-tidy unread channels without direct mentions.

```bash
hu slack auth                  # Authenticate with Slack
  --token <xoxb-...>           #   Bot token
  --user-token <xoxp-...>      #   User token (for search)
hu slack channels              # List channels
hu slack info <channel>        # Show channel details
hu slack send <channel> <msg>  # Send message
hu slack history <channel>     # Show message history
  --limit <n>                  #   Number of messages (default: 20)
hu slack search <query>        # Search messages
  -n, --count <n>              #   Max results (default: 20)
hu slack users                 # List users
hu slack config                # Show configuration status
hu slack whoami                # Show current user info
hu slack tidy                  # Mark channels as read if no mentions
  --dry-run                    #   Preview without marking
```

### PagerDuty

On-call schedules, active alerts, and incident management. Quickly check who's on call or review incident details.

```bash
hu pagerduty auth <token>      # Set API token
hu pagerduty config            # Show configuration status
hu pagerduty oncall            # Show who's currently on call
  -p, --policy <id>            #   Filter by escalation policy
  -s, --schedule <id>          #   Filter by schedule
  --json                       #   Output as JSON
hu pagerduty alerts            # List active alerts (triggered + acknowledged)
  -l, --limit <n>              #   Max alerts (default: 25)
  --json                       #   Output as JSON
hu pagerduty incidents         # List incidents with filters
  -s, --status <status>        #   Filter: triggered, acknowledged, resolved, active
  -l, --limit <n>              #   Max incidents (default: 25)
  --json                       #   Output as JSON
hu pagerduty show <id>         # Show incident details
  --json                       #   Output as JSON
hu pagerduty whoami            # Show current user info
  --json                       #   Output as JSON
hu pd ...                      # Alias: pd -> pagerduty
```

### Sentry

Error tracking integration. List unresolved issues, view error details, and browse event history.

```bash
hu sentry auth <token>         # Set auth token
hu sentry config               # Show configuration status
hu sentry issues               # List unresolved issues
hu sentry show <id>            # Show issue details
hu sentry events <id>          # List events for an issue
```

### NewRelic

Application performance monitoring. Query NRQL, list incidents and issues, check system health.

```bash
hu newrelic auth <key>         # Set API key
  --account <id>               #   Account ID (required)
hu newrelic config             # Show configuration status
hu newrelic issues             # List recent issues
  --limit <n>                  #   Max issues (default: 25)
hu newrelic incidents          # List recent incidents
  --limit <n>                  #   Max incidents (default: 25)
hu newrelic query <nrql>       # Run NRQL query
hu nr ...                      # Alias: nr -> newrelic
```

### Pipeline (AWS CodePipeline)

Monitor AWS CodePipeline deployments. List pipelines, check stage status, review execution history.

```bash
hu pipeline list               # List all pipelines
  -r, --region <region>        #   AWS region
  --json                       #   Output as JSON
hu pipeline status <name>      # Show pipeline status (stages and actions)
  -r, --region <region>        #   AWS region
  --json                       #   Output as JSON
hu pipeline history <name>     # Show execution history
  -r, --region <region>        #   AWS region
  -l, --limit <n>              #   Max results (default: 10)
  --json                       #   Output as JSON
```

### EKS

Kubernetes pod access for EKS clusters. List pods, exec into containers, tail logs.

```bash
hu eks list                    # List pods in the cluster
  -n, --namespace <ns>         #   Namespace
  -A, --all-namespaces         #   All namespaces
  -c, --context <ctx>          #   Kubeconfig context
  --json                       #   Output as JSON
hu eks exec <pod>              # Execute command in pod (shell by default)
  -n, --namespace <ns>         #   Namespace
  -c, --container <name>       #   Container name
  --context <ctx>              #   Kubeconfig context
  -- <command>                 #   Command to run (default: /bin/sh)
hu eks logs <pod>              # Tail logs from a pod
  -n, --namespace <ns>         #   Namespace
  -c, --container <name>       #   Container name
  -f, --follow                 #   Follow log output
  --previous                   #   Previous container instance
  --tail <n>                   #   Lines from end
  --context <ctx>              #   Kubeconfig context
```

### Data (Claude Code Sessions)

Sync and analyze Claude Code session data. Track usage statistics, search message history, monitor tool usage, and analyze costs.

```bash
hu data sync                   # Sync Claude data to local database
  -f, --force                  #   Force full resync
  -q, --quiet                  #   Quiet output
hu data config                 # Show data configuration
  -j, --json                   #   Output as JSON
hu data session list           # List sessions
  -p, --project <dir>          #   Filter by project
  -n, --limit <n>              #   Max results (default: 20)
  -j, --json                   #   Output as JSON
hu data session read <id>      # Read session messages
  -j, --json                   #   Output as JSON
hu data session current        # Show current session
  -j, --json                   #   Output as JSON
hu data stats                  # Usage statistics
  -t, --today                  #   Today only
  -j, --json                   #   Output as JSON
hu data todos list             # List all todos
  -s, --status <status>        #   Filter by status
  -j, --json                   #   Output as JSON
hu data todos pending          # Show pending todos
  -p, --project <dir>          #   Filter by project
  -j, --json                   #   Output as JSON
hu data search <query>         # Search messages (full-text)
  -n, --limit <n>              #   Max results (default: 20)
  -j, --json                   #   Output as JSON
hu data tools                  # Tool usage statistics
  -t, --tool <name>            #   Detail for specific tool
  -j, --json                   #   Output as JSON
hu data errors                 # Extract errors from debug logs
  -r, --recent <days>          #   Days to look back (default: 7)
  -j, --json                   #   Output as JSON
hu data pricing                # Pricing analysis vs API costs
  -s, --subscription <tier>    #   Subscription tier (default: max20x)
  -b, --billing-day <day>      #   Billing day of month (default: 6)
  -j, --json                   #   Output as JSON
hu data branches               # Branch activity statistics
  -b, --branch <name>          #   Filter by branch
  -l, --limit <n>              #   Max results (default: 20)
  -j, --json                   #   Output as JSON
```

### Utils

General-purpose utilities for web content, code search, and documentation.

```bash
# Fetch HTML and convert to markdown
hu utils fetch-html <url>
  -c, --content                # Extract main content only
  -s, --summary                # First N paragraphs + headings
  -l, --links                  # Extract links only
  -H, --headings               # Extract headings (outline)
  --selector <css>             # CSS selector (e.g., "article")
  -o, --output <file>          # Output to file
  -r, --raw                    # Raw output (no filtering)

# Smart grep with token-saving options
hu utils grep <pattern> [path]
  --refs                       # File:line references only
  --unique                     # Deduplicate similar matches
  --ranked                     # Sort by relevance
  -n, --limit <n>              # Limit results
  --signature                  # Function/class signature only
  -g, --glob <pattern>         # File glob (e.g., "*.rs")
  -i, --ignore-case            # Case insensitive
  --hidden                     # Include hidden files

# Web search (requires Brave Search API key)
hu utils web-search <query>
  -n, --results <n>            # Number of results (default: 3)
  -l, --list                   # Show results only (don't fetch)
  -o, --output <file>          # Output to file

# Documentation indexing
hu utils docs-index [path]     # Build heading index (JSON)
  -o, --output <file>          # Output index to file
hu utils docs-search <idx> <q> # Search docs index
  -n, --limit <n>              # Limit results
hu utils docs-section <f> <h>  # Extract section from markdown
```

### Context Tracking

Track which files have been loaded in a Claude Code session to prevent duplicate reads and save tokens.

```bash
hu context track <file...>     # Mark file(s) as loaded
hu context check <file...>     # Check if already in context
hu context summary             # Show all tracked files
hu context clear               # Reset tracking
```

### Smart File Reading

Token-efficient file reading modes for AI agents. Get outlines, public interfaces, or focused diffs instead of full file contents.

```bash
hu read <file>
  -o, --outline                # Show functions, structs, classes
  -i, --interface              # Public API only
  -a, --around <line>          # Lines around line number
  -n, --context <n>            # Context lines (default: 10)
  -d, --diff                   # Git diff
  --commit <ref>               # Diff against commit (default: HEAD)
```

## Configuration

Credentials are stored in `~/.config/hu/credentials.toml`. Settings in `~/.config/hu/settings.toml`.

## Development

```bash
just check    # fmt + clippy (must pass)
just test     # run tests (must pass)
just build    # build release
```

## License

MIT
