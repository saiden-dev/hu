use crate::install::types::{Component, ComponentKind};

// Embed hook scripts
pub const HOOK_PRE_READ: &str = r#"#!/bin/bash
# Pre-Read Hook: Prevent duplicate reads and auto-truncate large files
# Trigger: Before Claude's Read tool executes
#
# Token savings: Prevents 100% of duplicate file reads

set -euo pipefail

# Skip if disabled
[[ "${HU_SKIP_HOOKS:-}" == "1" ]] && exit 0

# Configuration
MAX_LINES="${HU_MAX_FILE_LINES:-500}"
CONTEXT_FILE="${HU_CONTEXT_FILE:-/tmp/hu-context-${CLAUDE_SESSION_ID:-default}.json}"

# Parse input (Claude passes tool args as JSON on stdin)
INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.file_path // empty')

[[ -z "$FILE_PATH" ]] && exit 0

# Check if file is already in context
if command -v hu &>/dev/null; then
    STATUS=$(hu context check "$FILE_PATH" 2>/dev/null || echo "")

    if echo "$STATUS" | grep -q "loaded"; then
        # File already in context - return early
        AGO=$(echo "$STATUS" | grep -oE '[0-9]+ (seconds?|minutes?|hours?) ago' || echo "recently")
        echo "File already in context (loaded $AGO). Use --force to re-read."
        exit 0
    fi
fi

# Check file size
if [[ -f "$FILE_PATH" ]]; then
    LINE_COUNT=$(wc -l < "$FILE_PATH" 2>/dev/null || echo "0")

    if [[ "$LINE_COUNT" -gt "$MAX_LINES" ]]; then
        echo "Warning: $FILE_PATH has $LINE_COUNT lines (>${MAX_LINES} threshold)."
        echo "Consider using:"
        echo "  hu read '$FILE_PATH' --outline    # Structure only"
        echo "  hu read '$FILE_PATH' --interface  # Public API only"
        echo "  hu read '$FILE_PATH' --around N   # Lines around line N"
        echo ""
        echo "Proceeding with full read..."
    fi

    # Track the file as being read
    if command -v hu &>/dev/null; then
        hu context track "$FILE_PATH" 2>/dev/null || true
    fi
fi

# Allow the read to proceed
exit 0
"#;

pub const HOOK_PRE_GREP: &str = r#"#!/bin/bash
# Pre-Grep Hook: Limit runaway searches and suggest efficient modes
# Trigger: Before Claude's Grep tool executes
#
# Token savings: 5-20x for broad searches

set -euo pipefail

# Skip if disabled
[[ "${HU_SKIP_HOOKS:-}" == "1" ]] && exit 0

# Configuration
MAX_RESULTS="${HU_MAX_GREP_RESULTS:-20}"

# Parse input
INPUT=$(cat)
PATTERN=$(echo "$INPUT" | jq -r '.pattern // empty')
PATH_ARG=$(echo "$INPUT" | jq -r '.path // "."')
HEAD_LIMIT=$(echo "$INPUT" | jq -r '.head_limit // 0')

[[ -z "$PATTERN" ]] && exit 0

# Warn about overly broad patterns
BROAD_PATTERNS=(
    "^.$"           # Single character
    "^..$"          # Two characters
    "the"
    "function"
    "import"
    "return"
    "if"
    "for"
    "var"
    "let"
    "const"
)

PATTERN_LOWER=$(echo "$PATTERN" | tr '[:upper:]' '[:lower:]')
for BROAD in "${BROAD_PATTERNS[@]}"; do
    if [[ "$PATTERN_LOWER" =~ $BROAD ]]; then
        echo "Warning: Pattern '$PATTERN' may match many results."
        echo "Consider:"
        echo "  - More specific pattern"
        echo "  - Add --glob '*.rs' to limit file types"
        echo "  - Use head_limit to cap results"
        echo ""
        break
    fi
done

# Suggest --refs mode for exploratory searches
if [[ "$HEAD_LIMIT" == "0" ]] || [[ "$HEAD_LIMIT" -gt "$MAX_RESULTS" ]]; then
    echo "Tip: For exploratory searches, use output_mode: 'files_with_matches'"
    echo "     to get file paths only, then read specific files."
    echo ""
fi

# Allow grep to proceed
exit 0
"#;

pub const HOOK_PRE_WEBFETCH: &str = r#"#!/bin/bash
# Pre-WebFetch Hook: Log and validate URL fetches
# Trigger: Before Claude's WebFetch tool executes

set -euo pipefail

# Skip if disabled
[[ "${HU_SKIP_HOOKS:-}" == "1" ]] && exit 0

# Parse input (Claude passes tool args as JSON on stdin)
INPUT=$(cat)
URL=$(echo "$INPUT" | jq -r '.url // empty')
PROMPT=$(echo "$INPUT" | jq -r '.prompt // empty')

[[ -z "$URL" ]] && exit 0

# Log the fetch
echo "WebFetch: $URL"
[[ -n "$PROMPT" ]] && echo "  Prompt: ${PROMPT:0:60}..."

