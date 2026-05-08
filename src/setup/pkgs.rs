//! Service layer for `hu setup pkgs`.
//!
//! Orchestrates T0 bootstrap (linuxbrew prereqs + brew) then walks the
//! configured brew package list, calling `BrewInstaller::ensure` per package.
//! Returns one `StatusRow` per step for the display module to render.

// reason: collector wired by `hu setup pkgs` (this chunk) and `setup run` (Phase 5).
#![allow(dead_code)]

use anyhow::Result;

use crate::setup::bootstrap::{ensure_brew, ensure_linuxbrew_prereqs};
use crate::setup::cli::PkgsArgs;
use crate::setup::config::SetupConfig;
use crate::setup::display::StatusRow;
use crate::setup::os::Os;
use crate::setup::packages::{BrewInstaller, InstallResult, Installer, MiseInstaller};
use crate::setup::types::Status;
use crate::util::shell::Shell;

/// Plan + execute the brew package phase.
///
/// Order of operations:
/// 1. Linuxbrew apt prereqs (skipped on macOS)
/// 2. T0: ensure brew itself is on PATH
/// 3. T1: ensure each filtered brew package is installed
///
/// `--dry-run` short-circuits at step 1: every step reports `Skipped(dry-run)`
/// without touching the shell.
pub async fn run<S: Shell + ?Sized>(
    shell: &S,
    config: &SetupConfig,
    args: &PkgsArgs,
    os: &Os,
) -> Result<Vec<StatusRow>> {
    let filtered_brew = filter_packages(&config.packages.brew, &args.only);
    let filtered_mise = filter_packages(&config.packages.mise, &args.only);

    if args.dry_run {
        return Ok(dry_run_rows(os, &filtered_brew, &filtered_mise));
    }

    let mut rows = Vec::new();

    // 1. Linuxbrew prereqs (or skip on macOS)
    for r in ensure_linuxbrew_prereqs(shell, os).await {
        rows.push(install_result_to_row("linuxbrew-prereq", &r));
    }

    // 2. T0 brew bootstrap
    let brew_result = ensure_brew(shell).await;
    rows.push(install_result_to_row("bootstrap", &brew_result));
    if !brew_result.status.is_satisfied() {
        // Brew missing → can't proceed with T1 / T2 packages (mise comes via brew).
        return Ok(rows);
    }

    // 3. T1 brew packages
    let brew = BrewInstaller;
    for pkg in &filtered_brew {
        let r = brew.ensure(shell, pkg).await;
        rows.push(install_result_to_row("brew", &r));
    }

    // 4. T2 mise-managed runtimes (only if mise itself is reachable)
    if !filtered_mise.is_empty() && shell.which("mise").await {
        let mise = MiseInstaller;
        for pkg in &filtered_mise {
            let r = mise.ensure(shell, pkg).await;
            rows.push(install_result_to_row("mise", &r));
        }
    } else if !filtered_mise.is_empty() {
        // mise not on PATH yet — can happen when brew just installed it but
        // shims aren't in this shell's environment. Surface as Failed so
        // Pilot knows to re-run after rehashing PATH.
        for pkg in &filtered_mise {
            rows.push(
                StatusRow::new("mise", pkg, Status::Failed)
                    .with_note("mise not on PATH — re-run after `eval \"$(mise activate)\"`"),
            );
        }
    }

    Ok(rows)
}

/// Apply the `--only` filter to a package list (brew or mise).
///
/// Empty filter → no filtering (all configured packages). Filter values that
/// don't match a configured package are silently dropped. Matches against the
/// full configured name (`node@lts` matches `node@lts`, not `node`).
pub fn filter_packages(configured: &[String], only: &[String]) -> Vec<String> {
    if only.is_empty() {
        return configured.to_vec();
    }
    configured
        .iter()
        .filter(|p| only.iter().any(|o| o == *p))
        .cloned()
        .collect()
}

