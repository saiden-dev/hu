//! Service layer for `hu setup status`.
//!
//! Collects status rows for every configured package by checking presence
//! via the [`Shell`] chokepoint. Pure logic — interface formats the result.

// reason: collector wired to `hu setup status` (this chunk) and reused by
// `setup preview` and (in Phase 1+) the per-package install pipeline.
#![allow(dead_code)]

use anyhow::Result;

use crate::setup::config::SetupConfig;
use crate::setup::display::StatusRow;
use crate::setup::types::Status;
use crate::util::shell::Shell;

/// Map a configured package id to the binary name `which` should look for.
///
/// For most packages the id matches the binary. Known mismatches:
/// - `postgresql` ships `psql` as the user-facing binary
/// - mise-managed `<lang>@<version>` strips the version qualifier
pub fn binary_name(pkg: &str) -> &str {
    if let Some((lang, _version)) = pkg.split_once('@') {
        return mise_lang_to_binary(lang);
    }
    match pkg {
        "postgresql" => "psql",
        "kitty" => "kitty",
        other => other,
    }
}

fn mise_lang_to_binary(lang: &str) -> &str {
    match lang {
        "node" => "node",
        "ruby" => "ruby",
        "python" => "python3",
        "rust" => "rustc",
        other => other,
    }
}

/// Collect status rows for every configured package + key host artifact.
///
/// Performs a `which <binary>` per package via the `Shell` chokepoint. No
/// side effects beyond the `which` calls themselves.
pub async fn collect(shell: &impl Shell, config: &SetupConfig) -> Result<Vec<StatusRow>> {
    let mut rows = Vec::new();
    for pkg in &config.packages.brew {
        rows.push(check_binary(shell, "brew", pkg).await);
    }
    for pkg in &config.packages.mise {
        rows.push(check_binary(shell, "mise", pkg).await);
    }
    Ok(rows)
}

async fn check_binary(shell: &impl Shell, category: &str, pkg: &str) -> StatusRow {
    let bin = binary_name(pkg);
    let status = if shell.which(bin).await {
        Status::Already
    } else {
        Status::Failed
    };
    StatusRow::new(category, pkg, status).with_note(
        if bin == pkg {
            String::new()
        } else {
            format!("binary: {}", bin)
        }
        .as_str(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup::config::SetupConfig;
    use crate::util::shell::FakeShell;

    #[test]
    fn binary_name_strips_version_for_mise_packages() {
        assert_eq!(binary_name("node@lts"), "node");
        assert_eq!(binary_name("ruby@latest"), "ruby");
        assert_eq!(binary_name("python@latest"), "python3");
        assert_eq!(binary_name("rust@latest"), "rustc");
    }

    #[test]
    fn binary_name_remaps_postgresql_to_psql() {
        assert_eq!(binary_name("postgresql"), "psql");
    }

    #[test]
    fn binary_name_passes_through_unknown() {
        assert_eq!(binary_name("gh"), "gh");
        assert_eq!(binary_name("zellij"), "zellij");
    }

    #[tokio::test]
    async fn collect_marks_present_packages_as_already() {
        let shell = FakeShell::new();
        shell.expect("which", &["gh"], "/opt/homebrew/bin/gh\n", 0);
        let mut config = SetupConfig::default();
        config.packages.brew = vec!["gh".into()];
        config.packages.mise = vec![];
        let rows = collect(&shell, &config).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].status, Status::Already);
        assert_eq!(rows[0].name, "gh");
        assert_eq!(rows[0].category, "brew");
    }

    #[tokio::test]
    async fn collect_marks_missing_packages_as_failed() {
        let shell = FakeShell::new();
        // no expect → unscripted → exit 127 → which returns false
        let mut config = SetupConfig::default();
        config.packages.brew = vec!["nonexistent".into()];
        config.packages.mise = vec![];
        let rows = collect(&shell, &config).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].status, Status::Failed);
    }

    #[tokio::test]
    async fn collect_uses_psql_binary_for_postgresql() {
        let shell = FakeShell::new();
        shell.expect("which", &["psql"], "/opt/homebrew/bin/psql\n", 0);
        let mut config = SetupConfig::default();
        config.packages.brew = vec!["postgresql".into()];
        config.packages.mise = vec![];
        let rows = collect(&shell, &config).await.unwrap();
        assert_eq!(rows[0].status, Status::Already);
        assert!(rows[0].note.contains("psql"));
    }

    #[tokio::test]
    async fn collect_handles_mise_versioned_packages() {
        let shell = FakeShell::new();
        shell.expect("which", &["node"], "/usr/local/bin/node\n", 0);
        let mut config = SetupConfig::default();
        config.packages.brew = vec![];
        config.packages.mise = vec!["node@lts".into()];
        let rows = collect(&shell, &config).await.unwrap();
        assert_eq!(rows[0].name, "node@lts");
        assert_eq!(rows[0].category, "mise");
        assert_eq!(rows[0].status, Status::Already);
    }

    #[tokio::test]
    async fn collect_returns_one_row_per_configured_package() {
        let shell = FakeShell::new();
        shell.expect("which", &["gh"], "/usr/bin/gh\n", 0);
        shell.expect("which", &["node"], "/usr/bin/node\n", 0);
        let mut config = SetupConfig::default();
        config.packages.brew = vec!["gh".into(), "missing".into()];
        config.packages.mise = vec!["node@lts".into()];
        let rows = collect(&shell, &config).await.unwrap();
        assert_eq!(rows.len(), 3);
    }
}
