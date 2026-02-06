# Token Savings Analysis

Comparison of token usage between hu CLI, MCP servers, and Claude Code built-in tools.

| Operation | MCP Server | Built-in Tool | hu CLI | Savings |
|-----------|------------|---------------|--------|---------|
| **Setup Overhead** | | | | |
| Tool schema loading (10 tools) | 2,000-3,000 | 0 | 0 | 100% |
| Tool schema loading (20 tools) | 4,000-6,000 | 0 | 0 | 100% |
| Per-session protocol overhead | 500-1,000 | 0 | 0 | 100% |
| **File Reading** | | | | |
| Read 500-line file (full) | 8,000 | 8,000 | 8,000 | 0% |
| Read 500-line file (outline) | 8,000 | 8,000 | 800 | 90% |
| Read 500-line file (interface) | 8,000 | 8,000 | 400 | 95% |
| Read specific function (around) | 8,000 | 8,000 | 200 | 97% |
| Read git diff only | 8,000 | 8,000 | 300 | 96% |
| Re-read same file | 8,000 | 8,000 | 20 | 99% |
| Check if file in context | N/A | N/A | 20 | 100% |
| **Code Search** | | | | |
| Grep codebase (full content) | 50,000 | 50,000 | 50,000 | 0% |
| Grep codebase (refs only) | 50,000 | 50,000 | 2,000 | 96% |
| Grep codebase (signatures) | 50,000 | 50,000 | 500 | 99% |
| Grep with limit 10 | 50,000 | 50,000 | 1,000 | 98% |
| Grep ranked results | 50,000 | 50,000 | 1,500 | 97% |
| Find function definition | 20,000 | 20,000 | 500 | 97% |
| **Web Operations** | | | | |
| Fetch full webpage | 100,000 | 100,000 | 100,000 | 0% |
| Fetch main content only | 100,000 | 100,000 | 5,000 | 95% |
| Fetch headings only | 100,000 | 100,000 | 500 | 99% |
| Fetch links only | 100,000 | 100,000 | 1,000 | 99% |
| Fetch with CSS selector | 100,000 | 100,000 | 2,000 | 98% |
| Web search (list results) | 30,000 | 30,000 | 1,500 | 95% |
| **Documentation** | | | | |
| Read full doc file | 15,000 | 15,000 | 15,000 | 0% |
| Extract specific section | 15,000 | 15,000 | 500 | 97% |
| Search docs index | 15,000 | 15,000 | 300 | 98% |
| **API Integrations** | | | | |
| Jira ticket details | 15,000 | N/A | 800 | 95% |
| Jira sprint list | 20,000 | N/A | 1,000 | 95% |
| GitHub PR list | 20,000 | N/A | 600 | 97% |
| GitHub CI failures | 25,000 | N/A | 1,500 | 94% |
| Slack message search | 25,000 | N/A | 1,000 | 96% |
| Slack channel history | 15,000 | N/A | 800 | 95% |
| PagerDuty incidents | 10,000 | N/A | 500 | 95% |
| PagerDuty oncall | 5,000 | N/A | 300 | 94% |
| Sentry issues list | 15,000 | N/A | 700 | 95% |
| Sentry issue details | 10,000 | N/A | 500 | 95% |
| NewRelic incidents | 12,000 | N/A | 600 | 95% |
| NewRelic NRQL query | 8,000 | N/A | 400 | 95% |
| AWS Pipeline status | 10,000 | N/A | 500 | 95% |
| EKS pod list | 8,000 | N/A | 400 | 95% |
| **Session Analytics** | | | | |
| Claude session stats | N/A | N/A | 300 | 100% |
| Claude session search | N/A | N/A | 500 | 100% |
| Claude tool usage | N/A | N/A | 400 | 100% |
| Claude pricing analysis | N/A | N/A | 600 | 100% |
| **Hooks (Automatic)** | | | | |
| Prevent duplicate file read | N/A | N/A | 20 | 100% |
| Large file warning | N/A | N/A | 50 | N/A |
| Broad grep warning | N/A | N/A | 50 | N/A |
| Context tracking | N/A | N/A | 50 | 100% |

**Typical 2-hour session comparison:**

| Metric | Without hu | With hu | Savings |
|--------|------------|---------|---------|
| File reads (20 files) | 160,000 | 20,000 | 87% |
| Grep searches (30) | 300,000 | 30,000 | 90% |
| Web fetches (5) | 500,000 | 25,000 | 95% |
| Duplicate reads (10) | 80,000 | 200 | 99% |
| MCP overhead | 50,000 | 0 | 100% |
| **Total** | **1,090,000** | **75,200** | **93%** |
