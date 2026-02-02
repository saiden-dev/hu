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

/// Print message for unimplemented Runs command (extracted for testability)
pub fn runs_not_implemented_msg() -> &'static str {
    "gh runs: not yet implemented"
}

/// Print message for unimplemented Ci command (extracted for testability)
pub fn ci_not_implemented_msg() -> &'static str {
    "gh ci: not yet implemented"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_not_implemented_returns_message() {
        let msg = runs_not_implemented_msg();
        assert_eq!(msg, "gh runs: not yet implemented");
    }

    #[test]
    fn ci_not_implemented_returns_message() {
        let msg = ci_not_implemented_msg();
        assert_eq!(msg, "gh ci: not yet implemented");
    }

    #[test]
    fn gh_command_exported() {
        // Verify GhCommand is accessible
        let _ = std::any::type_name::<GhCommand>();
    }
}
