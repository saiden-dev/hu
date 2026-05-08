//! Dotfiles phase — clone (or refresh) + stow apply.
//!
//! Phase 3 splits into two responsibilities:
//! 1. **Clone** — fetch the dotfiles repo via `gh repo clone <repo> <dest>`,
//!    or detect an existing clone and skip.
//! 2. **Apply** — for each configured package directory, run
//!    `stow -R -d <clone_to> -t ~ <pkg>` (restow is idempotent: re-creates
//!    symlinks even if they already exist, handles partial state cleanly).
//!
//! Both steps go through the [`Shell`] chokepoint. No `Installer` trait
//! needed — this is a single-shot orchestration, not a registry walk.

// reason: dotfiles flow wired by `hu setup dotfiles` (chunk 3.2) and
// `setup run` (Phase 5). Tests cover the surface.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use crate::setup::config::DotfilesConfig;
use crate::setup::display::StatusRow;
use crate::setup::types::Status;
use crate::util::shell::Shell;

/// Resolve a `~`-prefixed path to an absolute path.
///
/// Pure-function alternative to `shellexpand` — keeps the dependency surface
/// thin. `$HOME` lookup is the only env touch and only happens for paths
/// that actually start with `~/`.
pub fn expand_tilde(raw: &str) -> PathBuf {
    if let Some(rest) = raw.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(raw)
}

/// True when `<dir>/.git` exists. Used to detect an already-cloned dotfiles
/// repo so we can skip the clone step.
pub fn is_git_repo(dir: &Path) -> bool {
    dir.join(".git").exists()
}

/// Idempotent clone of the dotfiles repo. If `<clone_to>/.git` already
/// exists, returns `Status::Already`. Otherwise runs `gh repo clone`.
pub async fn ensure_clone<S: Shell + ?Sized>(shell: &S, config: &DotfilesConfig) -> StatusRow {
    let dest = expand_tilde(&config.clone_to);
    if is_git_repo(&dest) {
        return StatusRow::new("dotfiles", &config.repo, Status::Already)
            .with_note(&format!("clone exists: {}", dest.display()));
    }
    let dest_str = dest.to_string_lossy().into_owned();
    let result = shell
        .run("gh", &["repo", "clone", &config.repo, &dest_str])
        .await;
    match result {
        Ok(out) if out.is_success() => {
            // Re-verify the clone landed.
            if is_git_repo(&dest) {
                StatusRow::new("dotfiles", &config.repo, Status::Installed)
                    .with_note(&format!("cloned to {}", dest.display()))
            } else {
                StatusRow::new("dotfiles", &config.repo, Status::Failed)
                    .with_note("gh clone reported success but .git/ is missing")
            }
        }
        Ok(out) => StatusRow::new("dotfiles", &config.repo, Status::Failed).with_note(&format!(
            "gh clone failed (exit {:?}): {}",
            out.status.code(),
            out.stderr.trim()
        )),
        Err(e) => StatusRow::new("dotfiles", &config.repo, Status::Failed)
            .with_note(&format!("gh clone errored: {}", e)),
    }
}

/// Apply one stow package: `stow -R -d <clone_to> -t ~ <pkg>`.
///
/// `-R` (restow) is idempotent — recreates symlinks each invocation, handles
/// partial state cleanly. Conflicts (existing non-symlink files in the way)
/// surface as exit-nonzero with stderr describing the path.
pub async fn stow_apply<S: Shell + ?Sized>(
    shell: &S,
    clone_to: &str,
    target: &str,
    package: &str,
) -> StatusRow {
    let result = shell
        .run("stow", &["-R", "-d", clone_to, "-t", target, package])
        .await;
    match result {
        Ok(out) if out.is_success() => StatusRow::new("stow", package, Status::Installed)
            .with_note(&format!("stowed → {}", target)),
        Ok(out) => {
            let note = parse_stow_conflicts(&out.stderr);
            StatusRow::new("stow", package, Status::Failed).with_note(&note)
        }
        Err(e) => StatusRow::new("stow", package, Status::Failed)
            .with_note(&format!("stow errored: {}", e)),
    }
}

