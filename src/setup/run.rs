//! `hu setup run` — full host bootstrap orchestrator.
//!
//! Walks the three phases (packages, dotfiles, ssh) in order, honoring
//! `--only` filtering, `--dry-run` short-circuit, and per-host overrides
//! from `[host.<hostname>]` blocks in setup.toml. Returns the aggregated
//! `Vec<StatusRow>` for display at the call site.

// reason: orchestrator wired by `hu setup run` (this chunk). Tests cover
// the surface; only-filter + host-override logic is unit-tested without I/O.
#![allow(dead_code)]

use anyhow::Result;

use crate::setup::cli::{PkgsArgs, RunArgs, RunPhase};
use crate::setup::config::SetupConfig;
use crate::setup::display::StatusRow;
use crate::setup::os::Os;
use crate::setup::ssh::OpClient;
use crate::setup::{dotfiles, pkgs, ssh};
use crate::util::shell::Shell;

/// Resolve the active host name for `[host.<hostname>]` overrides.
///
/// Reads `$HOSTNAME` first (settable for tests), then falls back to
/// `hostname::get()` if needed. Returns `"unknown"` on lookup failure.
pub fn current_hostname() -> String {
    if let Ok(h) = std::env::var("HOSTNAME") {
        if !h.is_empty() {
            return h;
        }
    }
    // Fall back to the platform call.
    std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
            } else {
                None
            }
        })
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Apply per-host overrides to a base config.
///
/// Currently merges `brew_extra` and `mise_extra` from the matching
/// `[host.<name>]` block onto the base package lists.
pub fn apply_host_overrides(mut config: SetupConfig, hostname: &str) -> SetupConfig {
    if let Some(over) = config.host.get(hostname).cloned() {
        for pkg in over.brew_extra {
            if !config.packages.brew.contains(&pkg) {
                config.packages.brew.push(pkg);
            }
        }
        for pkg in over.mise_extra {
            if !config.packages.mise.contains(&pkg) {
                config.packages.mise.push(pkg);
            }
        }
    }
    config
}

/// Full orchestrator. Walks phases in order: packages → dotfiles → ssh.
///
/// Phase selection follows `args.only`:
/// - `None` → all three phases run
/// - `Some(Pkgs)` → only packages
/// - `Some(Dotfiles)` → only dotfiles
/// - `Some(Ssh)` → only ssh
pub async fn run_full<S: Shell + ?Sized, O: OpClient + ?Sized>(
    shell: &S,
    op: &O,
    config: &SetupConfig,
    args: &RunArgs,
    os: &Os,
) -> Result<Vec<StatusRow>> {
    let mut rows = Vec::new();

    if should_run_phase(&args.only, RunPhase::Pkgs) {
        let pkgs_args = PkgsArgs {
            only: vec![],
            dry_run: args.dry_run,
        };
        let pkg_rows = pkgs::run(shell, config, &pkgs_args, os).await?;
        rows.extend(pkg_rows);
    }

    if should_run_phase(&args.only, RunPhase::Dotfiles) {
        if args.dry_run {
            for pkg in &config.dotfiles.packages {
                rows.push(
                    crate::setup::display::StatusRow::new(
                        "stow",
                        pkg,
                        crate::setup::types::Status::Skipped,
                    )
                    .with_note("dry-run"),
                );
            }
        } else {
            let df_rows = dotfiles::run(shell, &config.dotfiles).await;
            rows.extend(df_rows);
        }
    }

    if should_run_phase(&args.only, RunPhase::Ssh) {
        if args.dry_run {
            for item in &config.ssh.op_items {
                rows.push(
                    crate::setup::display::StatusRow::new(
                        "ssh",
                        item,
                        crate::setup::types::Status::Skipped,
                    )
                    .with_note("dry-run"),
                );
            }
        } else {
            let ssh_rows = ssh::run(op, &config.ssh).await;
            rows.extend(ssh_rows);
        }
    }

    Ok(rows)
}

