//! SSH phase — read keys from 1Password and place them on disk.
//!
//! `OpClient` is the second earned trait abstraction (per doctrine §1: ≥2
//! implementers — `RealOp` over the `op` CLI + `MockOp` for tests; painful
//! I/O to fake otherwise — vault auth, account state).
//!
//! Key writing is split into a pure `KeySpec` description (path / mode /
//! content) and the I/O glue that applies it. Tests cover the spec
//! computation; `fs::write` + `chmod` are exercised at run-time.

// reason: ssh phase wired by `hu setup ssh` (chunk 4.2) and `setup run`
// (Phase 5). Tests cover the surface now.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use async_trait::async_trait;

use crate::util::shell::Shell;

/// Standard chmod for SSH private keys.
pub const PRIVATE_KEY_MODE: u32 = 0o600;
/// Standard chmod for SSH public keys.
pub const PUBLIC_KEY_MODE: u32 = 0o644;

/// 1Password CLI client.
#[async_trait]
pub trait OpClient: Send + Sync {
    /// Read a single field via `op read <op-ref>`. Refs are formatted as
    /// `op://<vault>/<item>/<field>`.
    async fn read(&self, op_ref: &str) -> Result<String>;

    /// Whether the host has at least one signed-in 1Password account.
    /// Used to short-circuit the SSH phase with a clear error rather than
    /// emitting cryptic `op` failures per item.
    async fn account_status(&self) -> Result<bool>;
}

/// Real `op` CLI wrapper backed by the [`Shell`] chokepoint.
pub struct RealOp<'s, S: Shell + ?Sized> {
    shell: &'s S,
}

impl<'s, S: Shell + ?Sized> RealOp<'s, S> {
    pub fn new(shell: &'s S) -> Self {
        Self { shell }
    }
}

#[async_trait]
impl<S: Shell + ?Sized> OpClient for RealOp<'_, S> {
    async fn read(&self, op_ref: &str) -> Result<String> {
        let out = self
            .shell
            .run("op", &["read", op_ref])
            .await
            .with_context(|| format!("op read {}", op_ref))?;
        if !out.is_success() {
            anyhow::bail!(
                "op read {} failed (exit {:?}): {}",
                op_ref,
                out.status.code(),
                out.stderr.trim()
            );
        }
        Ok(out.stdout)
    }

    async fn account_status(&self) -> Result<bool> {
        let out = self.shell.run("op", &["account", "list"]).await?;
        Ok(out.is_success() && !out.stdout.trim().is_empty())
    }
}

/// Specification for one SSH key file to write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeySpec {
    pub path: PathBuf,
    pub mode: u32,
    pub content: String,
}

/// Build the (private, public) `KeySpec` pair for one configured op item.
///
/// `op_item_ref` is the vault-relative path like `"SSH/id_ed25519"`. The
/// basename (`id_ed25519`) becomes the key file name in `key_dir`.
pub fn key_specs_for_item(
    key_dir: &Path,
    op_item_ref: &str,
    private_content: String,
    public_content: String,
) -> Vec<KeySpec> {
    let basename = op_item_ref
        .rsplit('/')
        .next()
        .unwrap_or(op_item_ref)
        .to_string();
    vec![
        KeySpec {
            path: key_dir.join(&basename),
            mode: PRIVATE_KEY_MODE,
            content: ensure_trailing_newline(&private_content),
        },
        KeySpec {
            path: key_dir.join(format!("{}.pub", basename)),
            mode: PUBLIC_KEY_MODE,
            content: ensure_trailing_newline(&public_content),
        },
    ]
}

fn ensure_trailing_newline(s: &str) -> String {
    if s.ends_with('\n') {
        s.to_string()
    } else {
        format!("{}\n", s)
    }
}

/// Build the full op `read` reference: `op://<vault>/<item>/<field>`.
pub fn op_ref(vault: &str, item: &str, field: &str) -> String {
    format!("op://{}/{}/{}", vault, item, field)
}

