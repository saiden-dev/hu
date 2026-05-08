//! `hu setup` — universal fresh-host bootstrap.
//!
//! Runs on a clean macOS or Linux host and converges the system to the
//! configured desired state: packages, dotfiles, SSH keys.
//!
//! Each step follows the idempotency contract `check → skip-or-act → re-verify`.

mod bootstrap;
mod cli;
mod config;
mod display;
mod dotfiles;
mod os;
mod packages;
mod pkgs;
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
        SetupCommand::Pkgs(args) => run_pkgs(args).await,
        SetupCommand::Dotfiles => run_dotfiles().await,
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

async fn run_dotfiles() -> Result<()> {
    let os = Os::detect()?;
    let cfg = config::load().context("load setup.toml")?;
    let shell = RealShell;
    println!("{} host: {}", "◆".cyan(), os.label());
    println!(
        "{} dotfiles: {} → {}",
        "◆".cyan(),
        cfg.dotfiles.repo,
        cfg.dotfiles.clone_to
    );
    let rows = dotfiles::run(&shell, &cfg.dotfiles).await;
    println!("{}", display::render(&rows));
    println!("{}", display::summary(&rows));
    let any_failed = rows.iter().any(|r| r.status == types::Status::Failed);
    if any_failed {
        bail!("dotfiles phase had failures — see table above");
    }
    Ok(())
}

async fn run_pkgs(args: cli::PkgsArgs) -> Result<()> {
    let os = Os::detect()?;
    let cfg = config::load().context("load setup.toml")?;
    let shell = RealShell;
    println!("{} host: {}", "◆".cyan(), os.label());
    if args.dry_run {
        println!("{} dry-run — no changes will be made", "◐".yellow());
    }
    let rows = pkgs::run(&shell, &cfg, &args, &os).await?;
    println!("{}", display::render(&rows));
    println!("{}", display::summary(&rows));
    let any_failed = rows.iter().any(|r| r.status == types::Status::Failed);
    if any_failed {
        bail!("one or more packages failed — see table above");
    }
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
