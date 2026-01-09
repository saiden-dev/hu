//! CLI integration tests

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

#[test]
fn test_help_flag() {
    cargo_bin_cmd!("eks-shell")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "CLI tool for connecting to Kubernetes pods",
        ));
}

#[test]
fn test_version_flag() {
    cargo_bin_cmd!("eks-shell")
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("eks-shell"));
}

#[test]
fn test_invalid_environment() {
    cargo_bin_cmd!("eks-shell")
        .args(["--env", "invalid"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn test_valid_environments_accepted() {
    // These will fail at runtime (no AWS/kubectl) but should parse correctly
    for env in ["prod", "dev", "stg"] {
        let result = cargo_bin_cmd!("eks-shell")
            .args(["--env", env, "--help"])
            .assert();
        // --help should succeed regardless of env value
        result.success();
    }
}
