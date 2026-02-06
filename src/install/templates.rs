use crate::install::types::{Component, ComponentKind};

// ============================================================================
// HOOKS
// ============================================================================

pub const HOOK_PRE_READ: &str = r#"#!/bin/bash
# Pre-Read Hook: Prevent duplicate reads and auto-truncate large files
set -euo pipefail
[[ "${HU_SKIP_HOOKS:-}" == "1" ]] && exit 0

MAX_LINES="${HU_MAX_FILE_LINES:-500}"
INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.file_path // empty')
[[ -z "$FILE_PATH" ]] && exit 0

if command -v hu &>/dev/null; then
    STATUS=$(hu context check "$FILE_PATH" 2>/dev/null || echo "")
    if echo "$STATUS" | grep -q "loaded"; then
        AGO=$(echo "$STATUS" | grep -oE '[0-9]+ (seconds?|minutes?|hours?) ago' || echo "recently")
        echo "File already in context (loaded $AGO). Use --force to re-read."
        exit 0
    fi
fi

if [[ -f "$FILE_PATH" ]]; then
    LINE_COUNT=$(wc -l < "$FILE_PATH" 2>/dev/null || echo "0")
    if [[ "$LINE_COUNT" -gt "$MAX_LINES" ]]; then
        echo "Warning: $FILE_PATH has $LINE_COUNT lines (>${MAX_LINES} threshold)."
        echo "Consider: hu read '$FILE_PATH' --outline | --interface | --around N"
    fi
    command -v hu &>/dev/null && hu context track "$FILE_PATH" 2>/dev/null || true
fi
exit 0
"#;

pub const HOOK_PRE_GREP: &str = r#"#!/bin/bash
# Pre-Grep Hook: Warn on broad patterns
set -euo pipefail
[[ "${HU_SKIP_HOOKS:-}" == "1" ]] && exit 0

INPUT=$(cat)
PATTERN=$(echo "$INPUT" | jq -r '.pattern // empty')
[[ -z "$PATTERN" ]] && exit 0

BROAD_PATTERNS=("^.$" "^..$" "the" "function" "import" "return" "if" "for" "var" "let" "const")
PATTERN_LOWER=$(echo "$PATTERN" | tr '[:upper:]' '[:lower:]')
for BROAD in "${BROAD_PATTERNS[@]}"; do
    if [[ "$PATTERN_LOWER" =~ $BROAD ]]; then
        echo "Warning: Pattern '$PATTERN' may match many results."
        break
    fi
done
exit 0
"#;

pub const HOOK_PRE_WEBFETCH: &str = r#"#!/bin/bash
# Pre-WebFetch Hook: Log URL fetches
set -euo pipefail
[[ "${HU_SKIP_HOOKS:-}" == "1" ]] && exit 0

INPUT=$(cat)
URL=$(echo "$INPUT" | jq -r '.url // empty')
[[ -z "$URL" ]] && exit 0

