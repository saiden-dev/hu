//! Jira integration
//!
//! # CLI Usage
//! Use [`run_command`] for CLI commands that format and print output.
//!
//! # Programmatic Usage (MCP/HTTP)
//! Use the reusable functions that return typed data:
//! - [`get_issue`] - Get a single issue
//! - [`search_issues`] - Search with JQL
//! - [`get_current_user`] - Get authenticated user
//! - [`update_issue`] - Update issue fields
//! - [`get_transitions`] - Get available transitions
//! - [`transition_issue`] - Change issue status

mod adf;
mod auth;
mod auth_handler;
mod cli;
mod client;
mod comments;
mod create;
mod search;
mod service;
mod show;
mod sprint;
mod sprints;
mod tickets;
mod types;
mod update;

use anyhow::Result;

pub use cli::JiraCommand;
pub use types::{Issue, IssueUpdate, Transition, User};

use comments::CommentsArgs;
use create::CreateArgs;
use update::UpdateArgs;

/// Run a Jira command (CLI entry point - formats and prints)
#[cfg(not(tarpaulin_include))]
pub async fn run_command(cmd: JiraCommand) -> anyhow::Result<()> {
    match cmd {
        JiraCommand::Auth => auth_handler::run().await,
        JiraCommand::Tickets => tickets::run().await,
        JiraCommand::Sprint => sprint::run(sprint::SprintArgs::default()).await,
        JiraCommand::Sprints { state } => sprints::run(&state).await,
        JiraCommand::Search { query } => search::run(&query).await,
        JiraCommand::Show { key } => show::run(&key).await,
        JiraCommand::Comments { key, full, json } => {
            comments::run(CommentsArgs { key, full, json }).await
        }
        JiraCommand::Create {
            summary,
            r#type,
            project,
            body,
            body_adf,
            assign,
            json,
        } => {
            create::run(CreateArgs {
                project_key: project,
                summary,
                issue_type: r#type,
                body,
                body_adf,
                assign,
                json,
            })
            .await
        }
        JiraCommand::Update {
            key,
            summary,
            status,
            assign,
            body,
            body_adf,
        } => {
            update::run(UpdateArgs {
                key,
                summary,
                status,
                assign,
                body,
                body_adf,
            })
            .await
        }
    }
}

// ============================================================================
// Reusable functions for MCP/HTTP - return typed data, never print
// ============================================================================

/// Get a single issue by key (for MCP/HTTP)
#[allow(dead_code)]
pub async fn get_issue(key: &str) -> Result<Issue> {
    let client = service::create_client().await?;
    service::get_issue(&client, key).await
}

/// Search issues using JQL (for MCP/HTTP)
#[allow(dead_code)]
pub async fn search_issues(jql: &str) -> Result<Vec<Issue>> {
    let client = service::create_client().await?;
    service::search_issues(&client, jql).await
}

/// Get current authenticated user (for MCP/HTTP)
#[allow(dead_code)]
pub async fn get_current_user() -> Result<User> {
    let client = service::create_client().await?;
    service::get_current_user(&client).await
}

/// Update issue fields (for MCP/HTTP)
#[allow(dead_code)]
pub async fn update_issue(key: &str, update: &IssueUpdate) -> Result<()> {
    let client = service::create_client().await?;
    service::update_issue(&client, key, update).await
}

/// Get available transitions for an issue (for MCP/HTTP)
#[allow(dead_code)]
pub async fn get_transitions(key: &str) -> Result<Vec<Transition>> {
    let client = service::create_client().await?;
    service::get_transitions(&client, key).await
}

/// Transition an issue to a new status (for MCP/HTTP)
#[allow(dead_code)]
pub async fn transition_issue(key: &str, transition_id: &str) -> Result<()> {
    let client = service::create_client().await?;
    service::transition_issue(&client, key, transition_id).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jira_command_exported() {
        // Verify JiraCommand is re-exported
        let _cmd = JiraCommand::Auth;
    }

    #[test]
    fn update_args_created() {
        let args = UpdateArgs {
            key: "X-1".to_string(),
            summary: None,
            status: None,
            assign: None,
            body: None,
            body_adf: None,
        };
        assert_eq!(args.key, "X-1");
    }
}
