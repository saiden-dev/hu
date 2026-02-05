# Token Savings Analysis

Comparison of hu CLI tools vs Claude Code built-in tools and MCP commands.

## Why CLI Over MCP?

**CLI tools are the most token-efficient way to extend Claude Code.**

MCP servers have significant hidden costs:
1. **Schema overhead**: Every MCP tool requires its full JSON schema loaded into context
2. **Multiplicative cost**: 10 tools × 200-300 tokens = 2-3k tokens before any work
3. **Per-session cost**: This overhead is paid every session, every conversation
4. **No composability**: Can't pipe MCP outputs or combine with shell tools

CLI tools like `hu` have **zero context overhead**:
- Claude already knows how to run shell commands
- Self-documenting via `--help` (only loaded when needed)
- Full Unix composability (pipes, redirects, xargs)
- Plain text errors (no JSON parsing overhead)

| Factor | MCP | CLI (hu) |
|--------|-----|----------|
| Schema overhead | 2-3k/session | 0 |
| Self-documenting | No | Yes (`--help`) |
| Composable | Limited | Full (pipes) |
| Error format | Wrapped JSON | Plain text |
| Learning curve | New protocol | Standard shell |

**Bottom line**: An MCP server with 20 tools costs ~4-6k tokens per session just to exist. That's equivalent to reading a 300-line file—wasted on tool definitions you might not even use.

## Key Insight

**Reasoning is cheap, I/O is expensive.**

| Operation | Token Cost |
|-----------|------------|
| Planning/reasoning | ~500-2k (output) |
| Reading a 500-line file | ~5-15k (input) |
| Reading 10 files | ~50-150k |
| MCP tool schemas | ~2-3k per session |

## Tool Comparison

### File Reading

| Approach | Tokens | Notes |
|----------|--------|-------|
| **Claude Read** (full file) | ~5-15k | Entire file in context |
| **hu read --outline** | ~200-500 | Functions/structs only |
| **hu read --interface** | ~300-800 | Public API only |
| **hu read --around N** | ~100-300 | Targeted context window |
| **Savings** | **10-50x** | |

### Code Search

| Approach | Tokens | Notes |
|----------|--------|-------|
| **Claude Grep** (content mode) | ~2-10k | Full matching lines |
| **hu utils grep --refs** | ~100-300 | File:line refs only |
| **Explore agent** (37 tools) | ~70k | Full exploration |
| **Savings** | **5-20x** | |

### Web Fetch

| Approach | Tokens | Notes |
|----------|--------|-------|
| **WebFetch** (full page) | ~10-50k | All HTML converted |
| **hu utils fetch-html -c** | ~2-10k | Cleaned content |
| **hu utils fetch-html -s** | ~1-5k | CSS selector target |
| **Savings** | **5-10x** | |

### Documentation Lookup

| Approach | Tokens | Notes |
|----------|--------|-------|
| **Read full doc file** | ~5-20k | Entire document |
| **hu utils docs-section** | ~200-1k | Specific heading only |
| **hu utils docs-search** | ~100-300 | Matching sections |
| **Savings** | **10-20x** | |

### Context Tracking

| Approach | Tokens | Notes |
|----------|--------|-------|
| **Re-read same file** | ~5-15k | 100% waste |
| **hu context check** | 0 | Already in context |
| **Savings** | **100%** | Prevents duplicates |

## Hook-Based Savings

Automatic optimizations via `~/.claude/hooks/`:

### pre-read.sh
- Checks if file already in context → skips re-read
- Warns for files >500 lines → suggests `--outline`
- Tracks files as loaded

**Estimated savings:** 100% of duplicate reads

### session-start.sh
- Cleans debug files >7 days (reduces I/O latency)
- Initializes context tracking
- Builds docs index in background

**Estimated savings:** Reduced latency, prepared indexes

### session-end.sh
- Clears context tracking
- Removes temp index files

## Summary: Token Waste Prevention

| Pattern | Savings |
|---------|---------|
| CLI tools vs MCP schemas | 2-6k/session |
| `hu read --outline` instead of full read | 10-50x |
| `hu context check` before read | 100% of dupes |
| `hu utils docs-section` vs full doc | 10-20x |
| `hu utils fetch-html -c` vs full page | 5-10x |

**Estimated total savings: 40-60% of typical session tokens.**

## Current Implementation Status

### Fully Implemented (Rust CLI)

**Core Utilities**
- `hu read --outline/--interface/--around/--diff` - Smart file reading
- `hu context track/check/summary/clear` - Context tracking
- `hu utils fetch-html` - Web content extraction
- `hu utils grep` - Token-efficient code search
- `hu utils web-search` - Brave Search integration
- `hu utils docs-index/docs-search/docs-section` - Documentation indexing

**Service Integrations**
- `hu jira` - Tickets, sprints, search, updates (OAuth 2.0)
- `hu gh` - PRs, runs, failures, fix (CI analysis)
- `hu slack` - Channels, messages, search, tidy
- `hu pagerduty` - Oncall, alerts, incidents
- `hu sentry` - Issues, events
- `hu newrelic` - Issues, incidents, NRQL queries
- `hu pipeline` - AWS CodePipeline status
- `hu eks` - Pod list, exec, logs

**Analytics**
- `hu data sync` - Claude Code session data to SQLite
- `hu data stats` - Usage statistics
- `hu data search` - Full-text message search
- `hu data tools` - Tool usage analysis
- `hu data pricing` - Cost analysis vs API
- `hu data branches` - Activity by git branch

### Hooks Active
- `pre-read.sh` - Context tracking, large file warnings
- `session-start.sh` - Cleanup, index building
- `session-end.sh` - Context cleanup
