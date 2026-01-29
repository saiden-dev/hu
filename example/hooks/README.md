# Token-Saving Hooks

Example Claude Code hooks that reduce token waste automatically.

## Installation

Copy to your Claude config:
```bash
cp -r example/hooks ~/.claude/hooks/
```

Or symlink for development:
```bash
ln -sf $(pwd)/example/hooks/* ~/.claude/hooks/
```

## Hooks Overview

| Hook | Trigger | Token Savings |
|------|---------|---------------|
| `pre-read.sh` | Before file read | Prevents re-reads, truncates large files |
| `pre-grep.sh` | Before grep | Limits runaway searches |
| `session-start.sh` | Session begins | Builds code index, cleans old files |
| `session-end.sh` | Session ends | Clears context tracking |

## Configuration

Set environment variables to customize:

```bash
# In ~/.zshrc or ~/.bashrc
export HU_CONTEXT_FILE="/tmp/hu-context-${CLAUDE_SESSION_ID:-default}.json"
export HU_MAX_FILE_LINES=500      # Auto-truncate threshold
export HU_MAX_GREP_RESULTS=20     # Grep result limit
export HU_CLEANUP_DAYS=7          # Days before cleanup
```

## How They Work

### Pre-Read Hook

Before Claude reads a file:
1. Checks if file already in context (via `hu context check`)
2. If loaded recently, returns "already in context" instead of content
3. If file > 500 lines, suggests `--outline` or `--around` instead
4. Tracks successfully read files

### Pre-Grep Hook

Before Claude runs grep:
1. Enforces max 20 results by default
2. Warns if pattern looks too broad (single char, common words)
3. Suggests `--refs` mode for exploratory searches

### Session-Start Hook

When a Claude session begins:
1. Cleans debug files older than 7 days
2. Builds/updates code index for current project
3. Initializes context tracking

### Session-End Hook

When a Claude session ends:
1. Clears context tracking file
2. Logs session token usage (if available)

## Bypass

Hooks can be bypassed when needed:

```bash
# Read without context check
HU_SKIP_HOOKS=1 claude "read the file anyway"

# Or use tool flags
hu read file.rs --raw  # Bypasses truncation
hu grep pattern --no-limit  # Bypasses result limit
```