/// Decide how to apply a key spec given the current filesystem state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecAction {
    /// File exists with matching content + correct mode → nothing to do.
    AlreadyMatches,
    /// File missing or content differs — write it (idempotent overwrite).
    WriteFile,
}

/// Inspect the filesystem and decide what action a `KeySpec` needs.
///
/// This is pure-ish — only reads. Tests cover the matrix without mocking.
pub fn classify_spec(spec: &KeySpec) -> SpecAction {
    if !spec.path.exists() {
        return SpecAction::WriteFile;
    }
    let existing = match std::fs::read_to_string(&spec.path) {
        Ok(s) => s,
        Err(_) => return SpecAction::WriteFile,
    };
    if existing != spec.content {
        return SpecAction::WriteFile;
    }
    if let Ok(meta) = std::fs::metadata(&spec.path) {
        use std::os::unix::fs::PermissionsExt;
        if meta.permissions().mode() & 0o777 != spec.mode {
            return SpecAction::WriteFile;
        }
    }
    SpecAction::AlreadyMatches
}

/// Apply one key spec to disk. Idempotent: returns `Already` when the file
/// already matches; writes + chmods otherwise; re-reads to verify mode.
pub fn apply_spec(spec: &KeySpec) -> Result<crate::setup::types::Status> {
    use std::os::unix::fs::PermissionsExt;
    if classify_spec(spec) == SpecAction::AlreadyMatches {
        return Ok(crate::setup::types::Status::Already);
    }
    if let Some(parent) = spec.path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create dir {}", parent.display()))?;
    }
    std::fs::write(&spec.path, &spec.content)
        .with_context(|| format!("write {}", spec.path.display()))?;
    let mut perms = std::fs::metadata(&spec.path)
        .with_context(|| format!("stat {}", spec.path.display()))?
        .permissions();
    perms.set_mode(spec.mode);
    std::fs::set_permissions(&spec.path, perms)
        .with_context(|| format!("chmod {}", spec.path.display()))?;
    // Re-verify mode landed.
    let final_mode = std::fs::metadata(&spec.path)?.permissions().mode() & 0o777;
    if final_mode != spec.mode {
        anyhow::bail!(
            "post-write mode mismatch on {}: got {:o}, want {:o}",
            spec.path.display(),
            final_mode,
            spec.mode
        );
    }
    Ok(crate::setup::types::Status::Installed)
}

/// Fetch the private + public key pair for one item.
pub async fn fetch_key_pair<O: OpClient + ?Sized>(
    op: &O,
    vault: &str,
    item: &str,
) -> Result<(String, String)> {
    let private = op
        .read(&op_ref(vault, item, "private_key"))
        .await
        .with_context(|| format!("read private key for {}/{}", vault, item))?;
    let public = op
        .read(&op_ref(vault, item, "public_key"))
        .await
        .with_context(|| format!("read public key for {}/{}", vault, item))?;
    Ok((private, public))
}

/// Orchestrate the full SSH phase: account check → for each item fetch
/// (private, public) → write specs.
pub async fn run<O: OpClient + ?Sized>(
    op: &O,
    config: &crate::setup::config::SshConfig,
) -> Vec<crate::setup::display::StatusRow> {
    use crate::setup::display::StatusRow;
    use crate::setup::types::Status;

    let mut rows = Vec::new();

    let signed_in = match op.account_status().await {
        Ok(b) => b,
        Err(e) => {
            rows.push(
                StatusRow::new("ssh", "op-account", Status::Failed)
                    .with_note(&format!("account_status errored: {}", e)),
            );
            return rows;
        }
    };
    if !signed_in {
        rows.push(
            StatusRow::new("ssh", "op-account", Status::Failed)
                .with_note("no signed-in 1Password account — run `op account add`"),
        );
        return rows;
    }
    rows.push(StatusRow::new("ssh", "op-account", Status::Already).with_note("signed in"));

    let key_dir = expand_tilde(&config.key_dir);
    for item in &config.op_items {
        match fetch_key_pair(op, &config.op_vault, item).await {
            Err(e) => {
                rows.push(
                    StatusRow::new("ssh", item, Status::Failed)
                        .with_note(&format!("fetch failed: {}", e)),
                );
                continue;
            }
            Ok((priv_k, pub_k)) => {
                let specs = key_specs_for_item(&key_dir, item, priv_k, pub_k);
                for spec in specs {
                    let basename = spec
                        .path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("?")
                        .to_string();
                    match apply_spec(&spec) {
                        Ok(status) => rows.push(
                            StatusRow::new("ssh", &basename, status)
                                .with_note(&format!("mode {:o}", spec.mode)),
                        ),
                        Err(e) => rows.push(
                            StatusRow::new("ssh", &basename, Status::Failed)
                                .with_note(&format!("apply failed: {}", e)),
                        ),
                    }
                }
            }
        }
    }
    rows
}

