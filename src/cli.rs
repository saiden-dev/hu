use clap::{Parser, Subcommand};

use crate::context::ContextCommand;
use crate::cron::CronCommand;
use crate::data::DataCommand;
use crate::docs::DocsCommand;
use crate::eks::EksCommand;
use crate::gh::GhCommand;
use crate::install::InstallCommand;
use crate::jira::JiraCommand;
use crate::newrelic::NewRelicCommand;
use crate::pagerduty::PagerDutyCommand;
use crate::pipeline::PipelineCommand;
use crate::read::ReadArgs;
use crate::sentry::SentryCommand;
use crate::shell::ShellCommand;
use crate::slack::SlackCommands;
use crate::utils::UtilsCommand;

#[derive(Parser)]
#[command(name = "hu")]
#[command(about = "Dev workflow CLI", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Jira operations (tickets, sprint, search)
    Jira {
        #[command(subcommand)]
        cmd: Option<JiraCommand>,
    },

    /// GitHub operations (prs, runs, failures)
    Gh {
        #[command(subcommand)]
        cmd: Option<GhCommand>,
    },

    /// Slack operations (messages, channels)
    Slack {
        #[command(subcommand)]
        cmd: Option<SlackCommands>,
    },

    /// PagerDuty (oncall, alerts)
    #[command(name = "pagerduty", alias = "pd")]
    PagerDuty {
        #[command(subcommand)]
        cmd: Option<PagerDutyCommand>,
    },

    /// Sentry (issues, errors)
    Sentry {
        #[command(subcommand)]
        cmd: Option<SentryCommand>,
    },

    /// NewRelic (incidents, queries)
    #[command(name = "newrelic", alias = "nr")]
    NewRelic {
        #[command(subcommand)]
        cmd: Option<NewRelicCommand>,
    },

    /// EKS pod access (list, exec, logs)
    Eks {
        #[command(subcommand)]
        cmd: Option<EksCommand>,
    },

    /// CodePipeline status (read-only)
    Pipeline {
        #[command(subcommand)]
        cmd: Option<PipelineCommand>,
    },

    /// Utility commands (fetch-html, grep)
    Utils {
        #[command(subcommand)]
        cmd: Option<UtilsCommand>,
    },

    /// Session context tracking (prevent duplicate file reads)
    Context {
        #[command(subcommand)]
        cmd: Option<ContextCommand>,
    },

    /// Smart file reading (outline, interface, around, diff)
    Read(ReadArgs),

    /// Claude Code session data (sync, stats, search)
    Data {
        #[command(subcommand)]
        cmd: Option<DataCommand>,
    },

    /// Install hu hooks and commands to Claude Code
    Install {
        #[command(subcommand)]
        cmd: Option<InstallCommand>,
    },

    /// Documentation management (add, get, list, remove, sync)
    Docs {
        #[command(subcommand)]
        cmd: Option<DocsCommand>,
    },

    /// Cron job management (add, list, remove)
    Cron {
        #[command(subcommand)]
        cmd: Option<CronCommand>,
    },

    /// Shell command wrappers (ls, etc.)
    Shell {
        #[command(subcommand)]
        cmd: Option<ShellCommand>,
    },
}
