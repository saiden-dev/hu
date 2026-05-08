//! `hu setup` — universal fresh-host bootstrap.
//!
//! Runs on a clean macOS or Linux host and converges the system to the
//! configured desired state: packages, dotfiles, SSH keys.
//!
//! Each step follows the idempotency contract `check → skip-or-act → re-verify`.

mod cli;
mod config;
mod display;
mod os;
mod status;
mod types;

pub use cli::SetupCommand;

use anyhow::{bail, Context, Result};
use owo_colors::OwoColorize;

use cli::ConfigCommand;
use os::Os;

use crate::util::shell::RealShell;

/// Dispatch entry point — called from `main.rs`.
pub async fn run_command(cmd: SetupCommand) -> Result<()> {
    match cmd {
        SetupCommand::Status | SetupCommand::Preview => run_status().await,
        SetupCommand::Run(_) => {
            bail!("hu setup run: not yet implemented (Phase 5)");
        }
        SetupCommand::Pkgs(_) => {
            bail!("hu setup pkgs: not yet implemented (Phase 1)");
        }
        SetupCommand::Dotfiles => {
            bail!("hu setup dotfiles: not yet implemented (Phase 3)");
        }
        SetupCommand::Ssh => {
            bail!("hu setup ssh: not yet implemented (Phase 4)");
        }
        SetupCommand::Config { cmd } => run_config(cmd).await,
    }
}

async fn run_config(cmd: Option<ConfigCommand>) -> Result<()> {
    let Some(cmd) = cmd else {
        // Default action: show path
        return show_config_path();
    };
    match cmd {
        ConfigCommand::Init => init_config(),
        ConfigCommand::Path => show_config_path(),
    }
}

fn init_config() -> Result<()> {
    let outcome = config::init_default().context("init setup.toml")?;
    if outcome.existed {
        println!(
            "{} setup.toml already exists at {}",
            "◐".yellow(),
            outcome.path.display()
        );
    } else {
        println!(
            "{} wrote default setup.toml to {}",
            "✓".green(),
            outcome.path.display()
        );
    }
    Ok(())
}

async fn run_status() -> Result<()> {
    let os = Os::detect()?;
    let cfg = config::load().context("load setup.toml")?;
    let shell = RealShell;
    let rows = status::collect(&shell, &cfg).await?;
    println!("{} host: {}", "◆".cyan(), os.label());
    println!("{}", display::render(&rows));
    println!("{}", display::summary(&rows));
    Ok(())
}

fn show_config_path() -> Result<()> {
    match config::config_path() {
        Some(path) => {
            let exists = path.exists();
            let icon = if exists {
                "✓".green().to_string()
            } else {
                "○".dimmed().to_string()
            };
            println!("{} {}", icon, path.display());
            if !exists {
                println!("  (not yet created — run `hu setup config init`)");
            }
            Ok(())
        }
        None => bail!("could not resolve config directory for hu on this platform"),
    }
}