fn expand_tilde(raw: &str) -> std::path::PathBuf {
    crate::setup::dotfiles::expand_tilde(raw)
}

#[cfg(test)]
mod fake {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Scripted op client for tests. Maps op refs → scripted output.
    pub struct MockOp {
        responses: Mutex<HashMap<String, String>>,
        signed_in: Mutex<bool>,
        reads: Mutex<Vec<String>>,
    }

    impl MockOp {
        pub fn new() -> Self {
            Self {
                responses: Mutex::new(HashMap::new()),
                signed_in: Mutex::new(true),
                reads: Mutex::new(Vec::new()),
            }
        }

        pub fn expect(&self, op_ref: &str, content: &str) {
            self.responses
                .lock()
                .expect("mock op mutex")
                .insert(op_ref.to_string(), content.to_string());
        }

        pub fn set_signed_in(&self, ok: bool) {
            *self.signed_in.lock().expect("mock op mutex") = ok;
        }

        pub fn reads(&self) -> Vec<String> {
            self.reads.lock().expect("mock op mutex").clone()
        }
    }

    #[async_trait]
    impl OpClient for MockOp {
        async fn read(&self, op_ref: &str) -> Result<String> {
            self.reads
                .lock()
                .expect("mock op mutex")
                .push(op_ref.to_string());
            let map = self.responses.lock().expect("mock op mutex");
            match map.get(op_ref) {
                Some(content) => Ok(content.clone()),
                None => anyhow::bail!("MockOp: no response registered for {}", op_ref),
            }
        }

        async fn account_status(&self) -> Result<bool> {
            Ok(*self.signed_in.lock().expect("mock op mutex"))
        }
    }
}