fn should_run_phase(only: &Option<RunPhase>, phase: RunPhase) -> bool {
    match only {
        None => true,
        Some(p) => *p == phase,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup::cli::{RunArgs, RunPhase};
    use crate::setup::config::HostOverride;
    use crate::setup::ssh::MockOp;
    use crate::util::shell::FakeShell;

    fn run_args(only: Option<RunPhase>, dry_run: bool, yes: bool) -> RunArgs {
        RunArgs { only, dry_run, yes }
    }

    #[test]
    fn host_override_appends_brew_extra() {
        let mut cfg = SetupConfig::default();
        cfg.packages.brew = vec!["gh".into()];
        cfg.host.insert(
            "marauder".into(),
            HostOverride {
                brew_extra: vec!["nvtop".into()],
                mise_extra: vec![],
            },
        );
        let merged = apply_host_overrides(cfg, "marauder");
        assert!(merged.packages.brew.contains(&"gh".to_string()));
        assert!(merged.packages.brew.contains(&"nvtop".to_string()));
    }

    #[test]
    fn host_override_dedupes_existing() {
        let mut cfg = SetupConfig::default();
        cfg.packages.brew = vec!["gh".into(), "nvtop".into()];
        cfg.host.insert(
            "marauder".into(),
            HostOverride {
                brew_extra: vec!["nvtop".into()],
                mise_extra: vec![],
            },
        );
        let merged = apply_host_overrides(cfg, "marauder");
        let count = merged
            .packages
            .brew
            .iter()
            .filter(|p| *p == "nvtop")
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn host_override_no_match_returns_unchanged() {
        let mut cfg = SetupConfig::default();
        cfg.packages.brew = vec!["gh".into()];
        cfg.host.insert(
            "other".into(),
            HostOverride {
                brew_extra: vec!["nvtop".into()],
                mise_extra: vec![],
            },
        );
        let merged = apply_host_overrides(cfg.clone(), "marauder");
        assert_eq!(merged.packages.brew, cfg.packages.brew);
    }

    #[test]
    fn host_override_appends_mise_extra() {
        let mut cfg = SetupConfig::default();
        cfg.packages.mise = vec!["node@lts".into()];
        cfg.host.insert(
            "h".into(),
            HostOverride {
                brew_extra: vec![],
                mise_extra: vec!["go@latest".into()],
            },
        );
        let merged = apply_host_overrides(cfg, "h");
        assert!(merged.packages.mise.contains(&"go@latest".to_string()));
    }

    #[test]
    fn should_run_phase_returns_true_when_no_filter() {
        assert!(should_run_phase(&None, RunPhase::Pkgs));
        assert!(should_run_phase(&None, RunPhase::Dotfiles));
        assert!(should_run_phase(&None, RunPhase::Ssh));
    }

    #[test]
    fn should_run_phase_filters_to_one() {
        assert!(should_run_phase(&Some(RunPhase::Ssh), RunPhase::Ssh));
        assert!(!should_run_phase(&Some(RunPhase::Ssh), RunPhase::Pkgs));
        assert!(!should_run_phase(&Some(RunPhase::Ssh), RunPhase::Dotfiles));
    }

    #[tokio::test]
    async fn run_full_dry_run_emits_all_phases_skipped() {
        let shell = FakeShell::new();
        let op = MockOp::new();
        let mut cfg = SetupConfig::default();
        // Trim defaults to keep test compact
        cfg.packages.brew = vec!["gh".into()];
        cfg.packages.mise = vec!["node@lts".into()];
        cfg.dotfiles.packages = vec!["zsh".into()];
        cfg.ssh.op_items = vec!["SSH/id_test".into()];
        let rows = run_full(&shell, &op, &cfg, &run_args(None, true, false), &Os::Mac)
            .await
            .unwrap();
        // bootstrap + 1 brew + 1 mise + 1 stow + 1 ssh = 5
        assert_eq!(rows.len(), 5);
        for r in &rows {
            assert_eq!(r.status, crate::setup::types::Status::Skipped);
        }
        assert!(shell.calls().is_empty());
    }

    #[tokio::test]
    async fn run_full_only_pkgs_skips_other_phases() {
        let shell = FakeShell::new();
        let op = MockOp::new();
        let mut cfg = SetupConfig::default();
        cfg.packages.brew = vec!["gh".into()];
        cfg.packages.mise = vec![];
        cfg.dotfiles.packages = vec!["zsh".into()];
        cfg.ssh.op_items = vec!["SSH/id_test".into()];
        let rows = run_full(
            &shell,
            &op,
            &cfg,
            &run_args(Some(RunPhase::Pkgs), true, false),
            &Os::Mac,
        )
        .await
        .unwrap();
        // bootstrap + 1 brew = 2 rows; no stow/ssh
        assert!(!rows.iter().any(|r| r.category == "stow"));
        assert!(!rows.iter().any(|r| r.category == "ssh"));
    }

    #[tokio::test]
    async fn run_full_only_ssh_skips_pkgs_and_dotfiles() {
        let shell = FakeShell::new();
        let op = MockOp::new();
        let mut cfg = SetupConfig::default();
        cfg.packages.brew = vec!["gh".into()];
        cfg.dotfiles.packages = vec!["zsh".into()];
        cfg.ssh.op_items = vec!["SSH/id_test".into()];
        let rows = run_full(
            &shell,
            &op,
            &cfg,
            &run_args(Some(RunPhase::Ssh), true, false),
            &Os::Mac,
        )
        .await
        .unwrap();
        assert!(!rows.iter().any(|r| r.category == "brew"));
        assert!(!rows.iter().any(|r| r.category == "stow"));
        assert!(rows.iter().any(|r| r.category == "ssh"));
    }
}
