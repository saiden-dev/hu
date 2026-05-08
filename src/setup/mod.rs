//! `hu setup` — universal fresh-host bootstrap.
//!
//! Runs on a clean macOS or Linux host and converges the system to the
//! configured desired state: packages, dotfiles, SSH keys.
//!
//! Each step follows the idempotency contract `check → skip-or-act → re-verify`.

mod cli;
mod types;

pub use cli::SetupCommand;

use anyhow::{bail, Result};

/// Dispatch entry point — called from `main.rs`.
pub async fn run_command(cmd: SetupCommand) -> Result<()> {
    match cmd {
        SetupCommand::Status | SetupCommand::Preview => {
            bail!("hu setup status: not yet implemented (Phase 0 chunk 0.4)");
        }
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
        SetupCommand::Config { cmd: _ } => {
            bail!("hu setup config: not yet implemented (Phase 0 chunk 0.3)");
        }
    }
}
