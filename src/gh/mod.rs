mod auth;
mod cli;
mod client;
mod login;
mod prs;
mod types;

pub use cli::GhCommand;

pub async fn run_command(cmd: GhCommand) -> anyhow::Result<()> {
    match cmd {
        GhCommand::Login => login::run().await,
        GhCommand::Prs => prs::run().await,
        GhCommand::Runs => {
            println!("gh runs: not yet implemented");
            Ok(())
        }
        GhCommand::Failures => {
            println!("gh failures: not yet implemented");
            Ok(())
        }
        GhCommand::Ci => {
            println!("gh ci: not yet implemented");
            Ok(())
        }
    }
}
