# Slash Commands

All hu commands are available as Claude Code slash commands after running `hu install run`.

## Jira

| Command | Description |
|---------|-------------|
| `/hu:jira:auth` | Authenticate with Jira via OAuth 2.0 |
| `/hu:jira:tickets` | List my tickets in current sprint |
| `/hu:jira:sprint` | Show all issues in current sprint |
| `/hu:jira:search` | Search tickets using JQL |
| `/hu:jira:show` | Show ticket details |
| `/hu:jira:update` | Update a Jira ticket |

## GitHub

| Command | Description |
|---------|-------------|
| `/hu:gh:login` | Authenticate with GitHub using a PAT |
| `/hu:gh:prs` | List open pull requests authored by you |
| `/hu:gh:runs` | List GitHub workflow runs |
| `/hu:gh:failures` | Extract test failures from CI |
| `/hu:gh:fix` | Analyze CI failures and output investigation context |

## Slack

| Command | Description |
|---------|-------------|
| `/hu:slack:auth` | Authenticate with Slack |
| `/hu:slack:config` | Show Slack configuration status |
| `/hu:slack:channels` | List channels in the workspace |
| `/hu:slack:info` | Show channel details |
| `/hu:slack:send` | Send a message to a channel |
| `/hu:slack:history` | Show message history for a channel |
| `/hu:slack:search` | Search Slack messages |
| `/hu:slack:users` | List users in the workspace |
| `/hu:slack:whoami` | Show current user info |
| `/hu:slack:tidy` | Mark channels as read if no direct mentions |

## PagerDuty

| Command | Description |
|---------|-------------|
| `/hu:pagerduty:auth` | Set PagerDuty API token |
| `/hu:pagerduty:config` | Show PagerDuty configuration status |
| `/hu:pagerduty:oncall` | Show who's currently on call |
| `/hu:pagerduty:alerts` | List active alerts |
| `/hu:pagerduty:incidents` | List incidents with filters |
| `/hu:pagerduty:show` | Show incident details |
| `/hu:pagerduty:whoami` | Show current PagerDuty user info |

## Sentry

| Command | Description |
|---------|-------------|
| `/hu:sentry:auth` | Set Sentry auth token |
| `/hu:sentry:config` | Show Sentry configuration status |
| `/hu:sentry:issues` | List Sentry issues |
| `/hu:sentry:show` | Show Sentry issue details |
| `/hu:sentry:events` | List events for a Sentry issue |

## NewRelic

| Command | Description |
|---------|-------------|
| `/hu:newrelic:auth` | Set NewRelic API key and account ID |
| `/hu:newrelic:config` | Show NewRelic configuration status |
| `/hu:newrelic:issues` | List recent NewRelic issues |
| `/hu:newrelic:incidents` | List recent NewRelic incidents |
| `/hu:newrelic:query` | Run NRQL query |

## AWS Pipeline

| Command | Description |
|---------|-------------|
| `/hu:pipeline:list` | List all CodePipeline pipelines |
| `/hu:pipeline:status` | Show pipeline status |
| `/hu:pipeline:history` | Show pipeline execution history |

## EKS

| Command | Description |
|---------|-------------|
| `/hu:eks:list` | List pods in the EKS cluster |
| `/hu:eks:exec` | Execute a command in a pod |
| `/hu:eks:logs` | Tail logs from a pod |

## Data (Claude Code Sessions)

| Command | Description |
|---------|-------------|
| `/hu:data:sync` | Sync Claude Code data to local database |
| `/hu:data:config` | Show data configuration |
| `/hu:data:session` | Session operations (list, read, current) |
| `/hu:data:stats` | Usage statistics |
| `/hu:data:todos` | Todo operations (list, pending) |
| `/hu:data:search` | Search messages |
| `/hu:data:tools` | Tool usage statistics |
| `/hu:data:errors` | Extract errors from debug logs |
| `/hu:data:pricing` | Pricing analysis |
| `/hu:data:branches` | Branch activity statistics |

## Utils

| Command | Description |
|---------|-------------|
| `/hu:utils:fetch-html` | Fetch URL and convert to markdown |
| `/hu:utils:grep` | Smart grep with token-saving options |
| `/hu:utils:web-search` | Web search using Brave Search API |
| `/hu:utils:docs-index` | Build heading index for markdown files |
| `/hu:utils:docs-search` | Search docs index |
| `/hu:utils:docs-section` | Extract a section from a markdown file |

## Context

| Command | Description |
|---------|-------------|
| `/hu:context:track` | Track file(s) as loaded in context |
| `/hu:context:check` | Check if file(s) are already in context |
| `/hu:context:summary` | Show summary of all tracked files |
| `/hu:context:clear` | Clear all tracked files |

## Read

| Command | Description |
|---------|-------------|
| `/hu:read` | Smart file reading with outline, interface, around, and diff modes |

## Install

| Command | Description |
|---------|-------------|
| `/hu:install:list` | List available components |
| `/hu:install:preview` | Show what would be installed |
| `/hu:install:run` | Install hooks and commands to Claude Code |
