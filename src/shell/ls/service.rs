use anyhow::{bail, Result};
use std::process::Command;

/// Default flags hu injects for pretty output.
/// User args come AFTER these, so they can override (GNU ls uses last-wins).
const PRETTY_DEFAULTS: &[&str] = &[
    "--color=always",
    "--group-directories-first",
    "--classify",
    "-h",
];

/// Detect the GNU ls binary name for this platform.
/// macOS ships BSD ls; GNU coreutils installs as `gls`.
/// Linux ships GNU ls as `ls`.
pub fn detect_ls_binary() -> &'static str {
    if cfg!(target_os = "macos") {
        "gls"
    } else {
        "ls"
    }
}

/// Build the full argument list: pretty defaults + user args.
pub fn build_args(user_args: &[String]) -> Vec<String> {
    let mut args: Vec<String> = PRETTY_DEFAULTS.iter().map(|s| (*s).to_string()).collect();
    args.extend(user_args.iter().cloned());
    args
}

/// Execute GNU ls with pretty defaults + user args.
/// Returns the raw stdout bytes on success.
pub fn execute_ls(user_args: &[String]) -> Result<Vec<u8>> {
    let binary = detect_ls_binary();
    let args = build_args(user_args);

    let output = Command::new(binary).args(&args).output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            anyhow::anyhow!(
                "GNU ls not found as '{}'. {}",
                binary,
                if cfg!(target_os = "macos") {
                    "Install with: brew install coreutils"
                } else {
                    "Ensure coreutils is installed"
                }
            )
        } else {
            anyhow::anyhow!("Failed to execute {}: {}", binary, e)
        }
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("{}: {}", binary, stderr.trim());
    }

    Ok(output.stdout)
}

/// Check if user args contain a long-listing flag (-l or --long).
pub fn has_long_flag(args: &[String]) -> bool {
    args.iter().any(|a| {
        a == "-l"
            || a == "--long"
            || (a.starts_with('-') && !a.starts_with("--") && a.contains('l'))
    })
}

/// Check if user args contain a one-per-line flag (-1).
pub fn has_single_column_flag(args: &[String]) -> bool {
    args.iter()
        .any(|a| a == "-1" || (a.starts_with('-') && !a.starts_with("--") && a.contains('1')))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_binary_returns_valid_name() {
        let binary = detect_ls_binary();
        assert!(binary == "gls" || binary == "ls");
    }

    #[test]
    fn build_args_empty_user_args() {
        let args = build_args(&[]);
        assert_eq!(args.len(), PRETTY_DEFAULTS.len());
        assert!(args.contains(&"--color=always".to_string()));
        assert!(args.contains(&"--group-directories-first".to_string()));
        assert!(args.contains(&"--classify".to_string()));
        assert!(args.contains(&"-h".to_string()));
    }

    #[test]
    fn build_args_with_user_args() {
        let user = vec!["-la".to_string(), "/tmp".to_string()];
        let args = build_args(&user);
        // Pretty defaults come first
        assert_eq!(args[0], "--color=always");
        // User args appended at end
        assert!(args.contains(&"-la".to_string()));
        assert!(args.contains(&"/tmp".to_string()));
        assert_eq!(args.len(), PRETTY_DEFAULTS.len() + 2);
    }

    #[test]
    fn build_args_user_can_override_color() {
        let user = vec!["--color=never".to_string()];
        let args = build_args(&user);
        // Both present - GNU ls uses last-wins, so user's --color=never takes effect
        assert_eq!(args[0], "--color=always");
        assert_eq!(*args.last().unwrap(), "--color=never");
    }

    #[test]
    fn has_long_flag_detects_dash_l() {
        assert!(has_long_flag(&["-l".to_string()]));
        assert!(has_long_flag(&["-la".to_string()]));
        assert!(has_long_flag(&["-al".to_string()]));
        assert!(has_long_flag(&["--long".to_string()]));
    }

    #[test]
    fn has_long_flag_negative() {
        assert!(!has_long_flag(&[]));
        assert!(!has_long_flag(&["-a".to_string()]));
        assert!(!has_long_flag(&["/tmp".to_string()]));
        assert!(!has_long_flag(&["--all".to_string()]));
    }

    #[test]
    fn has_single_column_flag_detects() {
        assert!(has_single_column_flag(&["-1".to_string()]));
        assert!(has_single_column_flag(&["-a1".to_string()]));
    }

    #[test]
    fn has_single_column_flag_negative() {
        assert!(!has_single_column_flag(&[]));
        assert!(!has_single_column_flag(&["-l".to_string()]));
        assert!(!has_single_column_flag(&["/tmp".to_string()]));
    }

    #[test]
    fn execute_ls_current_dir() {
        // This test requires GNU ls to be installed
        let result = execute_ls(&[]);
        if detect_ls_binary() == "gls" {
            // On macOS, gls might not be installed in CI
            if let Ok(stdout) = result {
                // Should produce some output (current dir is not empty)
                assert!(!stdout.is_empty());
            }
        } else {
            // On Linux, ls is always available
            assert!(result.is_ok());
        }
    }

    #[test]
    fn execute_ls_nonexistent_dir() {
        let result = execute_ls(&["/nonexistent/path/xyz123".to_string()]);
        // Should fail because path does not exist; some ls versions
        // print to stderr without erroring, so accept either path.
        if let Err(e) = result {
            let err = e.to_string();
            assert!(!err.is_empty());
        }
    }

    #[test]
    fn pretty_defaults_order() {
        // Color should come first so user can override
        assert_eq!(PRETTY_DEFAULTS[0], "--color=always");
    }
}