# Extract domain
DOMAIN=$(echo "$URL" | sed -E 's|https?://([^/]+).*|\1|')

# Warn about non-HTTPS
if [[ "$URL" == http://* ]]; then
    echo "Warning: Using HTTP instead of HTTPS"
fi

# Log domain for tracking
echo "  Domain: $DOMAIN"

# Allow the fetch to proceed
exit 0
"#;

pub const HOOK_PRE_WEBSEARCH: &str = r#"#!/bin/bash
# Pre-WebSearch Hook: Log and validate web searches
# Trigger: Before Claude's WebSearch tool executes

set -euo pipefail

# Skip if disabled
[[ "${HU_SKIP_HOOKS:-}" == "1" ]] && exit 0

# Parse input (Claude passes tool args as JSON on stdin)
INPUT=$(cat)
QUERY=$(echo "$INPUT" | jq -r '.query // empty')

[[ -z "$QUERY" ]] && exit 0

# Log the search
echo "WebSearch: \"$QUERY\""

# Warn about very short queries
if [[ ${#QUERY} -lt 5 ]]; then
    echo "Warning: Query is very short. Consider being more specific."
fi

# Warn about potentially sensitive searches
SENSITIVE_PATTERNS=(
    "password"
    "api.key"
    "secret"
    "credential"
    "token"
)

QUERY_LOWER=$(echo "$QUERY" | tr '[:upper:]' '[:lower:]')
for PATTERN in "${SENSITIVE_PATTERNS[@]}"; do
    if [[ "$QUERY_LOWER" == *"$PATTERN"* ]]; then
        echo "Warning: Query contains potentially sensitive term '$PATTERN'"
        break
    fi
done

# Allow the search to proceed
exit 0
"#;

pub const HOOK_SESSION_START: &str = r#"#!/bin/bash
# Session-Start Hook: Initialize context tracking and cleanup old files
# Trigger: When a Claude Code session begins
#
# Token savings: Reduces latency, prepares index

set -euo pipefail

# Skip if disabled
[[ "${HU_SKIP_HOOKS:-}" == "1" ]] && exit 0

# Configuration
CLEANUP_DAYS="${HU_CLEANUP_DAYS:-7}"
CLAUDE_DIR="${HOME}/.claude"

# --- Cleanup old files ---

# Debug files (can grow to 400+ MB)
if [[ -d "$CLAUDE_DIR/debug" ]]; then
    find "$CLAUDE_DIR/debug" -type f -mtime +$CLEANUP_DAYS -delete 2>/dev/null || true
    echo "Cleaned debug files older than $CLEANUP_DAYS days"
fi

# Shell snapshots
if [[ -d "$CLAUDE_DIR/shell-snapshots" ]]; then
    find "$CLAUDE_DIR/shell-snapshots" -type f -mtime +$CLEANUP_DAYS -delete 2>/dev/null || true
fi

# Paste cache
if [[ -d "$CLAUDE_DIR/paste-cache" ]]; then
    find "$CLAUDE_DIR/paste-cache" -type f -mtime +$CLEANUP_DAYS -delete 2>/dev/null || true
fi

# --- Initialize context tracking ---

if command -v hu &>/dev/null; then
    # Clear any stale context from previous sessions
    hu context clear 2>/dev/null || true
    echo "Context tracking initialized"
fi

# --- Build/update code index (optional, for faster lookups) ---

# Only if in a git repo with markdown docs
if [[ -d ".git" ]] && [[ -d "doc" || -d "docs" ]]; then
    DOC_DIR="doc"
    [[ -d "docs" ]] && DOC_DIR="docs"

    if command -v hu &>/dev/null; then
        # Build index in background (don't block session start)
        INDEX_FILE="/tmp/hu-docs-index-$(basename "$(pwd)").json"
        (hu utils docs-index "$DOC_DIR" -o "$INDEX_FILE" 2>/dev/null &)
        echo "Building docs index in background: $INDEX_FILE"
    fi
fi

# --- Summary ---

echo ""
echo "Session initialized. Token-saving features active:"
echo "  - Context tracking: prevents duplicate file reads"
echo "  - Large file warnings: suggests --outline/--around"
echo "  - Grep limits: caps broad searches"
echo ""
echo "Bypass with: HU_SKIP_HOOKS=1"

exit 0
"#;

pub const HOOK_SESSION_END: &str = r#"#!/bin/bash
# Session-End Hook: Cleanup context tracking
# Trigger: When a Claude Code session ends
#
# Keeps context files from accumulating

set -euo pipefail

# Skip if disabled
[[ "${HU_SKIP_HOOKS:-}" == "1" ]] && exit 0

# --- Clear context tracking ---

if command -v hu &>/dev/null; then
    # Get summary before clearing (for logging)
    SUMMARY=$(hu context summary 2>/dev/null || echo "")

    if [[ -n "$SUMMARY" ]] && ! echo "$SUMMARY" | grep -q "No files"; then
        echo "Session context summary:"
        echo "$SUMMARY"
        echo ""
    fi

    # Clear the context
    hu context clear 2>/dev/null || true
fi

# --- Remove temp index files ---

INDEX_FILE="/tmp/hu-docs-index-$(basename "$(pwd)").json"
[[ -f "$INDEX_FILE" ]] && rm -f "$INDEX_FILE"

echo "Session cleanup complete"
exit 0
"#;

// Command templates
pub const CMD_CONTEXT_TRACK: &str = r#"Track file(s) as loaded in context.

## Usage

```bash
hu context track <file>...
```

## Arguments

| Arg | Description |
|-----|-------------|
| `FILE` | File path(s) to mark as loaded |

## Example

```bash
hu context track src/main.rs src/lib.rs
```

Prevents duplicate reads of the same files in a session.
"#;

pub const CMD_CONTEXT_CHECK: &str = r#"Check if file(s) are already in context.

## Usage

```bash
hu context check <file>...
```

## Arguments

| Arg | Description |
|-----|-------------|
| `FILE` | File path(s) to check |

## Example

```bash
hu context check src/main.rs
# Output: src/main.rs: loaded 5 minutes ago
```
"#;

pub const CMD_CONTEXT_SUMMARY: &str = r#"Show summary of all tracked files in context.

## Usage

```bash
hu context summary
```

Shows all files currently tracked in the session context with load times.
"#;

pub const CMD_CONTEXT_CLEAR: &str = r#"Clear all tracked files from context.

## Usage

```bash
hu context clear
```

Resets the context tracking. Use at session end or to force re-reads.
"#;

pub const CMD_READ: &str = r#"Smart file reading with outline, interface, around, and diff modes.

## Usage

```bash
hu read src/main.rs                        # Full file
hu read src/main.rs -o                     # Outline (functions, structs, classes)
hu read src/main.rs -i                     # Public interface only
hu read src/main.rs -a 42                  # Lines around line 42
hu read src/main.rs -a 42 -n 20           # 20 context lines around line 42
hu read src/main.rs -d                     # Git diff (vs HEAD)
hu read src/main.rs -d --commit abc123     # Diff against specific commit
```

## Arguments

| Arg | Description |
|-----|-------------|
| `PATH` | File path to read (required) |

## Options

| Flag | Description |
|------|-------------|
| `-o, --outline` | Show file outline (functions, structs, classes) |
| `-i, --interface` | Public interface only (pub items in Rust, exports in JS) |
| `-a, --around` | Show lines around a specific line number |
| `-n, --context` | Context lines for `--around` (default: 10) |
| `-d, --diff` | Show git diff |
| `--commit` | Commit to diff against (default: HEAD) |

## Modes

### Outline (`-o`)
Shows structure: function signatures, struct/class definitions, impl blocks.

### Interface (`-i`)
Shows only public API: `pub fn`, `pub struct`, `pub enum`, exports.

### Around (`-a`)
Shows context around a specific line, useful for investigating errors at known line numbers.

### Diff (`-d`)
Shows git changes, optionally against a specific commit.
"#;

/// All available components
pub static COMPONENTS: &[Component] = &[
    // Hooks
    Component {
        id: "hooks/pre-read",
        kind: ComponentKind::Hook,
        description: "Prevents duplicate file reads, warns on large files",
        path: "hooks/pre-read.sh",
        content: HOOK_PRE_READ,
    },
    Component {
        id: "hooks/pre-grep",
        kind: ComponentKind::Hook,
        description: "Warns on broad patterns, suggests efficient modes",
        path: "hooks/pre-grep.sh",
        content: HOOK_PRE_GREP,
    },
    Component {
        id: "hooks/pre-webfetch",
        kind: ComponentKind::Hook,
        description: "Logs URL fetches, warns on HTTP",
        path: "hooks/pre-webfetch.sh",
        content: HOOK_PRE_WEBFETCH,
    },
    Component {
        id: "hooks/pre-websearch",
        kind: ComponentKind::Hook,
        description: "Logs searches, warns on sensitive terms",
        path: "hooks/pre-websearch.sh",
        content: HOOK_PRE_WEBSEARCH,
    },
    Component {
        id: "hooks/session-start",
        kind: ComponentKind::Hook,
        description: "Cleans old files, initializes context tracking",
        path: "hooks/session-start.sh",
        content: HOOK_SESSION_START,
    },
    Component {
        id: "hooks/session-end",
        kind: ComponentKind::Hook,
        description: "Shows context summary, clears tracking",
        path: "hooks/session-end.sh",
        content: HOOK_SESSION_END,
    },
    // Commands
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
    Component {
        id: "commands/hu/read",
        kind: ComponentKind::Command,
        description: "Smart file reading modes",
        path: "commands/hu/read.md",
        content: CMD_READ,
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
    fn components_not_empty() {
        assert!(!COMPONENTS.is_empty());
    }

    #[test]
    fn get_hooks_returns_only_hooks() {
        let hooks = get_hooks();
        assert!(!hooks.is_empty());
        for h in hooks {
            assert_eq!(h.kind, ComponentKind::Hook);
        }
    }

    #[test]
    fn get_commands_returns_only_commands() {
        let commands = get_commands();
        assert!(!commands.is_empty());
        for c in commands {
            assert_eq!(c.kind, ComponentKind::Command);
        }
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
}
