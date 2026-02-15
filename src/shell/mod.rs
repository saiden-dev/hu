mod cli;
mod df;
mod ls;

pub use cli::ShellCommand;

use anyhow::Result;

pub fn run_command(cmd: ShellCommand) -> Result<()> {
    match cmd {
        ShellCommand::Ls(args) => ls::run(args),
        ShellCommand::Df(args) => df::run(args),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::cli::DfArgs;

    #[test]
    fn shell_command_exported() {
        let _ = std::any::type_name::<ShellCommand>();
    }

    #[test]
    fn run_df_command() {
        let cmd = ShellCommand::Df(DfArgs { json: false });
        assert!(run_command(cmd).is_ok());
    }

    #[test]
    fn run_df_json_command() {
        let cmd = ShellCommand::Df(DfArgs { json: true });
        assert!(run_command(cmd).is_ok());
    }
}
