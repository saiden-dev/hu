# Token Economics in AI Agents

Observations on where tokens actually go and how to optimize.

## The Counterintuitive Reality

**Reasoning is cheap, I/O is expensive.**

| Operation | Token Cost |
|-----------|------------|
| Planning/reasoning | ~500-2k (output tokens) |
| Reading a 500-line file | ~5-15k (input tokens) |
| Reading 10 files | ~50-150k tokens |
| 37 tool uses (Explore agent) | ~70k tokens (~1.9k/operation) |

Complex planning and decision-making uses fewer tokens than a simple file read.

## MCP vs CLI Tools

MCP (Model Context Protocol) has a hidden cost:

- Every MCP tool requires its full JSON schema in context
- A dozen tools can consume 2-3k tokens before any work happens
- CLI tools are "zero context" - the model just needs to know `tool --help` exists

**CLI advantages:**
- Self-documenting (`--help`)
- Composable (pipes, redirects)
- No protocol overhead
- Works with any agent that can run bash
- Errors are plain text, not wrapped in protocol layers

**MCP's theoretical benefits:**
- Structured input/output (typed responses vs string parsing)
- Tool discovery
- Sandboxing potential

In practice: most agents treat MCP tools as second-class citizens, requiring explicit hints to use them. "Works today with bash" beats "might be elegant tomorrow."

## Optimization Strategies

### 1. Grep Before Read

Find the relevant lines first, then read only those:

```
Grep: "FlexiblePricing" → found in pricing.rs:142
Read: pricing.rs lines 140-160 (not the whole 800 lines)
```

### 2. Targeted Line Ranges

Use offset and limit parameters when reading files. Don't dump entire files into context.

### 3. Better Search Heuristics

One smart grep beats five exploratory reads:

```
# Expensive approach
Explore agent: reads 20 files → 70k tokens

# Cheap approach (when you know the codebase)
Grep pattern → 3 files
Read targeted sections → 5k tokens
```

### 4. Avoid Re-reading

Within a single agent run, re-reading the same file is pure waste. Track what's already in context.

### 5. Summarization Layers

Agents that return "here's what matters" rather than raw content.

## The Core Insight

**The smarter you make the agent, the more it can avoid reading.**

Planning which files matter is cheaper than reading everything to find out. Invest tokens in reasoning about what to read, not in reading everything.

---

## Current Setup Analysis

Audit of `~/.claude` and `~/Projects/hu` token waste patterns.

### Critical: Archive Accumulation

| Location | Size | Files |
|----------|------|-------|
| `~/.claude/debug/` | 423 MB | 1,273 |
| `~/.claude/file-history/` | 66 MB | 7,267 |
| `~/.claude/shell-snapshots/` | 34 MB | - |
| `~/.claude/paste-cache/` | 4.1 MB | - |
| `~/.claude/history.jsonl` | 1.3 MB | 1 |

Not directly loaded into context, but:
- Slows Glob/Grep traversal
- Creates session start latency
- Risk of accidental inclusion

**Fix:** Auto-cleanup older than 7 days, rotate history.jsonl to 1000 entries.

### High: Oversized Slash Commands

Jira commands are massive:
- `/jira:finalize.md` - 328 lines (~3,500 tokens)
- `/jira:check.md` - 182 lines
- `/jira:implement.md` - 188 lines

These load entirely into context on invocation. Only 20-30% typically needed.

**Fix:** Extract step-by-step sections into modular sub-commands.

### High: Hooks Injecting Context

Two hooks run constantly:
- `session-start.sh` (85 lines) - runs `hu data sync`, `hu data session list`, `hu data todos pending`
- `user-prompt-submit.sh` (74 lines) - runs `hu data search` on every prompt

**Impact:** 30,000-50,000 extra tokens/day (100 prompts × 300-500 tokens each).

**Fix:** Make opt-in, add sampling, cache 5-minute results.

### High: CLAUDE.md Bloat

| File | Lines | Tokens |
|------|-------|--------|
| `~/.claude/CLAUDE.md` | 277 | ~3,000 |
| `~/Projects/hu/CLAUDE.md` | 124 | ~1,300 |
| Referenced `doc/rules/` | 1,765 | ~19,000 |

The global CLAUDE.md embeds entire CLI reference instead of linking.

