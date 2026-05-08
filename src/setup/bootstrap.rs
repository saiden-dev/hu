//! T0 bootstrap — make the host capable of running the installers.
//!
//! On macOS this just verifies `brew` is on PATH. On Linux we additionally
//! ensure the apt prereqs that linuxbrew needs (`build-essential`, `curl`,
//! `git`, `procps`, `file`).
//!
//! Per doctrine §9: every step is `check → skip-or-act → re-verify`. Per §1:
//! Shell chokepoint covers all I/O, no extra trait wrappers per binary.

// reason: bootstrap functions wired by Phase 1 chunk 1.3 (`hu setup pkgs`)
// and Phase 5 (`hu setup run`). Tests cover the surface now.
#![allow(dead_code)]

use anyhow::Result;

use crate::setup::os::Os;
use crate::setup::packages::InstallResult;
use crate::util::shell::Shell;

/// Official Homebrew installer command.
///
/// Runs the upstream install script via `bash -c "$(curl …)"`. NONINTERACTIVE
/// flag skips the press-RETURN prompt the script normally requires.
const BREW_INSTALL: &str = "NONINTERACTIVE=1 /bin/bash -c \
     \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\"";

/// Linuxbrew apt prereqs (per the official Homebrew on Linux requirements).
pub const LINUXBREW_APT_PREREQS: &[&str] = &["build-essential", "curl", "git", "procps", "file"];

/// Ensure Homebrew is installed. Idempotent.
///
/// On macOS: checks `which brew`, installs via the upstream script if missing.
/// On Linux: same flow; the caller is responsible for ensuring apt prereqs
/// first via [`ensure_linuxbrew_prereqs`].
pub async fn ensure_brew<S: Shell + ?Sized>(shell: &S) -> InstallResult {
    if shell.which("brew").await {
        return InstallResult::already("brew");
    }
    let result = shell.run("bash", &["-c", BREW_INSTALL]).await;
    match result {
        Ok(out) if out.is_success() => {}
        Ok(out) => {
            return InstallResult::failed(
                "brew",
                &format!(
                    "brew install script exited {:?}: {}",
                    out.status.code(),
                    out.stderr.trim()
                ),
            );
        }
        Err(e) => {
            return InstallResult::failed("brew", &format!("brew install failed: {}", e));
        }
    }
    if shell.which("brew").await {
        InstallResult::installed("brew")
    } else {
        InstallResult::failed(
            "brew",
            "install reported success but `which brew` still fails",
        )
    }
}

/// Ensure linuxbrew apt prereqs are installed. Idempotent. Skips on non-Linux.
pub async fn ensure_linuxbrew_prereqs<S: Shell + ?Sized>(shell: &S, os: &Os) -> Vec<InstallResult> {
    if !os.is_linux() {
        return vec![InstallResult::skipped(
            "linuxbrew-prereqs",
            "not on linux — skipped",
        )];
    }
    let mut results = Vec::with_capacity(LINUXBREW_APT_PREREQS.len());
    for pkg in LINUXBREW_APT_PREREQS {
        results.push(ensure_apt_pkg(shell, pkg).await);
    }
    results
}

/// Ensure one apt package is installed via `dpkg -s` check + `apt-get install`.
async fn ensure_apt_pkg<S: Shell + ?Sized>(shell: &S, pkg: &str) -> InstallResult {
    if apt_check(shell, pkg).await {
        return InstallResult::already(pkg);
    }
    if let Err(e) = apt_install(shell, pkg).await {
        return InstallResult::failed(pkg, &format!("apt-get install failed: {}", e));
    }
    if apt_check(shell, pkg).await {
        InstallResult::installed(pkg)
    } else {
        InstallResult::failed(pkg, "install reported success but dpkg -s still fails")
    }
}

async fn apt_check<S: Shell + ?Sized>(shell: &S, pkg: &str) -> bool {
    match shell.run("dpkg", &["-s", pkg]).await {
        Ok(out) => out.is_success(),
        Err(_) => false,
    }
}

