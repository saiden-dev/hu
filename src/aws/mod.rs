//! AWS operations module
//!
//! This module is split into submodules for better organization:
//! - `identity`: AWS identity and whoami operations
//! - `discovery`: AWS profile discovery and capability checking
//! - `ec2`: EC2 instance listing, display, and SSM connection
//! - `spawn`: EC2 instance spawning and cleanup operations

mod discovery;
mod ec2;
mod identity;
mod spawn;

// Re-export commonly used items
pub use discovery::discover;
pub use ec2::{display_instances, list_instances, ssm_connect, Ec2Filter};
pub use identity::whoami;
pub use spawn::{display_spawned_instance, kill_instance, spawn_instance, SpawnConfig};

use anyhow::{bail, Context, Result};
use std::process::Command;

/// Get AWS SDK configuration with optional profile and region
pub async fn get_config(profile: Option<&str>, region: &str) -> aws_config::SdkConfig {
    let mut builder = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new(region.to_string()));

    if let Some(profile_name) = profile {
        builder = builder.profile_name(profile_name);
    }

    builder.load().await
}

/// Check if the current AWS session is valid
pub async fn check_session(config: &aws_config::SdkConfig) -> bool {
    let client = aws_sdk_sts::Client::new(config);
    client.get_caller_identity().send().await.is_ok()
}

/// Perform SSO login for the given profile
pub fn sso_login(profile: Option<&str>) -> Result<()> {
    let mut cmd = Command::new("aws");
    cmd.args(["sso", "login"]);

    if let Some(profile_name) = profile {
        cmd.args(["--profile", profile_name]);
    }

    let status = cmd.status().context("Failed to run aws sso login")?;

    if status.success() {
        Ok(())
    } else {
        bail!("AWS SSO login failed")
    }
}