#[cfg(test)]
pub use fake::MockOp;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn op_ref_formats_correctly() {
        assert_eq!(
            op_ref("Personal", "SSH/id_ed25519", "private_key"),
            "op://Personal/SSH/id_ed25519/private_key"
        );
    }

    #[test]
    fn ensure_trailing_newline_appends_when_missing() {
        assert_eq!(ensure_trailing_newline("abc"), "abc\n");
        assert_eq!(ensure_trailing_newline("abc\n"), "abc\n");
        assert_eq!(ensure_trailing_newline(""), "\n");
    }

    #[test]
    fn key_specs_extract_basename_from_path() {
        let specs = key_specs_for_item(
            Path::new("/home/u/.ssh"),
            "SSH/id_ed25519",
            "PRIV".into(),
            "PUB".into(),
        );
        assert_eq!(specs.len(), 2);
        assert_eq!(specs[0].path, PathBuf::from("/home/u/.ssh/id_ed25519"));
        assert_eq!(specs[1].path, PathBuf::from("/home/u/.ssh/id_ed25519.pub"));
    }

    #[test]
    fn key_specs_apply_correct_modes() {
        let specs = key_specs_for_item(
            Path::new("/home/u/.ssh"),
            "id_rsa",
            "priv".into(),
            "pub".into(),
        );
        assert_eq!(specs[0].mode, 0o600);
        assert_eq!(specs[1].mode, 0o644);
    }

    #[test]
    fn key_specs_normalize_trailing_newline() {
        let specs = key_specs_for_item(Path::new("/x"), "key", "PRIV-NO-NL".into(), "PUB\n".into());
        assert_eq!(specs[0].content, "PRIV-NO-NL\n");
        assert_eq!(specs[1].content, "PUB\n");
    }

    #[tokio::test]
    async fn mock_op_returns_scripted_content() {
        let op = MockOp::new();
        op.expect("op://V/I/private_key", "PRIVATE-CONTENT");
        let v = op.read("op://V/I/private_key").await.unwrap();
        assert_eq!(v, "PRIVATE-CONTENT");
    }

    #[tokio::test]
    async fn mock_op_errors_on_unscripted_read() {
        let op = MockOp::new();
        let err = op.read("op://X/Y/z").await.unwrap_err();
        assert!(err.to_string().contains("no response registered"));
    }

    #[tokio::test]
    async fn mock_op_account_status_default_true() {
        let op = MockOp::new();
        assert!(op.account_status().await.unwrap());
        op.set_signed_in(false);
        assert!(!op.account_status().await.unwrap());
    }

    #[tokio::test]
    async fn fetch_key_pair_reads_both_fields() {
        let op = MockOp::new();
        op.expect("op://Personal/SSH/id_ed25519/private_key", "PRIV");
        op.expect("op://Personal/SSH/id_ed25519/public_key", "PUB");
        let (priv_k, pub_k) = fetch_key_pair(&op, "Personal", "SSH/id_ed25519")
            .await
            .unwrap();
        assert_eq!(priv_k, "PRIV");
        assert_eq!(pub_k, "PUB");
    }

    #[tokio::test]
    async fn fetch_key_pair_propagates_op_error() {
        let op = MockOp::new();
        // private_key registered but public_key missing
        op.expect("op://V/I/private_key", "PRIV");
        let err = fetch_key_pair(&op, "V", "I").await.unwrap_err();
        assert!(err.to_string().contains("public key"));
    }

    #[tokio::test]
    async fn real_op_passes_ref_through_shell() {
        use crate::util::shell::FakeShell;
        let shell = FakeShell::new();
        shell.expect("op", &["read", "op://V/I/f"], "secret\n", 0);
        let op = RealOp::new(&shell);
        let v = op.read("op://V/I/f").await.unwrap();
        assert_eq!(v, "secret\n");
    }

    #[tokio::test]
    async fn real_op_errors_on_nonzero_exit() {
        use crate::util::shell::FakeShell;
        let shell = FakeShell::new();
        shell.expect("op", &["read", "op://V/I/f"], "", 1);
        let op = RealOp::new(&shell);
        let err = op.read("op://V/I/f").await.unwrap_err();
        assert!(err.to_string().contains("op read"));
    }

    #[test]
    fn classify_spec_returns_write_when_missing() {
        let spec = KeySpec {
            path: PathBuf::from("/nonexistent/path/file"),
            mode: 0o600,
            content: "x".into(),
        };
        assert_eq!(classify_spec(&spec), SpecAction::WriteFile);
    }

    #[test]
    fn classify_spec_returns_already_when_match() {
        let dir = std::env::temp_dir().join("hu-classify-already");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("k");
        std::fs::write(&path, "abc\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(&path, perms).unwrap();
        let spec = KeySpec {
            path: path.clone(),
            mode: 0o600,
            content: "abc\n".into(),
        };
        assert_eq!(classify_spec(&spec), SpecAction::AlreadyMatches);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn classify_spec_returns_write_when_content_differs() {
        let dir = std::env::temp_dir().join("hu-classify-content-diff");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("k");
        std::fs::write(&path, "old\n").unwrap();
        let spec = KeySpec {
            path: path.clone(),
            mode: 0o600,
            content: "new\n".into(),
        };
        assert_eq!(classify_spec(&spec), SpecAction::WriteFile);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn classify_spec_returns_write_when_mode_differs() {
        let dir = std::env::temp_dir().join("hu-classify-mode-diff");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("k");
        std::fs::write(&path, "abc\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o644); // different from spec
        std::fs::set_permissions(&path, perms).unwrap();
        let spec = KeySpec {
            path: path.clone(),
            mode: 0o600,
            content: "abc\n".into(),
        };
        assert_eq!(classify_spec(&spec), SpecAction::WriteFile);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn apply_spec_writes_file_with_correct_mode() {
        let dir = std::env::temp_dir().join("hu-apply-spec-write");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("id_test");
        let spec = KeySpec {
            path: path.clone(),
            mode: 0o600,
            content: "PRIVATE\n".into(),
        };
        let status = apply_spec(&spec).unwrap();
        assert_eq!(status, crate::setup::types::Status::Installed);
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "PRIVATE\n");
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn apply_spec_skips_when_already_matches() {
        let dir = std::env::temp_dir().join("hu-apply-spec-already");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("id_test");
        std::fs::write(&path, "abc\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(&path, perms).unwrap();
        let spec = KeySpec {
            path: path.clone(),
            mode: 0o600,
            content: "abc\n".into(),
        };
        let status = apply_spec(&spec).unwrap();
        assert_eq!(status, crate::setup::types::Status::Already);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[tokio::test]
    async fn ssh_run_fails_when_not_signed_in() {
        let op = MockOp::new();
        op.set_signed_in(false);
        let cfg = crate::setup::config::SshConfig::default();
        let rows = run(&op, &cfg).await;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].status, crate::setup::types::Status::Failed);
        assert!(rows[0].note.contains("op account add"));
    }

    #[tokio::test]
    async fn ssh_run_writes_keys_when_signed_in() {
        let dir = std::env::temp_dir().join("hu-ssh-run-keys");
        let _ = std::fs::remove_dir_all(&dir);
        let op = MockOp::new();
        op.expect("op://Personal/SSH/id_test/private_key", "PRIV-CONTENT");
        op.expect("op://Personal/SSH/id_test/public_key", "PUB-CONTENT");
        let cfg = crate::setup::config::SshConfig {
            op_vault: "Personal".into(),
            op_items: vec!["SSH/id_test".into()],
            key_dir: dir.to_string_lossy().into_owned(),
        };
        let rows = run(&op, &cfg).await;
        // 1 op-account + 2 key files
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].name, "op-account");
        assert_eq!(rows[1].name, "id_test");
        assert_eq!(rows[2].name, "id_test.pub");
        assert_eq!(rows[1].status, crate::setup::types::Status::Installed);
        assert_eq!(rows[2].status, crate::setup::types::Status::Installed);
        // Verify perms landed
        use std::os::unix::fs::PermissionsExt;
        let priv_mode = std::fs::metadata(dir.join("id_test"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(priv_mode, 0o600);
        let pub_mode = std::fs::metadata(dir.join("id_test.pub"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(pub_mode, 0o644);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[tokio::test]
    async fn ssh_run_marks_item_failed_when_op_read_errors() {
        let dir = std::env::temp_dir().join("hu-ssh-run-op-fail");
        let _ = std::fs::remove_dir_all(&dir);
        let op = MockOp::new();
        // private_key missing → fetch fails
        let cfg = crate::setup::config::SshConfig {
            op_vault: "V".into(),
            op_items: vec!["I".into()],
            key_dir: dir.to_string_lossy().into_owned(),
        };
        let rows = run(&op, &cfg).await;
        assert_eq!(rows.len(), 2); // op-account + 1 item failure
        let item_row = &rows[1];
        assert_eq!(item_row.status, crate::setup::types::Status::Failed);
        assert!(item_row.note.contains("fetch failed"));
    }

    #[tokio::test]
    async fn real_op_account_status_uses_account_list() {
        use crate::util::shell::FakeShell;
        let shell = FakeShell::new();
        shell.expect(
            "op",
            &["account", "list"],
            "URL    EMAIL    USER ID\nmy.1password.com  me@x.com  ABC\n",
            0,
        );
        let op = RealOp::new(&shell);
        assert!(op.account_status().await.unwrap());
    }
}