**Fix:** Reduce to 80 lines with index links.

### Medium: Large Documentation Files

| File | Lines |
|------|-------|
| `doc/cli/hooks.md` | 1,602 |
| `doc/cli/settings.md` | 1,505 |
| `doc/cli/plugins.md` | 1,192 |

Loaded by `/claude:reference`, injecting 8,000-15,000 tokens per lookup.

**Fix:** Create 200-line condensed versions, link to full docs.

### Medium: Plan/TODO Accumulation

- `~/.claude/plans/` - 46 plans (308 KB)
- `~/.claude/todos/` - 815 files (3.2 MB)

Not auto-cleaned. Old TODOs accidentally re-loaded.

**Fix:** Auto-archive after 90 days.

### Low: Settings Verbosity

`settings.json` has 63 explicit WebFetch domain permissions (~150 tokens).

**Fix:** Use wildcards: `WebFetch(domain:*.wikipedia.org)`

---

## Summary: Token Waste by Priority

| Pattern | Impact | Tokens/Session |
|---------|--------|----------------|
| Hooks context injection | High | 30-50K/day |
| CLAUDE.md bloat | High | 20K+ |
| Slash command definitions | Medium | 10-20K |
| Large doc references | Medium | 15K/lookup |
| Archive I/O overhead | Critical | Latency |

---

## Quick Wins

1. **Add cleanup to hooks** - Archive debug files >7 days
2. **Reduce CLAUDE.md** - Cut to 80 lines with index links
3. **Make hooks opt-in** - Add env var to disable context injection
4. **Condense doc references** - 1,600 lines → 200 lines

**Estimated savings: 40,000-60,000 tokens per typical session.**

---

## Proposed hu Features for Token Savings

### 1. Smarter HTML Fetching

Current `hu utils fetch-html` grabs everything. Add filtering modes:

```bash
hu utils fetch-html <url> --content  # Extract main content only
hu utils fetch-html <url> --summary  # Return 200-line summary
hu utils fetch-html <url> --links    # Just extract links
hu utils fetch-html <url> --headings # Just h1-h6 structure
```

**Implementation:**
- `--content`: Strip `<script>`, `<style>`, `<nav>`, `<footer>`, `<aside>`, `<header>`, ads. Keep only `<main>`, `<article>`, or largest content block
- `--summary`: Truncate to first N paragraphs + all headings
- `--links`: Return `[text](url)` list only (useful for crawling decisions)
- `--headings`: Return document outline (decide if worth reading)

**Token savings:** 10-50x reduction for documentation pages.

### 2. Smart Grep with Context Control

```bash
hu grep <pattern> --refs           # Return file:line only, no content
hu grep <pattern> --unique         # Dedupe similar matches
hu grep <pattern> --ranked         # Sort by relevance
hu grep <pattern> --limit 5        # Top N matches only
hu grep <pattern> --signature      # Show function signature only
```

**Implementation:**
- `--refs`: Output `src/foo.rs:142` not the line content (let agent decide what to read)
- `--unique`: Group identical/similar lines, show count
- `--signature`: Parse AST, return function/class signature not body

**Token savings:** 5-20x for broad searches.

### 3. File Intelligence

```bash
hu read <file> --outline           # Headings/function signatures only
hu read <file> --interface         # Public API only (skip implementation)
hu read <file> --diff <commit>     # Just what changed
hu read <file> --around <line> 10  # N lines around target
```

**Implementation:**
- `--outline`: For code, extract `fn`, `struct`, `impl`, `class`, `def` signatures. For markdown, extract headings
- `--interface`: Use tree-sitter to extract public items only
- `--around`: Centered context window (better than head/tail)

**Token savings:** 3-10x for large files.

### 4. Session-Aware Caching

```bash
hu context track <file>            # Mark file as "in context"
hu context check <file>            # Returns "already loaded" or size
hu context summary                 # What's currently tracked
hu context clear                   # Reset for new task
```

**Implementation:**
- Track files read in current session (via temp file or env)
- Before reading, check if already loaded
- Return "File already in context (loaded 2 mins ago)" instead of re-reading

**Token savings:** Prevents duplicate reads entirely.

### 5. Search Result Ranking

```bash
hu find <query> --smart            # Semantic ranking
hu find <query> --by-relevance     # Sort by match density
hu find <query> --files-only       # Just paths, no content
```

