mod auth;
mod auth_handler;
mod cli;
mod client;
mod search;
mod show;
mod sprint;
mod tickets;
mod types;
mod update;

pub use cli::JiraCommand;

use update::UpdateArgs;

pub async fn run_command(cmd: JiraCommand) -> anyhow::Result<()> {
    match cmd {
        JiraCommand::Auth => auth_handler::run().await,
        JiraCommand::Tickets { board } => tickets::run(board).await,
        JiraCommand::Sprint { board } => sprint::run(sprint::SprintArgs { board }).await,
        JiraCommand::Search { query } => search::run(&query).await,
        JiraCommand::Show { key } => show::run(&key).await,
        JiraCommand::Update {
            key,
            summary,
            status,
            assign,
        } => {
            update::run(UpdateArgs {
                key,
                summary,
                status,
                assign,
            })
            .await
        }
    }
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
        };
        assert_eq!(args.key, "X-1");
    }
}
