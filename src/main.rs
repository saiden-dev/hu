use clap::{CommandFactory, Parser};

mod cli;
mod dashboard;
mod eks;
mod gh;
mod jira;
mod newrelic;
mod pagerduty;
mod sentry;
mod slack;
mod util;
mod utils;

use cli::{Cli, Command};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(cmd) => run_command(cmd).await,
        None => {
            Cli::command().print_help()?;
            println!();
            Ok(())
        }
    }
}

async fn run_command(cmd: Command) -> anyhow::Result<()> {
    match cmd {
        Command::Dashboard { cmd } => {
            println!("dashboard: {:?}", cmd);
        }
        Command::Jira { cmd: Some(cmd) } => {
            return jira::run_command(cmd).await;
        }
        Command::Jira { cmd: None } => {
            print_subcommand_help("jira")?;
        }
        Command::Gh { cmd: Some(cmd) } => {
            return gh::run_command(cmd).await;
        }
        Command::Gh { cmd: None } => {
            print_subcommand_help("gh")?;
        }
        Command::Slack { cmd: Some(cmd) } => {
            println!("slack: {:?}", cmd);
        }
        Command::Slack { cmd: None } => {
            print_subcommand_help("slack")?;
        }
        Command::PagerDuty { cmd: Some(cmd) } => {
            println!("pagerduty: {:?}", cmd);
        }
        Command::PagerDuty { cmd: None } => {
            print_subcommand_help("pagerduty")?;
        }
        Command::Sentry { cmd: Some(cmd) } => {
            println!("sentry: {:?}", cmd);
        }
        Command::Sentry { cmd: None } => {
            print_subcommand_help("sentry")?;
        }
        Command::NewRelic { cmd: Some(cmd) } => {
            println!("newrelic: {:?}", cmd);
        }
        Command::NewRelic { cmd: None } => {
            print_subcommand_help("newrelic")?;
        }
        Command::Eks { cmd: Some(cmd) } => {
            println!("eks: {:?}", cmd);
        }
        Command::Eks { cmd: None } => {
            print_subcommand_help("eks")?;
        }
        Command::Utils { cmd: Some(cmd) } => {
            return utils::run_command(cmd).await;
        }
        Command::Utils { cmd: None } => {
            print_subcommand_help("utils")?;
        }
    }
    Ok(())
}

fn print_subcommand_help(name: &str) -> anyhow::Result<()> {
    let mut cmd = Cli::command();
    for sub in cmd.get_subcommands_mut() {
        if sub.get_name() == name {
            sub.print_help()?;
            println!();
            return Ok(());
        }
    }
    unreachable!("unknown subcommand: {}", name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_no_args() {
        let cli = Cli::try_parse_from::<[&str; 0], &str>([]).unwrap();
        assert!(cli.command.is_none());
    }

    #[test]
    fn parses_subcommand_without_action() {
        let cli = Cli::try_parse_from(["hu", "jira"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Jira { cmd: None })));
    }

    #[test]
    fn parses_command_aliases() {
        // pd -> pagerduty
        let cli = Cli::try_parse_from(["hu", "pd", "oncall"]).unwrap();
        assert!(matches!(cli.command, Some(Command::PagerDuty { .. })));

        // nr -> newrelic
        let cli = Cli::try_parse_from(["hu", "nr", "incidents"]).unwrap();
        assert!(matches!(cli.command, Some(Command::NewRelic { .. })));
    }
}
