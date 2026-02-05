mod auth;
mod cli;
mod client;
mod failures;
mod fix;
mod helpers;
mod login;
mod prs;
mod runs;
mod types;

pub use cli::GhCommand;

pub async fn run_command(cmd: GhCommand) -> anyhow::Result<()> {
    match cmd {
        GhCommand::Login(args) => login::run(args).await,
        GhCommand::Prs => prs::run().await,
        GhCommand::Failures(args) => failures::run(args).await,
        GhCommand::Fix(args) => fix::run(args).await,
        GhCommand::Runs(args) => runs::run(args).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gh_command_exported() {
        // Verify GhCommand is accessible
        let _ = std::any::type_name::<GhCommand>();
    }
}