async fn apt_install<S: Shell + ?Sized>(shell: &S, pkg: &str) -> Result<()> {
    let out = shell
        .run("sudo", &["apt-get", "install", "-y", pkg])
        .await?;
    if !out.is_success() {
        anyhow::bail!(
            "apt-get install -y {} exited {:?}: {}",
            pkg,
            out.status.code(),
            out.stderr.trim()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup::types::Status;
    use crate::util::shell::FakeShell;

    #[tokio::test]
    async fn ensure_brew_skips_when_already_present() {
        let shell = FakeShell::new();
        shell.expect("which", &["brew"], "/opt/homebrew/bin/brew\n", 0);
        let r = ensure_brew(&shell).await;
        assert_eq!(r.status, Status::Already);
        assert_eq!(shell.calls().len(), 1);
    }

    #[tokio::test]
    async fn ensure_brew_installs_when_missing_and_re_verifies_green() {
        let shell = FakeShell::new();
        shell.expect_sequence(
            "which",
            &["brew"],
            &[("", 1), ("/opt/homebrew/bin/brew\n", 0)],
        );
        shell.expect("bash", &["-c", BREW_INSTALL], "Homebrew installed\n", 0);
        let r = ensure_brew(&shell).await;
        assert_eq!(r.status, Status::Installed);
        // 3 calls: check, install, re-check
        assert_eq!(shell.calls().len(), 3);
    }

    #[tokio::test]
    async fn ensure_brew_marks_failed_when_install_lies() {
        let shell = FakeShell::new();
        shell.expect("which", &["brew"], "", 1);
        shell.expect("bash", &["-c", BREW_INSTALL], "Homebrew installed\n", 0);
        let r = ensure_brew(&shell).await;
        assert_eq!(r.status, Status::Failed);
        assert!(r.note.contains("install reported success"));
    }

    #[tokio::test]
    async fn ensure_brew_marks_failed_when_install_errors() {
        let shell = FakeShell::new();
        shell.expect("which", &["brew"], "", 1);
        shell.expect("bash", &["-c", BREW_INSTALL], "", 1);
        let r = ensure_brew(&shell).await;
        assert_eq!(r.status, Status::Failed);
        assert!(r.note.contains("brew install script exited"));
    }

    #[tokio::test]
    async fn ensure_linuxbrew_prereqs_skips_on_macos() {
        let shell = FakeShell::new();
        let results = ensure_linuxbrew_prereqs(&shell, &Os::Mac).await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, Status::Skipped);
        // No shell calls — short-circuited
        assert!(shell.calls().is_empty());
    }

    #[tokio::test]
    async fn ensure_linuxbrew_prereqs_returns_one_result_per_pkg_on_linux() {
        let shell = FakeShell::new();
        for pkg in LINUXBREW_APT_PREREQS {
            shell.expect("dpkg", &["-s", pkg], "Status: install ok installed\n", 0);
        }
        let os = Os::Linux {
            distro: "ubuntu".into(),
        };
        let results = ensure_linuxbrew_prereqs(&shell, &os).await;
        assert_eq!(results.len(), LINUXBREW_APT_PREREQS.len());
        for r in &results {
            assert_eq!(r.status, Status::Already);
        }
    }

    #[tokio::test]
    async fn ensure_linuxbrew_prereqs_installs_missing_pkg() {
        let shell = FakeShell::new();
        // first prereq missing → install path; subsequent ones present
        let first = LINUXBREW_APT_PREREQS[0];
        shell.expect_sequence(
            "dpkg",
            &["-s", first],
            &[("", 1), ("Status: install ok installed\n", 0)],
        );
        shell.expect("sudo", &["apt-get", "install", "-y", first], "ok\n", 0);
        for pkg in &LINUXBREW_APT_PREREQS[1..] {
            shell.expect("dpkg", &["-s", pkg], "Status: install ok installed\n", 0);
        }
        let os = Os::Linux {
            distro: "ubuntu".into(),
        };
        let results = ensure_linuxbrew_prereqs(&shell, &os).await;
        assert_eq!(results[0].status, Status::Installed);
        for r in &results[1..] {
            assert_eq!(r.status, Status::Already);
        }
    }

    #[tokio::test]
    async fn ensure_linuxbrew_prereqs_marks_failed_when_apt_fails() {
        let shell = FakeShell::new();
        let first = LINUXBREW_APT_PREREQS[0];
        shell.expect("dpkg", &["-s", first], "", 1);
        shell.expect(
            "sudo",
            &["apt-get", "install", "-y", first],
            "E: locked\n",
            1,
        );
        for pkg in &LINUXBREW_APT_PREREQS[1..] {
            shell.expect("dpkg", &["-s", pkg], "Status: install ok installed\n", 0);
        }
        let os = Os::Linux {
            distro: "ubuntu".into(),
        };
        let results = ensure_linuxbrew_prereqs(&shell, &os).await;
        assert_eq!(results[0].status, Status::Failed);
        assert!(results[0].note.contains("apt-get install failed"));
    }
}
