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
