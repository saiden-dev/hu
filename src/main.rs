use clap::{CommandFactory, Parser};

mod cli;
mod context;
mod data;
mod docs;
mod eks;
mod gh;
mod git;
mod install;
mod jira;
mod newrelic;
mod pagerduty;
mod pipeline;
mod read;
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
            return slack::run(cmd).await;
        }
        Command::Slack { cmd: None } => {
            print_subcommand_help("slack")?;
        }
        Command::PagerDuty { cmd: Some(cmd) } => {
            return pagerduty::run(cmd).await;
        }
        Command::PagerDuty { cmd: None } => {
            print_subcommand_help("pagerduty")?;
        }
        Command::Sentry { cmd: Some(cmd) } => {
            return sentry::run(cmd).await;
        }
        Command::Sentry { cmd: None } => {
            print_subcommand_help("sentry")?;
        }
        Command::NewRelic { cmd: Some(cmd) } => {
            return newrelic::run(cmd).await;
        }
        Command::NewRelic { cmd: None } => {
            print_subcommand_help("newrelic")?;
        }
        Command::Eks { cmd: Some(cmd) } => {
            return eks::run(cmd).await;
        }
        Command::Eks { cmd: None } => {
            print_subcommand_help("eks")?;
        }
        Command::Pipeline { cmd: Some(cmd) } => {
            return pipeline::run(cmd).await;
        }
        Command::Pipeline { cmd: None } => {
            print_subcommand_help("pipeline")?;
        }
        Command::Utils { cmd: Some(cmd) } => {
            return utils::run_command(cmd).await;
        }
        Command::Utils { cmd: None } => {
            print_subcommand_help("utils")?;
        }
        Command::Context { cmd: Some(cmd) } => {
            return context::run_command(cmd).await;
        }
        Command::Context { cmd: None } => {
            print_subcommand_help("context")?;
        }
        Command::Read(args) => {
            return read::run(args);
        }
        Command::Data { cmd: Some(cmd) } => {
            return data::run_command(cmd).await;
        }
        Command::Data { cmd: None } => {
            print_subcommand_help("data")?;
        }
        Command::Install { cmd: Some(cmd) } => {
            return install::run_command(cmd).await;
        }
        Command::Install { cmd: None } => {
            print_subcommand_help("install")?;
        }
        Command::Docs { cmd: Some(cmd) } => {
            return docs::run_command(cmd).await;
        }
        Command::Docs { cmd: None } => {
            print_subcommand_help("docs")?;
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
