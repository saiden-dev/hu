use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Default settings.toml content with all options commented out
pub const DEFAULT_SETTINGS: &str = r#"# hu settings
# Location: ~/Library/Application Support/hu/settings.toml (macOS)
#           ~/.config/hu/settings.toml (Linux)

# ============================================================================
# AWS Configuration
# ============================================================================

[aws]
# AWS profile to use (overridden by --aws-profile flag)
# profile = "default"

# AWS region for EKS clusters
# region = "us-east-1"

# ============================================================================
# Kubernetes Configuration
# ============================================================================

[kubernetes]
# Default namespace for pod operations
# namespace = "cms"

# Default pod type/pattern to filter
# pod_type = "web"

# ============================================================================
# Environment Configuration
# ============================================================================

[environments]
# Map environment names to EKS cluster names
# [environments.clusters]
# prod = "prod-eks"
# dev = "eks-dev"
# stg = "eks-stg"

# ============================================================================
# Logging Configuration
# ============================================================================

[logging]
# Default log file path template ({env} is replaced with environment name)
# log_path = "~/.config/hu/{env}.log"

# ============================================================================
# Display Configuration
# ============================================================================

[display]
# Environment emojis
# [display.emojis]
# prod = "🔴"
# dev = "🟢"
# stg = "🟡"
"#;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub aws: AwsSettings,
    #[serde(default)]
    pub kubernetes: KubernetesSettings,
    #[serde(default)]
    pub environments: EnvironmentSettings,
    #[serde(default)]
    pub logging: LoggingSettings,
    #[serde(default)]
    pub display: DisplaySettings,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AwsSettings {
    pub profile: Option<String>,
    pub region: String,
}

impl Default for AwsSettings {
    fn default() -> Self {
        Self {
            profile: None,
            region: "us-east-1".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct KubernetesSettings {
    pub namespace: String,
    pub pod_type: String,
}

impl Default for KubernetesSettings {
    fn default() -> Self {
        Self {
            namespace: "cms".to_string(),
            pod_type: "web".to_string(),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct EnvironmentSettings {
    pub clusters: ClusterMap,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ClusterMap {
    pub prod: String,
    pub dev: String,
    pub stg: String,
}

impl Default for ClusterMap {
    fn default() -> Self {
        Self {
            prod: "prod-eks".to_string(),
            dev: "eks-dev".to_string(),
            stg: "eks-stg".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingSettings {
    pub log_path: String,
}

impl Default for LoggingSettings {
    fn default() -> Self {
        Self {
            log_path: "~/.config/hu/{env}.log".to_string(),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplaySettings {
    pub emojis: EmojiMap,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct EmojiMap {
    pub prod: String,
    pub dev: String,
    pub stg: String,
}

impl Default for EmojiMap {
    fn default() -> Self {
        Self {
            prod: "🔴".to_string(),
            dev: "🟢".to_string(),
            stg: "🟡".to_string(),
        }
    }
}

/// Get the path to the settings file
pub fn settings_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir().context("Could not determine config directory")?;
    Ok(config_dir.join("hu").join("settings.toml"))
}

/// Ensure the settings file exists, creating it with defaults if not
pub fn ensure_settings_file() -> Result<PathBuf> {
    let path = settings_path()?;

    if !path.exists() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {:?}", parent))?;
        }
        std::fs::write(&path, DEFAULT_SETTINGS)
            .with_context(|| format!("Failed to write default settings to {:?}", path))?;
    }

    Ok(path)
}

/// Load settings from the config file
pub fn load_settings() -> Result<Settings> {
    let path = ensure_settings_file()?;
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read settings from {:?}", path))?;

    let settings: Settings =
        toml::from_str(&content).with_context(|| "Failed to parse settings.toml")?;

    Ok(settings)
}