**Implementation:**
- Score files by: match count, match density, recency, file importance (src/ > test/)
- Return ranked list with scores
- Let agent pick top 3 instead of reading all 20

### 6. Documentation Index

```bash
hu docs index <dir>                # Build searchable index
hu docs search <query>             # Search index, return relevant sections
hu docs section <file> <heading>   # Extract specific section only
```

**Implementation:**
- Pre-index markdown files by heading
- Return just the relevant section, not whole file
- Cache index for fast repeated lookups

**Token savings:** 5-10x for doc lookups.

### 7. Code Navigation Shortcuts

```bash
hu code definition <symbol>        # Jump to definition (file:line)
hu code references <symbol>        # Where it's used (refs only)
hu code callers <function>         # What calls this
hu code structure <file>           # AST outline
```

**Implementation:**
- Wrap rust-analyzer/tree-sitter for fast lookups
- Return locations, not content
- Let agent decide what to actually read

---

## Implementation Priority

| Feature | Effort | Token Savings | Priority |
|---------|--------|---------------|----------|
| `fetch-html --content` | Low | 10-50x | 1 |
| `grep --refs` | Low | 5-20x | 1 |
| `read --outline` | Medium | 3-10x | 2 |
| `context track/check` | Medium | 100% of dupes | 2 |
| `docs section` | Medium | 5-10x | 3 |
| `code definition` | High | 3-5x | 4 |

The first two are quick wins - minimal implementation, massive savings.

---

## Agent Tools vs Hooks

Not all token-saving features should be agent-invoked. Some should run automatically.

### Best as Agent Tools

Agent decides when/how to use:

| Tool | Why Agent-Controlled |
|------|---------------------|
| `fetch-html --content/--links` | Agent chooses URL and extraction mode |
| `grep --refs/--signature` | Agent crafts pattern, decides detail level |
| `read --outline/--around` | Agent picks file and what view it needs |
| `docs section` | Agent decides which section to extract |
| `code definition/references` | Agent chooses symbol to investigate |

### Best as Hooks

Transparent, automatic - agent shouldn't think about them:

| Hook Trigger | Action | Why Automatic |
|--------------|--------|---------------|
| **Pre-Read** | Check if file already in context | Agent shouldn't track this manually |
| **Pre-Read** | Auto-truncate files >500 lines | Prevent accidental token bombs |
| **Pre-Grep** | Limit results to 20 matches | Prevent runaway searches |
| **Post-Fetch** | Strip scripts/styles automatically | Always wanted, never skip |
| **Session-Start** | Build/update code index | Ready before agent needs it |
| **Session-End** | Clear context tracking, cleanup | Maintenance |
| **Pre-Tool** | Warn if operation exceeds token budget | Guard rails |

### Hybrid: Hook + Tool Override

Some features work as hooks with agent override:

```bash
# Hook auto-strips HTML junk, but agent can bypass:
hu utils fetch-html <url> --raw

# Hook auto-limits grep to 20, but agent can bypass:
hu grep <pattern> --no-limit
```

---

## Session History Files (.jsonl)

Claude Code stores session data in `.jsonl` files under `~/.claude/projects/`.

### What's Stored

Each session file contains:
- Session ID, project path, timestamps
- All messages (user + assistant)
- Tool calls and results
- Token counts
- Model used

### Primary Uses

| Purpose | Description |
|---------|-------------|
| `--continue` / `--resume` | Resume previous sessions |
| `hu data sessions` | List/read past sessions |
| `hu data search` | Search through message history |
| `hu data stats` | Usage analytics (tokens, costs) |
| `hu data tools` | Tool usage statistics |
| Hook context injection | Hooks query these for "similar past work" |

### The Token Waste Problem

The files themselves aren't loaded into Claude's context, but:

1. **Hooks query them** → extract "relevant" history → inject into context
2. **More sessions = slower searches** = more latency
3. **815+ files** across todos and sessions = significant I/O

### Optimization

```bash
# Index into SQLite (faster queries than raw .jsonl)
hu data sync

# Prune old sessions
hu data sessions --prune --keep 100

# Or archive raw .jsonl files older than 30 days
```

If not using `--continue` or search/analytics, raw `.jsonl` files can be archived aggressively. The SQLite DB is more efficient for queries.
