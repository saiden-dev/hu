mod auth;
mod cli;
mod client;
mod failures;
mod login;
mod prs;
mod types;

pub use cli::GhCommand;

pub async fn run_command(cmd: GhCommand) -> anyhow::Result<()> {
    match cmd {
        GhCommand::Login(args) => login::run(args).await,
        GhCommand::Prs => prs::run().await,
        GhCommand::Runs => {
            println!("gh runs: not yet implemented");
            Ok(())
        }
        GhCommand::Failures(args) => failures::run(args).await,
        GhCommand::Ci => {
            println!("gh ci: not yet implemented");
            Ok(())
        }
    }
}
