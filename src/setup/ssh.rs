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
