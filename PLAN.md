# hu - Feature Plan

A unified CLI for dev workflows: Kubernetes pods, Jira tickets, GitHub PRs/Actions, and AWS pipelines.

## Current Features

- [x] AWS SSO login integration
- [x] EKS cluster connection (prod/dev/stg)
- [x] Pod discovery and filtering by type
- [x] Interactive shell access with custom prompts
- [x] Multi-pod log tailing with color-coded output

## Planned Features

### Jira Integration

- OAuth authentication flow
- Fetch ticket details by ID (`PROJ-123`)
- Display ticket summary, description, status, assignee
- Quick ticket search by project/sprint
- Open ticket in browser

### GitHub Integration

- List PRs for current branch or by number
- Show PR status, reviews, checks
- Monitor GitHub Actions workflow runs
- Display action logs and failure details
- Link PRs to Jira tickets (extract from branch name)

### AWS CodePipeline Integration

- List pipeline executions
- Show pipeline stage status (Source → Build → Deploy)
- Display execution history
- Quick link to AWS Console

### AWS Secrets Manager Integration

- List secrets by prefix/pattern
- Display secret value (with confirmation)
- Copy secret to clipboard
- Show secret metadata (last rotated, etc.)

## Architecture

```
src/
├── main.rs              # CLI entry, command routing
├── lib.rs               # Public API
├── cli/
│   ├── mod.rs           # Clap parser
│   └── commands/
│       ├── eks.rs       # Pod access (current functionality)
│       ├── jira.rs      # Ticket lookup
│       ├── gh.rs        # GitHub PRs and Actions
│       ├── pipeline.rs  # CodePipeline status
│       └── secret.rs    # Secrets Manager
├── auth/
│   ├── aws.rs           # SSO session management
│   ├── jira.rs          # OAuth flow
│   └── github.rs        # GitHub token
├── api/
│   ├── jira.rs          # Jira REST client
│   ├── github.rs        # GitHub API client
│   └── aws.rs           # AWS SDK wrappers
└── ui/
    ├── output.rs        # Consistent formatting
    ├── table.rs         # Table display
    └── progress.rs      # Spinners
```

## Command Examples

```bash
# EKS (existing)
hu eks                            # List pods
hu eks -p 1                       # Connect to pod
hu eks --log                      # Tail logs

# Jira
hu jira PROJ-123                  # Show ticket details
hu jira search "auth bug"         # Search tickets

# GitHub
hu gh pr                          # Show PR for current branch
hu gh pr 456                      # Show specific PR
hu gh actions                     # Show workflow runs
hu gh actions --watch             # Live monitor

# AWS Pipelines
hu pipeline                       # List recent executions
hu pipeline cms-deploy            # Show specific pipeline

# Secrets
hu secret list prod/              # List secrets by prefix
hu secret get prod/api-key        # Show secret value
```

## Dependencies to Add

- `gouqi` - Jira API client (see below)
- `octocrab` - GitHub API (see below)
- `aws-sdk-codepipeline` - Pipeline status
- `aws-sdk-secretsmanager` - Secrets access
- `keyring` - Secure token storage
- `open` - Open URLs in browser

## Gouqi - Jira API Client

**Version**: 0.20.0
**Repo**: https://github.com/bazaah/gouqi (fork of goji)

### Features

- **Sync & Async API**: Default sync, async via feature flag
- **Multiple auth**: Basic auth, bearer tokens, cookie sessions, OAuth 1.0a (Jira Server)
- **Full coverage**: Issues, projects, boards, sprints, users, attachments
- **Caching & observability**: Built-in infrastructure

### Installation

```bash
cargo add gouqi
# For async support:
cargo add gouqi --features async
```

### Usage Examples

```rust
use gouqi::{Credentials, Jira};

// Create client with basic auth
let jira = Jira::new(
    "https://company.atlassian.net",
    Credentials::Basic("user@example.com", "api-token")
)?;

// Search issues with JQL
let issues = jira.search()
    .iter("project = PROJ AND assignee = currentUser()", &Default::default())?;

for issue in issues {
    println!("{}: {}", issue.key, issue.fields.summary);
}

// Get single issue
let issue = jira.issues().get("PROJ-123")?;
```

### Async Usage

```rust
use gouqi::{Credentials, Jira};

let jira = Jira::new(host, Credentials::Basic(user, token))?;
let issue = jira.issues().get_async("PROJ-123").await?;
```

### hu Integration Plan

For `hu jira` commands:
- Store Jira credentials in config (host, email, API token)
- Use gouqi for issue fetching and search
- Implement `hu jira PROJ-123`, `hu jira search "query"`

## Octocrab - GitHub API Client

**Version**: 0.49.5 (Dec 2025)
**Repo**: https://github.com/XAMPPRocky/octocrab

### Features

- **Semantic API**: Strongly-typed access to GitHub endpoints (repos, issues, PRs, commits, Actions, orgs, teams, users)
- **GraphQL support**: For complex queries
- **Webhook support**: Deserializable types for GitHub webhook events
- **HTTP API**: Lower-level access for custom extensions
- **Static API**: Reference-counted singleton pattern

### Installation

```bash
cargo add octocrab
```

### Usage Examples

```rust
// Get a pull request
let pr = octocrab::instance()
    .pulls("owner", "repo")
    .get(123)
    .await?;

// List workflow runs
let runs = octocrab::instance()
    .workflows("owner", "repo")
    .list_runs("ci.yml")
    .send()
    .await?;

// List PRs
let prs = octocrab::instance()
    .pulls("owner", "repo")
    .list()
    .state(octocrab::params::State::Open)
    .send()
    .await?;
```

### Authentication

```rust
// Personal access token
let octocrab = octocrab::Octocrab::builder()
    .personal_token(token)
    .build()?;

// GitHub App
let octocrab = octocrab::Octocrab::builder()
    .app(app_id, key)
    .build()?;
```

### hu Integration Plan

For `hu gh` commands:
- Store GitHub token in config or use `gh` CLI's auth
- Use octocrab for PR listing, status checks, workflow runs
- Implement `hu gh pr`, `hu gh runs`, `hu gh clear-runs`
