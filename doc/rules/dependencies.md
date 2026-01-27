# Dependencies

## Prefer Established Crates

**Don't reinvent the wheel.** Use mature, maintained, popular crates over custom implementations.

| Need | Use crate | Don't implement |
|------|-----------|-----------------|
| HTTP client | `reqwest` | Custom HTTP handling |
| JSON parsing | `serde_json` | Manual parsing |
| CLI parsing | `clap` | Custom arg parsing |
| Date/time | `chrono` | Manual date math |
| Regex | `regex` | Custom pattern matching |
| UUID | `uuid` | Custom ID generation |
| Base64 | `base64` | Manual encoding |
| URL parsing | `url` | String manipulation |
| Retries | `backoff`, `tokio-retry` | Custom retry loops |
| Rate limiting | `governor` | Manual throttling |

**Selection criteria:**
- Downloads: >100k/month on crates.io
- Maintenance: Updated within last 6 months
- Ecosystem: Used by other popular crates
- Documentation: Has examples and API docs

**When to implement custom:**
- Trivial one-liner (don't add dep for 3 lines of code)
- Domain-specific logic unique to your app
- When existing crates don't fit and wrapping is harder than implementing

## Ask Before Adding Dependencies

**Always ask the user** before adding new crates. Present choices:

**When feature is simple:**
> "This feature is simple. We only need one method from `chrono`.
> Do you want me to:
> 1. Add `chrono` (recommended - handles edge cases)
> 2. Implement manually (~10 lines)"

**When multiple solutions exist:**
> "There are multiple crates for HTTP retries:
> 1. `backoff` - simple, sync-focused
> 2. `tokio-retry` - async-native, minimal
> 3. `reqwest-retry` - reqwest middleware
>
> Which do you prefer?"

**When crate seems heavy:**
> "For UUID generation, options are:
> 1. `uuid` crate (full UUID support, 50kb)
> 2. Manual with `rand` (v4 only, already have rand)
>
> We only need v4 UUIDs. Preference?"

**Don't silently add dependencies** — the user should know what's being added and why.

## Core Crates

- **clap** (derive) - CLI parsing
- **ratatui** - Terminal UI (tables, progress, colors, layouts)
- **crossterm** - Terminal backend for ratatui
- **tui-markdown** - Markdown rendering for ratatui
- **anyhow** - Application errors
- **thiserror** - Library errors
- **serde** + **serde_json** - Serialization
- **tokio** - Async runtime
- **tracing** - Structured logging
- **directories** - XDG config/data/cache paths (cross-platform)

### Config Paths with `directories`

```rust
use directories::ProjectDirs;

pub fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("", "", "hu")
}

// Usage:
let dirs = project_dirs().expect("no home directory");
let config_path = dirs.config_dir().join("config.toml");
// Linux: ~/.config/hu/config.toml
// macOS: ~/Library/Application Support/hu/config.toml
```

**Standard paths for `hu`:**
| Purpose | Method | Linux | macOS |
|---------|--------|-------|-------|
| Config | `config_dir()` | `~/.config/hu/` | `~/Library/Application Support/hu/` |
| Data | `data_dir()` | `~/.local/share/hu/` | `~/Library/Application Support/hu/` |
| Cache | `cache_dir()` | `~/.cache/hu/` | `~/Library/Caches/hu/` |

## Humanization Crates (ActiveSupport-like)

- **timeago** - "2 hours ago", "in 3 days"
- **humantime** - Duration <-> "1h 30m 32s" (parse & format)
- **Inflector** - pluralize, singularize, case conversion
- **humansize** - Bytes -> "1.43 MiB"

Put wrappers in `util/fmt.rs`:

```rust
// util/fmt.rs - Humanization helpers
use std::time::Duration;
use timeago::Formatter;
use humantime::format_duration;
use inflector::Inflector;
use humansize::{format_size, BINARY};

/// "2 hours ago", "in 3 days"
pub fn time_ago(duration: Duration) -> String {
    Formatter::new().convert(duration)
}

/// "1h 30m 32s"
pub fn duration(d: Duration) -> String {
    format_duration(d).to_string()
}

/// 1500000 -> "1.43 MiB"
pub fn bytes(size: u64) -> String {
    format_size(size, BINARY)
}

/// "user" -> "users"
pub fn pluralize(s: &str) -> String {
    s.to_plural()
}

/// "posts" -> "post"
pub fn singularize(s: &str) -> String {
    s.to_singular()
}

/// 1 -> "1st", 2 -> "2nd"
pub fn ordinalize(n: i64) -> String {
    n.ordinalize()
}
```

Usage in handlers:
```rust
use crate::util::fmt;

println!("Updated {}", fmt::time_ago(last_modified));
println!("Size: {}", fmt::bytes(file_size));
println!("Found {} {}", count, fmt::pluralize("ticket"));
```

## API Client Crates

Use established client libraries instead of raw HTTP:

| Service | Crate | Notes |
|---------|-------|-------|
| **GitHub** | `octocrab` | Typed API, extensible, 1.2k+ stars |
| **Jira** | `gouqi` | Async, V3 API, ADF support |
| **Slack** | `slack-rust` | SocketMode, Event API, Web API |
| **AWS** | `aws-sdk-*` | Official SDK, one crate per service |

```toml
# API Clients
octocrab = "0.44"
gouqi = "0.10"
slack-rust = "0.1"

# AWS (add only what you need)
aws-config = "1"
aws-sdk-eks = "1"
aws-sdk-codepipeline = "1"
aws-sdk-ec2 = "1"
aws-sdk-s3 = "1"
```

**AWS SDK pattern:**
```rust
use aws_config::BehaviorVersion;
use aws_sdk_eks::Client as EksClient;

let config = aws_config::defaults(BehaviorVersion::latest())
    .profile_name("my-profile")
    .load()
    .await;

let eks = EksClient::new(&config);
let clusters = eks.list_clusters().send().await?;
```

## Observability Services (Custom Wrappers)

No mature Rust crates exist for **reading from** these APIs. Build thin wrappers:

| Service | API Type | Approach |
|---------|----------|----------|
| **NewRelic** | GraphQL (NerdGraph) | `graphql_client` + `reqwest` |
| **Sentry** | REST | `reqwest` only |
| **PagerDuty** | REST | `reqwest` only |

```toml
# For NewRelic (GraphQL)
graphql_client = "0.14"
reqwest = { version = "0.12", features = ["json"] }

# For Sentry/PagerDuty (REST) - just reqwest
```

**NewRelic (NerdGraph + NRQL):**
```rust
// newrelic/client.rs
use reqwest::Client;

const NERDGRAPH_URL: &str = "https://api.newrelic.com/graphql";

pub struct NewRelicClient {
    client: Client,
    api_key: String,
    account_id: String,
}

impl NewRelicClient {
    pub async fn query_nrql(&self, nrql: &str) -> Result<serde_json::Value> {
        let query = format!(r#"{{
            actor {{
                account(id: {}) {{
                    nrql(query: "{}") {{ results }}
                }}
            }}
        }}"#, self.account_id, nrql);

        self.client
            .post(NERDGRAPH_URL)
            .header("API-Key", &self.api_key)
            .json(&serde_json::json!({ "query": query }))
            .send()
            .await?
            .json()
            .await
    }
}
```

**Sentry (REST):**
```rust
// sentry/client.rs
use reqwest::Client;

const SENTRY_API: &str = "https://sentry.io/api/0";

pub struct SentryClient {
    client: Client,
    auth_token: String,
    org: String,
}

impl SentryClient {
    pub async fn list_issues(&self, project: &str) -> Result<Vec<Issue>> {
        self.client
            .get(format!("{}/projects/{}/{}/issues/", SENTRY_API, self.org, project))
            .bearer_auth(&self.auth_token)
            .send()
            .await?
            .json()
            .await
    }

    pub async fn get_issue(&self, issue_id: &str) -> Result<Issue> {
        self.client
            .get(format!("{}/issues/{}/", SENTRY_API, issue_id))
            .bearer_auth(&self.auth_token)
            .send()
            .await?
            .json()
            .await
    }
}
```

**PagerDuty (REST):**
```rust
// pagerduty/client.rs
use reqwest::Client;

const PAGERDUTY_API: &str = "https://api.pagerduty.com";

pub struct PagerDutyClient {
    client: Client,
    api_key: String,
}

impl PagerDutyClient {
    /// Get current on-call users for schedules
    pub async fn get_oncalls(&self, schedule_ids: Option<&[&str]>) -> Result<Vec<OnCall>> {
        let mut req = self.client
            .get(format!("{}/oncalls", PAGERDUTY_API))
            .header("Authorization", format!("Token token={}", self.api_key));

        if let Some(ids) = schedule_ids {
            req = req.query(&[("schedule_ids[]", ids)]);
        }

        req.send().await?.json().await
    }

    /// List incidents by status (triggered, acknowledged, resolved)
    pub async fn list_incidents(&self, statuses: &[&str]) -> Result<Vec<Incident>> {
        self.client
            .get(format!("{}/incidents", PAGERDUTY_API))
            .header("Authorization", format!("Token token={}", self.api_key))
            .query(&[("statuses[]", statuses)])
            .send()
            .await?
            .json()
            .await
    }

    /// Check if current user is on-call
    pub async fn am_i_oncall(&self, user_id: &str) -> Result<bool> {
        let oncalls: Vec<OnCall> = self.client
            .get(format!("{}/oncalls", PAGERDUTY_API))
            .header("Authorization", format!("Token token={}", self.api_key))
            .query(&[("user_ids[]", user_id)])
            .send()
            .await?
            .json()
            .await?;

        Ok(!oncalls.is_empty())
    }
}
```

## CLI Authentication Strategies

### OAuth Device Flow (Best UX)
User sees code, opens browser, enters code. No localhost server needed.

```rust
// Supported by: GitHub
// Flow:
// 1. POST to device/code endpoint → get user_code + device_code
// 2. Display: "Enter code ABCD-1234 at https://github.com/login/device"
// 3. Poll token endpoint until user completes auth
// 4. Store token in config_dir()/credentials.toml
```

**GitHub Device Flow:**
```rust
const DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
// Requires OAuth App client_id (no client_secret needed for device flow)
```

### OAuth with Localhost Redirect
Spin up temporary local server to catch redirect.

```rust
// Supported by: Jira, Slack, Sentry, PagerDuty
// Flow:
// 1. Start localhost:PORT server
// 2. Open browser to auth URL with redirect_uri=http://localhost:PORT/callback
// 3. User authorizes, browser redirects to localhost with code
// 4. Exchange code for token
// 5. Shutdown server
```

**Jira OAuth 2.0 3LO:**
```rust
const AUTH_URL: &str = "https://auth.atlassian.com/authorize";
const TOKEN_URL: &str = "https://auth.atlassian.com/oauth/token";
// Requires: client_id, client_secret, redirect_uri
// Scopes: read:jira-work, read:jira-user, etc.
```

### API Keys (Simplest)
User creates key in web UI, pastes into CLI.

```rust
// Supported by: NewRelic, Sentry, PagerDuty
// Flow:
// 1. Prompt: "Enter API key (create at https://...):"
// 2. Store in config_dir()/credentials.toml
```

### AWS SSO
Uses IAM Identity Center, not OAuth.

```rust
// Flow: Shell out to AWS CLI
// $ aws sso login --profile my-profile
// Or use aws-config crate with SsoCredentialsProvider
```

### Credential Storage

```rust
use directories::ProjectDirs;

// Store in: ~/.config/hu/credentials.toml
// Format:
// [github]
// token = "ghp_xxx"
//
// [jira]
// access_token = "xxx"
// refresh_token = "xxx"
// expires_at = 1234567890
//
// [newrelic]
// api_key = "NRAK-xxx"

// NEVER store in plain text in repo or logs
// Consider: keyring crate for OS keychain integration
```

| Service | Auth Method | Crate |
|---------|-------------|-------|
| GitHub | Device Flow | `octocrab` + custom |
| Jira | OAuth 2.0 3LO | `oauth2` |
| Slack | OAuth 2.0 | `oauth2` |
| PagerDuty | API Key | - |
| Sentry | API Token | - |
| NewRelic | API Key | - |
| AWS | SSO | `aws-config` |

## Dev Dependencies

- **insta** - Snapshot testing
- **criterion** - Benchmarks
- **tempfile** - Test fixtures