/// Orchestrate the full dotfiles phase: clone (or skip) → stow each package.
///
/// Returns one `StatusRow` per step. If the clone fails the per-package
/// stow rows are skipped — there's nothing to stow from.
pub async fn run<S: Shell + ?Sized>(shell: &S, config: &DotfilesConfig) -> Vec<StatusRow> {
    let mut rows = Vec::with_capacity(1 + config.packages.len());
    let clone_row = ensure_clone(shell, config).await;
    let clone_satisfied = clone_row.status.is_satisfied();
    rows.push(clone_row);
    if !clone_satisfied {
        for pkg in &config.packages {
            rows.push(
                StatusRow::new("stow", pkg, Status::Skipped)
                    .with_note("clone failed — nothing to stow"),
            );
        }
        return rows;
    }
    let target = expand_tilde("~/").to_string_lossy().into_owned();
    let clone_to = expand_tilde(&config.clone_to)
        .to_string_lossy()
        .into_owned();
    for pkg in &config.packages {
        rows.push(stow_apply(shell, &clone_to, &target, pkg).await);
    }
    rows
}

/// Extract a concise conflict summary from stow stderr.
///
/// stow's stderr lists conflicting files line by line. We keep the gist
/// (first conflicting path + count) without dumping the whole blob into a
/// table cell.
pub fn parse_stow_conflicts(stderr: &str) -> String {
    let conflicts: Vec<&str> = stderr
        .lines()
        .filter(|l| {
            l.contains("existing target")
                || l.contains("CONFLICT")
                || l.contains("would cause conflicts")
        })
        .collect();
    if conflicts.is_empty() {
        return format!("stow failed: {}", stderr.trim());
    }
    let first = conflicts.first().copied().unwrap_or_default();
    if conflicts.len() == 1 {
        first.trim().to_string()
    } else {
        format!("{} (+{} more conflicts)", first.trim(), conflicts.len() - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::shell::FakeShell;

    fn dotfiles_config() -> DotfilesConfig {
        DotfilesConfig {
            repo: "aladac/dotfiles".into(),
            branch: "main".into(),
            clone_to: "/tmp/hu-test-dotfiles".into(),
            strategy: "stow".into(),
            packages: vec!["zsh".into(), "kitty".into()],
        }
    }

    #[test]
    fn expand_tilde_replaces_with_home() {
        let original = std::env::var("HOME").ok();
        std::env::set_var("HOME", "/Users/test");
        let path = expand_tilde("~/Projects/dotfiles");
        assert_eq!(path, PathBuf::from("/Users/test/Projects/dotfiles"));
        if let Some(h) = original {
            std::env::set_var("HOME", h);
        }
    }

    #[test]
    fn expand_tilde_passes_through_non_tilde() {
        assert_eq!(expand_tilde("/abs/path"), PathBuf::from("/abs/path"));
        assert_eq!(
            expand_tilde("relative/path"),
            PathBuf::from("relative/path")
        );
    }

    #[test]
    fn is_git_repo_detects_dot_git_dir() {
        let temp = std::env::temp_dir().join("hu-is-git-repo-test");
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(temp.join(".git")).unwrap();
        assert!(is_git_repo(&temp));
        std::fs::remove_dir_all(&temp).unwrap();
    }

    #[test]
    fn is_git_repo_returns_false_when_missing() {
        assert!(!is_git_repo(Path::new("/nonexistent/directory")));
    }

    #[tokio::test]
    async fn ensure_clone_skips_when_already_cloned() {
        let temp = std::env::temp_dir().join("hu-ensure-clone-skip");
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(temp.join(".git")).unwrap();
        let mut config = dotfiles_config();
        config.clone_to = temp.to_string_lossy().into_owned();
        let shell = FakeShell::new();
        let row = ensure_clone(&shell, &config).await;
        assert_eq!(row.status, Status::Already);
        assert!(shell.calls().is_empty());
        std::fs::remove_dir_all(&temp).unwrap();
    }

    #[tokio::test]
    async fn ensure_clone_failed_when_gh_errors() {
        let mut config = dotfiles_config();
        config.clone_to = "/tmp/hu-ensure-clone-fail-XXXX".into();
        let _ = std::fs::remove_dir_all(&config.clone_to);
        let shell = FakeShell::new();
        shell.expect(
            "gh",
            &[
                "repo",
                "clone",
                "aladac/dotfiles",
                "/tmp/hu-ensure-clone-fail-XXXX",
            ],
            "",
            1,
        );
        let row = ensure_clone(&shell, &config).await;
        assert_eq!(row.status, Status::Failed);
        assert!(row.note.contains("gh clone failed"));
    }

    #[tokio::test]
    async fn stow_apply_marks_installed_on_success() {
        let shell = FakeShell::new();
        shell.expect(
            "stow",
            &["-R", "-d", "/clone", "-t", "/home/u", "zsh"],
            "",
            0,
        );
        let row = stow_apply(&shell, "/clone", "/home/u", "zsh").await;
        assert_eq!(row.status, Status::Installed);
        assert!(row.note.contains("stowed"));
    }

    #[tokio::test]
    async fn stow_apply_marks_failed_with_conflict_summary() {
        let shell = FakeShell::new();
        shell.expect(
            "stow",
            &["-R", "-d", "/clone", "-t", "/home/u", "zsh"],
            "",
            1,
        );
        // FakeShell always returns empty stderr on expect; stuff a conflict via raw stdin
        // — we test parse_stow_conflicts directly below to cover the parsing branch.
        let row = stow_apply(&shell, "/clone", "/home/u", "zsh").await;
        assert_eq!(row.status, Status::Failed);
    }

    #[test]
    fn parse_stow_conflicts_summarizes_one_conflict() {
        let stderr = "existing target is not owned by stow: .zshrc\n";
        assert_eq!(parse_stow_conflicts(stderr), stderr.trim());
    }

    #[test]
    fn parse_stow_conflicts_counts_extras() {
        let stderr = "\
existing target is not owned by stow: .zshrc
existing target is not owned by stow: .gitconfig
existing target is not owned by stow: .vimrc
";
        let summary = parse_stow_conflicts(stderr);
        assert!(summary.contains(".zshrc"));
        assert!(summary.contains("+2 more"));
    }

    #[test]
    fn parse_stow_conflicts_falls_back_when_no_keyword() {
        let stderr = "stow: command not found\n";
        let summary = parse_stow_conflicts(stderr);
        assert!(summary.starts_with("stow failed:"));
    }

    #[tokio::test]
    async fn run_skips_stow_when_clone_fails() {
        let mut config = dotfiles_config();
        config.clone_to = "/tmp/hu-run-clone-fails".into();
        let _ = std::fs::remove_dir_all(&config.clone_to);
        let shell = FakeShell::new();
        shell.expect(
            "gh",
            &[
                "repo",
                "clone",
                "aladac/dotfiles",
                "/tmp/hu-run-clone-fails",
            ],
            "",
            1,
        );
        let rows = run(&shell, &config).await;
        assert_eq!(rows.len(), 1 + config.packages.len());
        assert_eq!(rows[0].status, Status::Failed);
        for stow_row in &rows[1..] {
            assert_eq!(stow_row.status, Status::Skipped);
            assert!(stow_row.note.contains("clone failed"));
        }
    }

    #[tokio::test]
    async fn run_stows_each_configured_package_when_clone_present() {
        let temp = std::env::temp_dir().join("hu-run-clone-present");
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(temp.join(".git")).unwrap();
        let mut config = dotfiles_config();
        config.clone_to = temp.to_string_lossy().into_owned();
        config.packages = vec!["zsh".into(), "kitty".into()];

        let target = expand_tilde("~/").to_string_lossy().into_owned();
        let clone_to = temp.to_string_lossy().into_owned();
        let shell = FakeShell::new();
        shell.expect(
            "stow",
            &["-R", "-d", &clone_to, "-t", &target, "zsh"],
            "",
            0,
        );
        shell.expect(
            "stow",
            &["-R", "-d", &clone_to, "-t", &target, "kitty"],
            "",
            0,
        );
        let rows = run(&shell, &config).await;
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].status, Status::Already);
        assert_eq!(rows[1].status, Status::Installed);
        assert_eq!(rows[2].status, Status::Installed);
        std::fs::remove_dir_all(&temp).unwrap();
    }
}
