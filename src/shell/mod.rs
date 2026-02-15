mod cli;
mod ls;

pub use cli::ShellCommand;

use anyhow::Result;

pub fn run_command(cmd: ShellCommand) -> Result<()> {
    match cmd {
        ShellCommand::Ls(args) => ls::run(args),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_command_exported() {
        let _ = std::any::type_name::<ShellCommand>();
    }
}