fn dry_run_rows(os: &Os, brew: &[String], mise: &[String]) -> Vec<StatusRow> {
    let mut rows = Vec::new();
    if os.is_linux() {
        for pkg in crate::setup::bootstrap::LINUXBREW_APT_PREREQS {
            rows.push(
                StatusRow::new("linuxbrew-prereq", pkg, Status::Skipped).with_note("dry-run"),
            );
        }
    }
    rows.push(StatusRow::new("bootstrap", "brew", Status::Skipped).with_note("dry-run"));
    for pkg in brew {
        rows.push(StatusRow::new("brew", pkg, Status::Skipped).with_note("dry-run"));
    }
    for pkg in mise {
        rows.push(StatusRow::new("mise", pkg, Status::Skipped).with_note("dry-run"));
    }
    rows
}

fn install_result_to_row(category: &str, r: &InstallResult) -> StatusRow {
    StatusRow::new(category, &r.package, r.status).with_note(&r.note)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup::cli::PkgsArgs;
    use crate::util::shell::FakeShell;

    fn args(only: Vec<&str>, dry_run: bool) -> PkgsArgs {
        PkgsArgs {
            only: only.into_iter().map(String::from).collect(),
            dry_run,
        }
    }

    fn config_with_brew(pkgs: &[&str]) -> SetupConfig {
        let mut cfg = SetupConfig::default();
        cfg.packages.brew = pkgs.iter().map(|s| s.to_string()).collect();
        cfg.packages.mise = vec![];
        cfg
    }

    fn config_with_brew_and_mise(brew: &[&str], mise: &[&str]) -> SetupConfig {
        let mut cfg = SetupConfig::default();
        cfg.packages.brew = brew.iter().map(|s| s.to_string()).collect();
        cfg.packages.mise = mise.iter().map(|s| s.to_string()).collect();
        cfg
    }

    #[test]
    fn filter_empty_returns_full_list() {
        let configured = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let only: Vec<String> = vec![];
        assert_eq!(filter_packages(&configured, &only), configured);
    }

    #[test]
    fn filter_keeps_matching_packages() {
        let configured = vec!["gh".to_string(), "jq".to_string(), "op".to_string()];
        let only = vec!["gh".to_string(), "op".to_string()];
        assert_eq!(filter_packages(&configured, &only), vec!["gh", "op"]);
    }

    #[test]
    fn filter_drops_unconfigured_names() {
        let configured = vec!["gh".to_string()];
        let only = vec!["gh".to_string(), "nonexistent".to_string()];
        assert_eq!(filter_packages(&configured, &only), vec!["gh"]);
    }

    #[tokio::test]
    async fn dry_run_on_macos_skips_prereqs() {
        let shell = FakeShell::new();
        let cfg = config_with_brew(&["gh", "jq"]);
        let rows = run(&shell, &cfg, &args(vec![], true), &Os::Mac)
            .await
            .unwrap();
        // No linuxbrew-prereq rows on macOS dry-run
        assert!(!rows.iter().any(|r| r.category == "linuxbrew-prereq"));
        // bootstrap + 2 brew packages
        assert_eq!(rows.len(), 3);
        assert!(rows.iter().all(|r| r.status == Status::Skipped));
        assert!(shell.calls().is_empty());
    }

    #[tokio::test]
    async fn dry_run_on_linux_includes_prereq_rows() {
        let shell = FakeShell::new();
        let cfg = config_with_brew(&["gh"]);
        let os = Os::Linux {
            distro: "ubuntu".into(),
        };
        let rows = run(&shell, &cfg, &args(vec![], true), &os).await.unwrap();
        let prereq_rows = rows
            .iter()
            .filter(|r| r.category == "linuxbrew-prereq")
            .count();
        assert_eq!(prereq_rows, 5); // build-essential, curl, git, procps, file
        assert!(shell.calls().is_empty());
    }

    #[tokio::test]
    async fn run_skips_t1_when_brew_bootstrap_fails() {
        let shell = FakeShell::new();
        // brew missing + install fails
        shell.expect("which", &["brew"], "", 1);
        shell.expect(
            "bash",
            &["-c", crate::setup::bootstrap::BREW_INSTALL],
            "",
            1,
        );
        let cfg = config_with_brew(&["gh"]);
        let rows = run(&shell, &cfg, &args(vec![], false), &Os::Mac)
            .await
            .unwrap();
        // bootstrap row marked Failed; no T1 rows added
        let brew_rows = rows.iter().filter(|r| r.category == "brew").count();
        assert_eq!(brew_rows, 0);
        let bootstrap_row = rows.iter().find(|r| r.category == "bootstrap").unwrap();
        assert_eq!(bootstrap_row.status, Status::Failed);
    }

    #[tokio::test]
    async fn run_walks_all_filtered_packages_when_brew_present() {
        let shell = FakeShell::new();
        shell.expect("which", &["brew"], "/opt/homebrew/bin/brew\n", 0);
        shell.expect("brew", &["list", "--versions", "gh"], "gh 2.50.0\n", 0);
        shell.expect("brew", &["list", "--versions", "jq"], "", 1);
        shell.expect("brew", &["install", "jq"], "Successfully installed jq\n", 0);
        shell.expect_sequence(
            "brew",
            &["list", "--versions", "jq"],
            &[("", 1), ("jq 1.7\n", 0)],
        );
        let cfg = config_with_brew(&["gh", "jq"]);
        let rows = run(&shell, &cfg, &args(vec![], false), &Os::Mac)
            .await
            .unwrap();
        let brew_rows: Vec<_> = rows.iter().filter(|r| r.category == "brew").collect();
        assert_eq!(brew_rows.len(), 2);
        // gh already, jq installed
        assert_eq!(brew_rows[0].name, "gh");
        assert_eq!(brew_rows[0].status, Status::Already);
        assert_eq!(brew_rows[1].name, "jq");
        assert_eq!(brew_rows[1].status, Status::Installed);
    }

    #[tokio::test]
    async fn run_walks_brew_then_mise_when_both_configured() {
        let shell = FakeShell::new();
        shell.expect("which", &["brew"], "/opt/homebrew/bin/brew\n", 0);
        shell.expect("brew", &["list", "--versions", "gh"], "gh 2.50.0\n", 0);
        shell.expect("which", &["mise"], "/opt/homebrew/bin/mise\n", 0);
        shell.expect("mise", &["current", "node"], "20.10.0\n", 0);
        let cfg = config_with_brew_and_mise(&["gh"], &["node@lts"]);
        let rows = run(&shell, &cfg, &args(vec![], false), &Os::Mac)
            .await
            .unwrap();
        assert!(rows.iter().any(|r| r.category == "brew" && r.name == "gh"));
        assert!(rows
            .iter()
            .any(|r| r.category == "mise" && r.name == "node@lts"));
        let mise_row = rows.iter().find(|r| r.category == "mise").unwrap();
        assert_eq!(mise_row.status, Status::Already);
    }

    #[tokio::test]
    async fn run_marks_mise_failed_when_mise_not_on_path() {
        let shell = FakeShell::new();
        shell.expect("which", &["brew"], "/opt/homebrew/bin/brew\n", 0);
        // mise not yet on PATH
        shell.expect("which", &["mise"], "", 1);
        let cfg = config_with_brew_and_mise(&[], &["node@lts"]);
        let rows = run(&shell, &cfg, &args(vec![], false), &Os::Mac)
            .await
            .unwrap();
        let mise_row = rows.iter().find(|r| r.category == "mise").unwrap();
        assert_eq!(mise_row.status, Status::Failed);
        assert!(mise_row.note.contains("mise activate"));
    }

    #[tokio::test]
    async fn dry_run_includes_mise_rows() {
        let shell = FakeShell::new();
        let cfg = config_with_brew_and_mise(&["gh"], &["node@lts", "rust@latest"]);
        let rows = run(&shell, &cfg, &args(vec![], true), &Os::Mac)
            .await
            .unwrap();
        let mise_count = rows.iter().filter(|r| r.category == "mise").count();
        assert_eq!(mise_count, 2);
        assert!(shell.calls().is_empty());
    }

    #[tokio::test]
    async fn run_with_only_filter_walks_subset() {
        let shell = FakeShell::new();
        shell.expect("which", &["brew"], "/opt/homebrew/bin/brew\n", 0);
        shell.expect("brew", &["list", "--versions", "gh"], "gh 2.50.0\n", 0);
        let cfg = config_with_brew(&["gh", "jq", "op"]);
        let rows = run(&shell, &cfg, &args(vec!["gh"], false), &Os::Mac)
            .await
            .unwrap();
        let brew_rows: Vec<_> = rows.iter().filter(|r| r.category == "brew").collect();
        assert_eq!(brew_rows.len(), 1);
        assert_eq!(brew_rows[0].name, "gh");
    }
}