echo "WebFetch: $URL"
[[ "$URL" == http://* ]] && echo "Warning: Using HTTP instead of HTTPS"
exit 0
"#;

pub const HOOK_PRE_WEBSEARCH: &str = r#"#!/bin/bash
# Pre-WebSearch Hook: Log searches
set -euo pipefail
[[ "${HU_SKIP_HOOKS:-}" == "1" ]] && exit 0

INPUT=$(cat)
QUERY=$(echo "$INPUT" | jq -r '.query // empty')
[[ -z "$QUERY" ]] && exit 0

echo "WebSearch: \"$QUERY\""
[[ ${#QUERY} -lt 5 ]] && echo "Warning: Query is very short."
exit 0
"#;

pub const HOOK_SESSION_START: &str = r#"#!/bin/bash
# Session-Start Hook: Initialize context tracking and cleanup
set -euo pipefail
[[ "${HU_SKIP_HOOKS:-}" == "1" ]] && exit 0

CLEANUP_DAYS="${HU_CLEANUP_DAYS:-7}"
CLAUDE_DIR="${HOME}/.claude"

[[ -d "$CLAUDE_DIR/debug" ]] && find "$CLAUDE_DIR/debug" -type f -mtime +$CLEANUP_DAYS -delete 2>/dev/null || true
echo "Cleaned debug files older than $CLEANUP_DAYS days"

[[ -d "$CLAUDE_DIR/shell-snapshots" ]] && find "$CLAUDE_DIR/shell-snapshots" -type f -mtime +$CLEANUP_DAYS -delete 2>/dev/null || true
[[ -d "$CLAUDE_DIR/paste-cache" ]] && find "$CLAUDE_DIR/paste-cache" -type f -mtime +$CLEANUP_DAYS -delete 2>/dev/null || true

if command -v hu &>/dev/null; then
    hu context clear 2>/dev/null || true
    echo "Context tracking initialized"
fi

if [[ -d ".git" ]] && [[ -d "doc" || -d "docs" ]]; then
    DOC_DIR="doc"; [[ -d "docs" ]] && DOC_DIR="docs"
    if command -v hu &>/dev/null; then
        INDEX_FILE="/tmp/hu-docs-index-$(basename "$(pwd)").json"
        (hu utils docs-index "$DOC_DIR" -o "$INDEX_FILE" 2>/dev/null &)
        echo "Building docs index in background: $INDEX_FILE"
    fi
fi

echo -e "\nSession initialized. Token-saving features active:"
echo "  - Context tracking: prevents duplicate file reads"
echo "  - Large file warnings: suggests --outline/--around"
echo "  - Grep limits: caps broad searches"
echo -e "\nBypass with: HU_SKIP_HOOKS=1"
exit 0
"#;

pub const HOOK_SESSION_END: &str = r#"#!/bin/bash
# Session-End Hook: Cleanup context tracking
set -euo pipefail
[[ "${HU_SKIP_HOOKS:-}" == "1" ]] && exit 0

if command -v hu &>/dev/null; then
    SUMMARY=$(hu context summary 2>/dev/null || echo "")
    if [[ -n "$SUMMARY" ]] && ! echo "$SUMMARY" | grep -q "No files"; then
        echo "Session context summary:"
        echo "$SUMMARY"
    fi
    hu context clear 2>/dev/null || true
fi

INDEX_FILE="/tmp/hu-docs-index-$(basename "$(pwd)").json"
[[ -f "$INDEX_FILE" ]] && rm -f "$INDEX_FILE"
echo "Session cleanup complete"
exit 0
"#;

// ============================================================================
// COMMANDS - Context
// ============================================================================

pub const CMD_CONTEXT_TRACK: &str = r#"Track file(s) as loaded in context.

```bash
hu context track <file>...
```

Prevents duplicate reads of the same files in a session.
"#;

pub const CMD_CONTEXT_CHECK: &str = r#"Check if file(s) are already in context.

```bash
hu context check <file>...
# Output: src/main.rs: loaded 5 minutes ago
```
"#;

pub const CMD_CONTEXT_SUMMARY: &str = r#"Show summary of all tracked files in context.

```bash
hu context summary
```

Shows all files currently tracked in the session context with load times.
"#;

pub const CMD_CONTEXT_CLEAR: &str = r#"Clear all tracked files from context.

```bash
hu context clear
```

Resets the context tracking. Use at session end or to force re-reads.
"#;

// ============================================================================
// COMMANDS - Read
// ============================================================================

pub const CMD_READ: &str = r#"Smart file reading with outline, interface, around, and diff modes.

```bash
hu read src/main.rs                        # Full file
hu read src/main.rs -o                     # Outline (functions, structs, classes)
hu read src/main.rs -i                     # Public interface only
hu read src/main.rs -a 42                  # Lines around line 42
hu read src/main.rs -a 42 -n 20            # 20 context lines around line 42
hu read src/main.rs -d                     # Git diff (vs HEAD)
hu read src/main.rs -d --commit abc123     # Diff against specific commit
```

| Flag | Description |
|------|-------------|
| `-o, --outline` | Show file outline (functions, structs, classes) |
| `-i, --interface` | Public interface only (pub items in Rust, exports in JS) |
| `-a, --around` | Show lines around a specific line number |
| `-n, --context` | Context lines for `--around` (default: 10) |
| `-d, --diff` | Show git diff |
| `--commit` | Commit to diff against (default: HEAD) |
"#;

// ============================================================================
// COMMANDS - Jira
// ============================================================================

pub const CMD_JIRA_AUTH: &str = r#"Authenticate with Jira via OAuth 2.0.

```bash
hu jira auth
```

Opens browser for OAuth flow, stores credentials in `~/.config/hu/credentials.toml`.
"#;

pub const CMD_JIRA_TICKETS: &str = r#"List my tickets in current sprint.

```bash
hu jira tickets           # List assigned tickets
hu jira tickets -j        # JSON output
```
"#;

pub const CMD_JIRA_SPRINT: &str = r#"Show all issues in current sprint.

```bash
hu jira sprint            # List all sprint issues
hu jira sprint -j         # JSON output
```
"#;

pub const CMD_JIRA_SEARCH: &str = r#"Search tickets using JQL.

```bash
hu jira search "project = PROJ AND status = Open"
hu jira search "assignee = currentUser()" -j
```
"#;

pub const CMD_JIRA_SHOW: &str = r#"Show ticket details.

```bash
hu jira show PROJ-123     # Show ticket details
hu jira show PROJ-123 -j  # JSON output
```
"#;

pub const CMD_JIRA_UPDATE: &str = r#"Update a Jira ticket.

```bash
hu jira update PROJ-123 --status "In Progress"
hu jira update PROJ-123 --assignee "user@example.com"
hu jira update PROJ-123 --comment "Working on this"
```
"#;

// ============================================================================
// COMMANDS - GitHub
// ============================================================================

pub const CMD_GH_LOGIN: &str = r#"Authenticate with GitHub using a Personal Access Token.

```bash
hu gh login <token>
```

Stores token in `~/.config/hu/credentials.toml`.
"#;

pub const CMD_GH_PRS: &str = r#"List open pull requests authored by you.

```bash
hu gh prs                 # List your PRs
hu gh prs -s "search"     # Search PRs
hu gh prs -j              # JSON output
```
"#;

pub const CMD_GH_RUNS: &str = r#"List GitHub workflow runs.

```bash
hu gh runs                # List recent runs
hu gh runs -b main        # Filter by branch
hu gh runs -j             # JSON output
```
"#;

pub const CMD_GH_FAILURES: &str = r#"Extract test failures from CI.

```bash
hu gh failures            # Get failures from current branch
hu gh failures --pr 123   # Get failures from PR
hu gh failures -j         # JSON output
```
"#;

pub const CMD_GH_FIX: &str = r#"Analyze CI failures and output investigation context.

```bash
hu gh fix                 # Analyze failures from current branch
hu gh fix --pr 123        # Analyze failures from PR
hu gh fix -j              # JSON output with file paths
```
"#;

// ============================================================================
// COMMANDS - Slack
// ============================================================================

pub const CMD_SLACK_AUTH: &str = r#"Authenticate with Slack (OAuth flow or direct token).

```bash
hu slack auth             # Start OAuth flow
hu slack auth <token>     # Use direct token
```
"#;

pub const CMD_SLACK_CHANNELS: &str = r#"List channels in the workspace.

```bash
hu slack channels         # List all channels
hu slack channels -j      # JSON output
```
"#;

pub const CMD_SLACK_INFO: &str = r#"Show channel details.

```bash
hu slack info #channel    # Show channel info
hu slack info C123456     # By channel ID
```
"#;

pub const CMD_SLACK_SEND: &str = r#"Send a message to a channel.

```bash
hu slack send #channel "Hello world"
hu slack send C123456 "Message text"
```
"#;

pub const CMD_SLACK_HISTORY: &str = r#"Show message history for a channel.

```bash
hu slack history #channel      # Recent messages
hu slack history #channel -n 50  # Last 50 messages
```
"#;

pub const CMD_SLACK_SEARCH: &str = r#"Search Slack messages.

```bash
hu slack search "query"        # Search messages
hu slack search "from:@user"   # Search by user
```
"#;

pub const CMD_SLACK_USERS: &str = r#"List users in the workspace.

```bash
hu slack users            # List all users
hu slack users -j         # JSON output
```
"#;

pub const CMD_SLACK_CONFIG: &str = r#"Show Slack configuration status.

```bash
hu slack config           # Show config
hu slack config -j        # JSON output
```
"#;

pub const CMD_SLACK_WHOAMI: &str = r#"Show current user info from token.

```bash
hu slack whoami           # Show current user
```
"#;

pub const CMD_SLACK_TIDY: &str = r#"Mark channels as read if no direct mentions.

```bash
hu slack tidy             # Tidy all channels
hu slack tidy -d          # Dry run
```
"#;

// ============================================================================
// COMMANDS - PagerDuty
// ============================================================================

pub const CMD_PD_CONFIG: &str = r#"Show PagerDuty configuration status.

```bash
hu pagerduty config       # Show config
hu pagerduty config -j    # JSON output
```
"#;

pub const CMD_PD_AUTH: &str = r#"Set PagerDuty API token.

```bash
hu pagerduty auth <token>
```
"#;

pub const CMD_PD_ONCALL: &str = r#"Show who's currently on call.

```bash
hu pagerduty oncall       # Show on-call
hu pagerduty oncall -j    # JSON output
```
"#;

pub const CMD_PD_ALERTS: &str = r#"List active alerts (triggered + acknowledged).

```bash
hu pagerduty alerts       # List alerts
hu pagerduty alerts -j    # JSON output
```
"#;

pub const CMD_PD_INCIDENTS: &str = r#"List incidents with filters.

```bash
hu pagerduty incidents              # List incidents
hu pagerduty incidents --status triggered
hu pagerduty incidents -j           # JSON output
```
"#;

pub const CMD_PD_SHOW: &str = r#"Show incident details.

```bash
hu pagerduty show <incident-id>
hu pagerduty show <incident-id> -j  # JSON output
```
"#;

pub const CMD_PD_WHOAMI: &str = r#"Show current PagerDuty user info.

```bash
hu pagerduty whoami
```
"#;

// ============================================================================
// COMMANDS - Sentry
// ============================================================================

pub const CMD_SENTRY_CONFIG: &str = r#"Show Sentry configuration status.

```bash
hu sentry config          # Show config
hu sentry config -j       # JSON output
```
"#;

pub const CMD_SENTRY_AUTH: &str = r#"Set Sentry auth token.

```bash
hu sentry auth <token>
```
"#;

pub const CMD_SENTRY_ISSUES: &str = r#"List Sentry issues.

```bash
hu sentry issues          # List recent issues
hu sentry issues -j       # JSON output
```
"#;

pub const CMD_SENTRY_SHOW: &str = r#"Show Sentry issue details.

```bash
hu sentry show <issue-id>
hu sentry show <issue-id> -j  # JSON output
```
"#;

pub const CMD_SENTRY_EVENTS: &str = r#"List events for a Sentry issue.

```bash
hu sentry events <issue-id>
hu sentry events <issue-id> -j  # JSON output
```
"#;

// ============================================================================
// COMMANDS - NewRelic
// ============================================================================

pub const CMD_NR_CONFIG: &str = r#"Show NewRelic configuration status.

```bash
hu newrelic config        # Show config
hu newrelic config -j     # JSON output
```
"#;

pub const CMD_NR_AUTH: &str = r#"Set NewRelic API key and account ID.

```bash
hu newrelic auth <api-key> <account-id>
```
"#;

pub const CMD_NR_ISSUES: &str = r#"List recent NewRelic issues.

```bash
hu newrelic issues        # List issues
hu newrelic issues -j     # JSON output
```
"#;

pub const CMD_NR_INCIDENTS: &str = r#"List recent NewRelic incidents.

```bash
hu newrelic incidents     # List incidents
hu newrelic incidents -j  # JSON output
```
"#;

pub const CMD_NR_QUERY: &str = r#"Run NRQL query.

```bash
hu newrelic query "SELECT * FROM Transaction LIMIT 10"
hu newrelic query "SELECT count(*) FROM Transaction" -j
```
"#;

// ============================================================================
// COMMANDS - EKS
// ============================================================================

pub const CMD_EKS_LIST: &str = r#"List pods in the EKS cluster.

```bash
hu eks list               # List all pods
hu eks list -n namespace  # Filter by namespace
hu eks list -j            # JSON output
```
"#;

pub const CMD_EKS_EXEC: &str = r#"Execute a command in a pod (interactive shell by default).

```bash
hu eks exec <pod>                    # Open shell
hu eks exec <pod> -- ls -la          # Run command
hu eks exec <pod> -n namespace       # Specify namespace
```
"#;

pub const CMD_EKS_LOGS: &str = r#"Tail logs from a pod.

```bash
hu eks logs <pod>                    # Tail logs
hu eks logs <pod> -f                 # Follow logs
hu eks logs <pod> -n namespace       # Specify namespace
hu eks logs <pod> --since 1h         # Logs from last hour
```
"#;

// ============================================================================
// COMMANDS - Pipeline
// ============================================================================

pub const CMD_PIPELINE_LIST: &str = r#"List all CodePipeline pipelines.

```bash
hu pipeline list          # List pipelines
hu pipeline list -j       # JSON output
```
"#;

pub const CMD_PIPELINE_STATUS: &str = r#"Show pipeline status (stages and actions).

```bash
hu pipeline status <pipeline-name>
hu pipeline status <pipeline-name> -j  # JSON output
```
"#;

pub const CMD_PIPELINE_HISTORY: &str = r#"Show pipeline execution history.

```bash
hu pipeline history <pipeline-name>
hu pipeline history <pipeline-name> -n 10  # Last 10 executions
```
"#;

// ============================================================================
// COMMANDS - Utils
// ============================================================================

pub const CMD_UTILS_FETCH_HTML: &str = r#"Fetch URL and convert to markdown.

```bash
hu utils fetch-html <url>              # Fetch and convert
hu utils fetch-html <url> -c           # Extra cleaning
hu utils fetch-html <url> -s "article" # Target CSS selector
hu utils fetch-html <url> -o out.md    # Write to file
```
"#;

pub const CMD_UTILS_GREP: &str = r#"Smart grep with token-saving options.

```bash
hu utils grep "pattern" path/          # Search in path
hu utils grep "pattern" -g "*.rs"      # Filter by glob
hu utils grep "pattern" --refs         # File paths only
hu utils grep "pattern" -n 20          # Limit results
```
"#;

pub const CMD_UTILS_WEB_SEARCH: &str = r#"Web search using Brave Search API.

```bash
hu utils web-search "query"            # Search web
hu utils web-search "query" -n 10      # Limit results
hu utils web-search "query" -j         # JSON output
```
"#;

pub const CMD_UTILS_DOCS_INDEX: &str = r#"Build heading index for markdown files.

```bash
hu utils docs-index docs/              # Build index
hu utils docs-index docs/ -o index.json  # Output to file
```
"#;

pub const CMD_UTILS_DOCS_SEARCH: &str = r#"Search docs index for matching sections.

```bash
hu utils docs-search "query" -i index.json
hu utils docs-search "authentication" -i docs-index.json
```
"#;

pub const CMD_UTILS_DOCS_SECTION: &str = r#"Extract a section from a markdown file by heading.

```bash
hu utils docs-section docs/api.md "Authentication"
hu utils docs-section README.md "Installation"
```
"#;

// ============================================================================
// COMMANDS - Data
// ============================================================================

pub const CMD_DATA_SYNC: &str = r#"Sync Claude Code data to local database.

```bash
hu data sync              # Incremental sync
hu data sync -f           # Force full sync
```
"#;

pub const CMD_DATA_CONFIG: &str = r#"Show data configuration.

```bash
hu data config            # Show config
hu data config -j         # JSON output
```
"#;

pub const CMD_DATA_STATS: &str = r#"Usage statistics.

```bash
hu data stats             # Show stats
hu data stats -j          # JSON output
```
"#;

pub const CMD_DATA_SEARCH: &str = r#"Search messages.

```bash
hu data search "query"    # Search messages
hu data search "error" -n 20  # Limit results
```
"#;

pub const CMD_DATA_TODOS: &str = r#"Todo operations.

```bash
hu data todos pending     # Show pending todos
hu data todos all         # Show all todos
hu data todos -j          # JSON output
```
"#;

pub const CMD_DATA_TOOLS: &str = r#"Tool usage statistics.

```bash
hu data tools             # Show tool usage
hu data tools -j          # JSON output
```
"#;

pub const CMD_DATA_ERRORS: &str = r#"Extract errors from debug logs.

```bash
hu data errors            # Show recent errors
hu data errors -n 50      # Last 50 errors
```
"#;

pub const CMD_DATA_PRICING: &str = r#"Pricing analysis.

```bash
hu data pricing           # Show pricing analysis
hu data pricing -j        # JSON output
```
"#;

pub const CMD_DATA_SESSION: &str = r#"Session operations.

```bash
hu data session list      # List sessions
hu data session list -p . # Filter by project
hu data session show <id> # Show session details
```
"#;

pub const CMD_DATA_BRANCHES: &str = r#"Branch activity statistics.

```bash
hu data branches          # Show branch stats
hu data branches -j       # JSON output
```
"#;

// ============================================================================
// COMMANDS - Install
// ============================================================================

pub const CMD_INSTALL_LIST: &str = r#"List available components.

```bash
hu install list           # List all components
```
"#;

pub const CMD_INSTALL_PREVIEW: &str = r#"Show what would be installed without making changes.

```bash
hu install preview              # Preview global install
hu install preview --local      # Preview local install
hu install preview --hooks-only # Preview hooks only
```
"#;

pub const CMD_INSTALL_RUN: &str = r#"Install hooks and commands to Claude Code.

```bash
hu install run                  # Install to ~/.claude (global)
hu install run --local          # Install to ./.claude (local)
hu install run --force          # Override modified files
hu install run --hooks-only     # Install only hooks
hu install run --commands-only  # Install only commands
hu install run hooks/hu/pre-read   # Install specific component
```
"#;

// ============================================================================
// COMPONENT REGISTRY
// ============================================================================

/// All available components
pub static COMPONENTS: &[Component] = &[
    // Hooks (6)
    Component {
        id: "hooks/hu/pre-read",
        kind: ComponentKind::Hook,
        description: "Prevents duplicate file reads, warns on large files",
        path: "hooks/hu/pre-read.sh",
        content: HOOK_PRE_READ,
    },
    Component {
        id: "hooks/hu/pre-grep",
        kind: ComponentKind::Hook,
        description: "Warns on broad patterns, suggests efficient modes",
        path: "hooks/hu/pre-grep.sh",
        content: HOOK_PRE_GREP,
    },
    Component {
        id: "hooks/hu/pre-webfetch",
        kind: ComponentKind::Hook,
        description: "Logs URL fetches, warns on HTTP",
        path: "hooks/hu/pre-webfetch.sh",
        content: HOOK_PRE_WEBFETCH,
    },
    Component {
        id: "hooks/hu/pre-websearch",
        kind: ComponentKind::Hook,
        description: "Logs searches, warns on sensitive terms",
        path: "hooks/hu/pre-websearch.sh",
        content: HOOK_PRE_WEBSEARCH,
    },
    Component {
        id: "hooks/hu/session-start",
        kind: ComponentKind::Hook,
        description: "Cleans old files, initializes context tracking",
        path: "hooks/hu/session-start.sh",
        content: HOOK_SESSION_START,
    },
    Component {
        id: "hooks/hu/session-end",
        kind: ComponentKind::Hook,
        description: "Shows context summary, clears tracking",
        path: "hooks/hu/session-end.sh",
        content: HOOK_SESSION_END,
    },
    // Context commands (4)
    Component {
        id: "commands/hu/context/track",
        kind: ComponentKind::Command,
        description: "Track files as loaded in context",
        path: "commands/hu/context/track.md",
        content: CMD_CONTEXT_TRACK,
    },
    Component {
        id: "commands/hu/context/check",
        kind: ComponentKind::Command,
        description: "Check if files are in context",
        path: "commands/hu/context/check.md",
        content: CMD_CONTEXT_CHECK,
    },
    Component {
        id: "commands/hu/context/summary",
        kind: ComponentKind::Command,
        description: "Show context summary",
        path: "commands/hu/context/summary.md",
        content: CMD_CONTEXT_SUMMARY,
    },
    Component {
        id: "commands/hu/context/clear",
        kind: ComponentKind::Command,
        description: "Clear context tracking",
        path: "commands/hu/context/clear.md",
        content: CMD_CONTEXT_CLEAR,
    },
    // Read command (1)
    Component {
        id: "commands/hu/read",
        kind: ComponentKind::Command,
        description: "Smart file reading modes",
        path: "commands/hu/read.md",
        content: CMD_READ,
    },
    // Jira commands (6)
    Component {
        id: "commands/hu/jira/auth",
        kind: ComponentKind::Command,
        description: "Authenticate with Jira",
        path: "commands/hu/jira/auth.md",
        content: CMD_JIRA_AUTH,
    },
    Component {
        id: "commands/hu/jira/tickets",
        kind: ComponentKind::Command,
        description: "List my tickets in sprint",
        path: "commands/hu/jira/tickets.md",
        content: CMD_JIRA_TICKETS,
    },
    Component {
        id: "commands/hu/jira/sprint",
        kind: ComponentKind::Command,
        description: "Show sprint issues",
        path: "commands/hu/jira/sprint.md",
        content: CMD_JIRA_SPRINT,
    },
    Component {
        id: "commands/hu/jira/search",
        kind: ComponentKind::Command,
        description: "Search tickets with JQL",
        path: "commands/hu/jira/search.md",
        content: CMD_JIRA_SEARCH,
    },
    Component {
        id: "commands/hu/jira/show",
        kind: ComponentKind::Command,
        description: "Show ticket details",
        path: "commands/hu/jira/show.md",
        content: CMD_JIRA_SHOW,
    },
    Component {
        id: "commands/hu/jira/update",
        kind: ComponentKind::Command,
        description: "Update a ticket",
        path: "commands/hu/jira/update.md",
        content: CMD_JIRA_UPDATE,
    },
    // GitHub commands (5)
    Component {
        id: "commands/hu/gh/login",
        kind: ComponentKind::Command,
        description: "Authenticate with GitHub",
        path: "commands/hu/gh/login.md",
        content: CMD_GH_LOGIN,
    },
    Component {
        id: "commands/hu/gh/prs",
        kind: ComponentKind::Command,
        description: "List pull requests",
        path: "commands/hu/gh/prs.md",
        content: CMD_GH_PRS,
    },
    Component {
        id: "commands/hu/gh/runs",
        kind: ComponentKind::Command,
        description: "List workflow runs",
        path: "commands/hu/gh/runs.md",
        content: CMD_GH_RUNS,
    },
    Component {
        id: "commands/hu/gh/failures",
        kind: ComponentKind::Command,
        description: "Extract CI test failures",
        path: "commands/hu/gh/failures.md",
        content: CMD_GH_FAILURES,
    },
    Component {
        id: "commands/hu/gh/fix",
        kind: ComponentKind::Command,
        description: "Analyze CI failures",
        path: "commands/hu/gh/fix.md",
        content: CMD_GH_FIX,
    },
    // Slack commands (10)
    Component {
        id: "commands/hu/slack/auth",
        kind: ComponentKind::Command,
        description: "Authenticate with Slack",
        path: "commands/hu/slack/auth.md",
        content: CMD_SLACK_AUTH,
    },
    Component {
        id: "commands/hu/slack/channels",
        kind: ComponentKind::Command,
        description: "List channels",
        path: "commands/hu/slack/channels.md",
        content: CMD_SLACK_CHANNELS,
    },
    Component {
        id: "commands/hu/slack/info",
        kind: ComponentKind::Command,
        description: "Show channel info",
        path: "commands/hu/slack/info.md",
        content: CMD_SLACK_INFO,
    },
    Component {
        id: "commands/hu/slack/send",
        kind: ComponentKind::Command,
        description: "Send message",
        path: "commands/hu/slack/send.md",
        content: CMD_SLACK_SEND,
    },
    Component {
        id: "commands/hu/slack/history",
        kind: ComponentKind::Command,
        description: "Show message history",
        path: "commands/hu/slack/history.md",
        content: CMD_SLACK_HISTORY,
    },
    Component {
        id: "commands/hu/slack/search",
        kind: ComponentKind::Command,
        description: "Search messages",
        path: "commands/hu/slack/search.md",
        content: CMD_SLACK_SEARCH,
    },
    Component {
        id: "commands/hu/slack/users",
        kind: ComponentKind::Command,
        description: "List users",
        path: "commands/hu/slack/users.md",
        content: CMD_SLACK_USERS,
    },
    Component {
        id: "commands/hu/slack/config",
        kind: ComponentKind::Command,
        description: "Show Slack config",
        path: "commands/hu/slack/config.md",
        content: CMD_SLACK_CONFIG,
    },
    Component {
        id: "commands/hu/slack/whoami",
        kind: ComponentKind::Command,
        description: "Show current user",
        path: "commands/hu/slack/whoami.md",
        content: CMD_SLACK_WHOAMI,
    },
    Component {
        id: "commands/hu/slack/tidy",
        kind: ComponentKind::Command,
        description: "Mark channels as read",
        path: "commands/hu/slack/tidy.md",
        content: CMD_SLACK_TIDY,
    },
    // PagerDuty commands (7)
    Component {
        id: "commands/hu/pagerduty/config",
        kind: ComponentKind::Command,
        description: "Show PagerDuty config",
        path: "commands/hu/pagerduty/config.md",
        content: CMD_PD_CONFIG,
    },
    Component {
        id: "commands/hu/pagerduty/auth",
        kind: ComponentKind::Command,
        description: "Set API token",
        path: "commands/hu/pagerduty/auth.md",
        content: CMD_PD_AUTH,
    },
    Component {
        id: "commands/hu/pagerduty/oncall",
        kind: ComponentKind::Command,
        description: "Show on-call schedule",
        path: "commands/hu/pagerduty/oncall.md",
        content: CMD_PD_ONCALL,
    },
    Component {
        id: "commands/hu/pagerduty/alerts",
        kind: ComponentKind::Command,
        description: "List active alerts",
        path: "commands/hu/pagerduty/alerts.md",
        content: CMD_PD_ALERTS,
    },
    Component {
        id: "commands/hu/pagerduty/incidents",
        kind: ComponentKind::Command,
        description: "List incidents",
        path: "commands/hu/pagerduty/incidents.md",
        content: CMD_PD_INCIDENTS,
    },
    Component {
        id: "commands/hu/pagerduty/show",
        kind: ComponentKind::Command,
        description: "Show incident details",
        path: "commands/hu/pagerduty/show.md",
        content: CMD_PD_SHOW,
    },
    Component {
        id: "commands/hu/pagerduty/whoami",
        kind: ComponentKind::Command,
        description: "Show current user",
        path: "commands/hu/pagerduty/whoami.md",
        content: CMD_PD_WHOAMI,
    },
    // Sentry commands (5)
    Component {
        id: "commands/hu/sentry/config",
        kind: ComponentKind::Command,
        description: "Show Sentry config",
        path: "commands/hu/sentry/config.md",
        content: CMD_SENTRY_CONFIG,
    },
    Component {
        id: "commands/hu/sentry/auth",
        kind: ComponentKind::Command,
        description: "Set auth token",
        path: "commands/hu/sentry/auth.md",
        content: CMD_SENTRY_AUTH,
    },
    Component {
        id: "commands/hu/sentry/issues",
        kind: ComponentKind::Command,
        description: "List issues",
        path: "commands/hu/sentry/issues.md",
        content: CMD_SENTRY_ISSUES,
    },
    Component {
        id: "commands/hu/sentry/show",
        kind: ComponentKind::Command,
        description: "Show issue details",
        path: "commands/hu/sentry/show.md",
        content: CMD_SENTRY_SHOW,
    },
    Component {
        id: "commands/hu/sentry/events",
        kind: ComponentKind::Command,
        description: "List issue events",
        path: "commands/hu/sentry/events.md",
        content: CMD_SENTRY_EVENTS,
    },
    // NewRelic commands (5)
    Component {
        id: "commands/hu/newrelic/config",
        kind: ComponentKind::Command,
        description: "Show NewRelic config",
        path: "commands/hu/newrelic/config.md",
        content: CMD_NR_CONFIG,
    },
    Component {
        id: "commands/hu/newrelic/auth",
        kind: ComponentKind::Command,
        description: "Set API key",
        path: "commands/hu/newrelic/auth.md",
        content: CMD_NR_AUTH,
    },
    Component {
        id: "commands/hu/newrelic/issues",
        kind: ComponentKind::Command,
        description: "List issues",
        path: "commands/hu/newrelic/issues.md",
        content: CMD_NR_ISSUES,
    },
    Component {
        id: "commands/hu/newrelic/incidents",
        kind: ComponentKind::Command,
        description: "List incidents",
        path: "commands/hu/newrelic/incidents.md",
        content: CMD_NR_INCIDENTS,
    },
    Component {
        id: "commands/hu/newrelic/query",
        kind: ComponentKind::Command,
        description: "Run NRQL query",
        path: "commands/hu/newrelic/query.md",
        content: CMD_NR_QUERY,
    },
    // EKS commands (3)
    Component {
        id: "commands/hu/eks/list",
        kind: ComponentKind::Command,
        description: "List pods",
        path: "commands/hu/eks/list.md",
        content: CMD_EKS_LIST,
    },
    Component {
        id: "commands/hu/eks/exec",
        kind: ComponentKind::Command,
        description: "Execute in pod",
        path: "commands/hu/eks/exec.md",
        content: CMD_EKS_EXEC,
    },
    Component {
        id: "commands/hu/eks/logs",
        kind: ComponentKind::Command,
        description: "Tail pod logs",
        path: "commands/hu/eks/logs.md",
        content: CMD_EKS_LOGS,
    },
    // Pipeline commands (3)
    Component {
        id: "commands/hu/pipeline/list",
        kind: ComponentKind::Command,
        description: "List pipelines",
        path: "commands/hu/pipeline/list.md",
        content: CMD_PIPELINE_LIST,
    },
    Component {
        id: "commands/hu/pipeline/status",
        kind: ComponentKind::Command,
        description: "Show pipeline status",
        path: "commands/hu/pipeline/status.md",
        content: CMD_PIPELINE_STATUS,
    },
    Component {
        id: "commands/hu/pipeline/history",
        kind: ComponentKind::Command,
        description: "Show execution history",
        path: "commands/hu/pipeline/history.md",
        content: CMD_PIPELINE_HISTORY,
    },
    // Utils commands (6)
    Component {
        id: "commands/hu/utils/fetch-html",
        kind: ComponentKind::Command,
        description: "Fetch URL as markdown",
        path: "commands/hu/utils/fetch-html.md",
        content: CMD_UTILS_FETCH_HTML,
    },
    Component {
        id: "commands/hu/utils/grep",
        kind: ComponentKind::Command,
        description: "Smart grep",
        path: "commands/hu/utils/grep.md",
        content: CMD_UTILS_GREP,
    },
    Component {
        id: "commands/hu/utils/web-search",
        kind: ComponentKind::Command,
        description: "Web search",
        path: "commands/hu/utils/web-search.md",
        content: CMD_UTILS_WEB_SEARCH,
    },
    Component {
        id: "commands/hu/utils/docs-index",
        kind: ComponentKind::Command,
        description: "Build docs index",
        path: "commands/hu/utils/docs-index.md",
        content: CMD_UTILS_DOCS_INDEX,
    },
    Component {
        id: "commands/hu/utils/docs-search",
        kind: ComponentKind::Command,
        description: "Search docs index",
        path: "commands/hu/utils/docs-search.md",
        content: CMD_UTILS_DOCS_SEARCH,
    },
    Component {
        id: "commands/hu/utils/docs-section",
        kind: ComponentKind::Command,
        description: "Extract doc section",
        path: "commands/hu/utils/docs-section.md",
        content: CMD_UTILS_DOCS_SECTION,
    },
    // Data commands (10)
    Component {
        id: "commands/hu/data/sync",
        kind: ComponentKind::Command,
        description: "Sync Claude data",
        path: "commands/hu/data/sync.md",
        content: CMD_DATA_SYNC,
    },
    Component {
        id: "commands/hu/data/config",
        kind: ComponentKind::Command,
        description: "Show data config",
        path: "commands/hu/data/config.md",
        content: CMD_DATA_CONFIG,
    },
    Component {
        id: "commands/hu/data/stats",
        kind: ComponentKind::Command,
        description: "Usage statistics",
        path: "commands/hu/data/stats.md",
        content: CMD_DATA_STATS,
    },
    Component {
        id: "commands/hu/data/search",
        kind: ComponentKind::Command,
        description: "Search messages",
        path: "commands/hu/data/search.md",
        content: CMD_DATA_SEARCH,
    },
    Component {
        id: "commands/hu/data/todos",
        kind: ComponentKind::Command,
        description: "Todo operations",
        path: "commands/hu/data/todos.md",
        content: CMD_DATA_TODOS,
    },
    Component {
        id: "commands/hu/data/tools",
        kind: ComponentKind::Command,
        description: "Tool usage stats",
        path: "commands/hu/data/tools.md",
        content: CMD_DATA_TOOLS,
    },
    Component {
        id: "commands/hu/data/errors",
        kind: ComponentKind::Command,
        description: "Extract errors",
        path: "commands/hu/data/errors.md",
        content: CMD_DATA_ERRORS,
    },
    Component {
        id: "commands/hu/data/pricing",
        kind: ComponentKind::Command,
        description: "Pricing analysis",
        path: "commands/hu/data/pricing.md",
        content: CMD_DATA_PRICING,
    },
    Component {
        id: "commands/hu/data/session",
        kind: ComponentKind::Command,
        description: "Session operations",
        path: "commands/hu/data/session.md",
        content: CMD_DATA_SESSION,
    },
    Component {
        id: "commands/hu/data/branches",
        kind: ComponentKind::Command,
        description: "Branch statistics",
        path: "commands/hu/data/branches.md",
        content: CMD_DATA_BRANCHES,
    },
    // Install commands (3)
    Component {
        id: "commands/hu/install/list",
        kind: ComponentKind::Command,
        description: "List components",
        path: "commands/hu/install/list.md",
        content: CMD_INSTALL_LIST,
    },
    Component {
        id: "commands/hu/install/preview",
        kind: ComponentKind::Command,
        description: "Preview install",
        path: "commands/hu/install/preview.md",
        content: CMD_INSTALL_PREVIEW,
    },
    Component {
        id: "commands/hu/install/run",
        kind: ComponentKind::Command,
        description: "Run install",
        path: "commands/hu/install/run.md",
        content: CMD_INSTALL_RUN,
    },
];

/// Get components filtered by kind
pub fn get_components(include_hooks: bool, include_commands: bool) -> Vec<&'static Component> {
    COMPONENTS
        .iter()
        .filter(|c| match c.kind {
            ComponentKind::Hook => include_hooks,
            ComponentKind::Command => include_commands,
        })
        .collect()
}

/// Get hooks only
pub fn get_hooks() -> Vec<&'static Component> {
    get_components(true, false)
}

/// Get commands only
pub fn get_commands() -> Vec<&'static Component> {
    get_components(false, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn components_count() {
        assert_eq!(COMPONENTS.len(), 74); // 6 hooks + 68 commands
    }

    #[test]
    fn hooks_count() {
        assert_eq!(get_hooks().len(), 6);
    }

    #[test]
    fn commands_count() {
        assert_eq!(get_commands().len(), 68);
    }

    #[test]
    fn all_hooks_have_sh_extension() {
        for c in get_hooks() {
            assert!(c.path.ends_with(".sh"), "Hook {} should end with .sh", c.id);
        }
    }

    #[test]
    fn all_commands_have_md_extension() {
        for c in get_commands() {
            assert!(
                c.path.ends_with(".md"),
                "Command {} should end with .md",
                c.id
            );
        }
    }

    #[test]
    fn all_hooks_have_shebang() {
        for c in get_hooks() {
            assert!(
                c.content.starts_with("#!/bin/bash"),
                "Hook {} should start with shebang",
                c.id
            );
        }
    }

    #[test]
    fn unique_component_ids() {
        let mut ids: Vec<_> = COMPONENTS.iter().map(|c| c.id).collect();
        ids.sort();
        let original_len = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), original_len, "Component IDs must be unique");
    }

    #[test]
    fn unique_component_paths() {
        let mut paths: Vec<_> = COMPONENTS.iter().map(|c| c.path).collect();
        paths.sort();
        let original_len = paths.len();
        paths.dedup();
        assert_eq!(paths.len(), original_len, "Component paths must be unique");
    }
}
