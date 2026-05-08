//! Config types and TOML loader for `hu setup`.
//!
//! Path resolution follows the existing `hu` convention via
//! `directories::ProjectDirs::from("", "", "hu")`:
//!
//! - macOS: `~/Library/Application Support/hu/setup.toml`
//! - Linux: `~/.config/hu/setup.toml`
//!
//! Per project doctrine §1 — TOML serialize/deserialize is unit-tested via
//! round-trip; `fs::write` and `fs::read_to_string` are I/O glue and use
//! `#[coverage(off)]` carve-outs.

// reason: config types wired to `hu setup config init/path` (this chunk) and
// `hu setup status / pkgs / dotfiles / ssh` (Phases 1–4). Suppress dead_code
// until each consumer lands; serialize / deserialize is verified by tests now.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

const CONFIG_FILENAME: &str = "setup.toml";

/// Top-level setup config.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetupConfig {
    #[serde(default)]
    pub dotfiles: DotfilesConfig,
    #[serde(default)]
    pub ssh: SshConfig,
    #[serde(default)]
    pub packages: PackagesConfig,
    #[serde(default)]
    pub host: BTreeMap<String, HostOverride>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DotfilesConfig {
    pub repo: String,
    pub branch: String,
    pub clone_to: String,
    pub strategy: String,
    pub packages: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SshConfig {
    pub op_vault: String,
    pub op_items: Vec<String>,
    pub key_dir: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackagesConfig {
    pub brew: Vec<String>,
    pub mise: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HostOverride {
    #[serde(default)]
    pub brew_extra: Vec<String>,
    #[serde(default)]
    pub mise_extra: Vec<String>,
}

impl Default for DotfilesConfig {
    fn default() -> Self {
        Self {
            repo: "aladac/dotfiles".into(),
            branch: "main".into(),
            clone_to: "~/Projects/dotfiles".into(),
            strategy: "stow".into(),
            packages: vec![
                "zsh".into(),
                "git".into(),
                "kitty".into(),
                "starship".into(),
                "zellij".into(),
            ],
        }
    }
}

impl Default for SshConfig {
    fn default() -> Self {
        Self {
            op_vault: "Personal".into(),
            op_items: vec!["SSH/id_ed25519".into()],
            key_dir: "~/.ssh".into(),
        }
    }
}

impl Default for PackagesConfig {
    fn default() -> Self {
        Self {
            brew: vec![
                "mise".into(),
                "uv".into(),
                "hf".into(),
                "op".into(),
                "gh".into(),
                "jq".into(),
                "stow".into(),
                "starship".into(),
                "zellij".into(),
                "kitty".into(),
                "kitten".into(),
                "flarectl".into(),
                "cloudflared".into(),
                "hcloud".into(),
                "postgresql".into(),
            ],
            mise: vec![
                "node@lts".into(),
                "ruby@latest".into(),
                "python@latest".into(),
                "rust@latest".into(),
            ],
        }
    }
}

/// Serialize config to a TOML string. Pure function — testable without I/O.
pub fn serialize(config: &SetupConfig) -> Result<String> {
    toml::to_string_pretty(config).context("serialize SetupConfig to TOML")
}

/// Deserialize config from a TOML string. Pure function — testable without I/O.
pub fn deserialize(raw: &str) -> Result<SetupConfig> {
    toml::from_str(raw).context("parse setup.toml")
}

/// Resolve the config path via `ProjectDirs`. Returns `None` if no usable
/// config directory exists for this platform.
pub fn config_path() -> Option<PathBuf> {
    ProjectDirs::from("", "", "hu").map(|dirs| dirs.config_dir().join(CONFIG_FILENAME))
}

/// Read the current setup.toml. Returns the default config when the file is
/// absent (so `hu setup status` works on a fresh host before `config init`).
pub fn load() -> Result<SetupConfig> {
    let Some(path) = config_path() else {
        return Ok(SetupConfig::default());
    };
    if !path.exists() {
        return Ok(SetupConfig::default());
    }
    let raw = std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    deserialize(&raw)
}

/// Write the default config to disk if absent. Idempotent — returns the path
/// either way and an `existed` flag indicating whether a file already lived
/// there.
pub fn init_default() -> Result<InitOutcome> {
    let path = config_path().context("could not resolve config directory for hu")?;
    if path.exists() {
        return Ok(InitOutcome {
            path,
            existed: true,
        });
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create config dir {}", parent.display()))?;
    }
    let raw = serialize(&SetupConfig::default())?;
    std::fs::write(&path, raw).with_context(|| format!("write {}", path.display()))?;
    Ok(InitOutcome {
        path,
        existed: false,
    })
}

#[derive(Debug, Clone)]
pub struct InitOutcome {
    pub path: PathBuf,
    pub existed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_dotfiles_repo_is_aladac() {
        let cfg = SetupConfig::default();
        assert_eq!(cfg.dotfiles.repo, "aladac/dotfiles");
        assert_eq!(cfg.dotfiles.strategy, "stow");
    }

    #[test]
    fn default_brew_includes_mise_and_stow() {
        let cfg = SetupConfig::default();
        assert!(cfg.packages.brew.contains(&"mise".to_string()));
        assert!(cfg.packages.brew.contains(&"stow".to_string()));
        assert!(cfg.packages.brew.contains(&"op".to_string()));
        assert!(cfg.packages.brew.contains(&"gh".to_string()));
    }

    #[test]
    fn default_mise_includes_all_t2() {
        let cfg = SetupConfig::default();
        assert_eq!(cfg.packages.mise.len(), 4);
        assert!(cfg.packages.mise.iter().any(|p| p.starts_with("node@")));
        assert!(cfg.packages.mise.iter().any(|p| p.starts_with("ruby@")));
        assert!(cfg.packages.mise.iter().any(|p| p.starts_with("python@")));
        assert!(cfg.packages.mise.iter().any(|p| p.starts_with("rust@")));
    }

    #[test]
    fn default_ssh_uses_op_with_personal_vault() {
        let cfg = SetupConfig::default();
        assert_eq!(cfg.ssh.op_vault, "Personal");
        assert_eq!(cfg.ssh.key_dir, "~/.ssh");
        assert_eq!(cfg.ssh.op_items, vec!["SSH/id_ed25519"]);
    }

    #[test]
    fn round_trips_default_config() {
        let cfg = SetupConfig::default();
        let raw = serialize(&cfg).unwrap();
        let parsed = deserialize(&raw).unwrap();
        assert_eq!(parsed, cfg);
    }

    #[test]
    fn round_trips_with_host_override() {
        let mut cfg = SetupConfig::default();
        cfg.host.insert(
            "marauder".to_string(),
            HostOverride {
                brew_extra: vec!["nvtop".into()],
                mise_extra: vec![],
            },
        );
        let raw = serialize(&cfg).unwrap();
        let parsed = deserialize(&raw).unwrap();
        assert_eq!(parsed, cfg);
        assert_eq!(
            parsed.host.get("marauder").unwrap().brew_extra,
            vec!["nvtop"]
        );
    }

    #[test]
    fn deserialize_accepts_minimal_toml() {
        let raw = r#"
[dotfiles]
repo = "x/y"
branch = "main"
clone_to = "~/dot"
strategy = "stow"
packages = ["zsh"]

[ssh]
op_vault = "V"
op_items = ["a"]
key_dir = "~/.ssh"

[packages]
brew = ["gh"]
mise = ["node@lts"]
"#;
        let cfg = deserialize(raw).unwrap();
        assert_eq!(cfg.dotfiles.repo, "x/y");
        assert_eq!(cfg.packages.brew, vec!["gh"]);
        assert!(cfg.host.is_empty());
    }

    #[test]
    fn deserialize_rejects_invalid_toml() {
        let err = deserialize("not = valid = toml").unwrap_err();
        assert!(err.to_string().contains("setup.toml"));
    }

    #[test]
    fn config_path_returns_setup_toml() {
        let path = config_path().expect("ProjectDirs should resolve on test platform");
        assert_eq!(path.file_name().unwrap().to_str().unwrap(), CONFIG_FILENAME);
    }

    #[test]
    fn host_override_default_is_empty() {
        let h = HostOverride::default();
        assert!(h.brew_extra.is_empty());
        assert!(h.mise_extra.is_empty());
    }
}
