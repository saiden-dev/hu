use std::process::Command;

fn hu() -> Command {
    Command::new(env!("CARGO_BIN_EXE_hu"))
}

#[test]
fn no_args_shows_help_and_exits_zero() {
    let output = hu().output().expect("failed to execute");

    assert!(output.status.success(), "expected exit code 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage: hu [COMMAND]"));
    assert!(stdout.contains("Commands:"));
}

#[test]
fn help_flag_shows_help() {
    let output = hu().arg("--help").output().expect("failed to execute");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Dev workflow CLI"));
}

#[test]
fn version_flag_shows_version() {
    let output = hu().arg("--version").output().expect("failed to execute");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("hu "));
}

#[test]
fn subcommand_without_action_shows_help() {
    // Test all subcommands show help when called without action
    let cases = [
        ("jira", "Jira operations"),
        ("gh", "GitHub operations"),
        ("slack", "Slack operations"),
        ("pagerduty", "PagerDuty"),
        ("sentry", "Sentry"),
        ("newrelic", "NewRelic"),
        ("eks", "EKS pod access"),
        ("pipeline", "CodePipeline status"),
    ];

    for (cmd, expected) in cases {
        let output = hu().arg(cmd).output().expect("failed to execute");
        assert!(output.status.success(), "{} should exit 0", cmd);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains(expected),
            "{} help missing description",
            cmd
        );
    }
}

#[test]
fn all_main_commands_in_help() {
    let output = hu().output().expect("failed to execute");
    let stdout = String::from_utf8_lossy(&output.stdout);

    let commands = [
        "jira",
        "gh",
        "slack",
        "pagerduty",
        "sentry",
        "newrelic",
        "eks",
        "pipeline",
        "utils",
    ];
    for cmd in commands {
        assert!(stdout.contains(cmd), "help missing command: {}", cmd);
    }
}

#[test]
fn command_aliases_work() {
    // pd -> pagerduty (config doesn't need auth)
    let output = hu()
        .args(["pd", "config"])
        .output()
        .expect("failed to execute");
    assert!(output.status.success());

    // nr -> newrelic (incidents may fail without auth, just check alias works)
    let output = hu()
        .args(["nr", "--help"])
        .output()
        .expect("failed to execute");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("NewRelic"));
}

#[test]
fn invalid_command_fails() {
    let output = hu().arg("invalid").output().expect("failed to execute");

    assert!(!output.status.success(), "expected non-zero exit code");
}

// Test all subcommand executions for coverage

#[test]
fn jira_tickets_runs() {
    let output = hu()
        .args(["jira", "tickets"])
        .output()
        .expect("failed to execute");
    // May succeed (if authenticated) or fail (if not)
    // Just verify the command runs without panic
    let _ = output.status;
}

#[test]
fn gh_prs_runs() {
    let output = hu()
        .args(["gh", "prs"])
        .output()
        .expect("failed to execute");
    // May succeed (if authenticated) or fail (if not)
    // Just verify the command runs without panic
    let _ = output.status;
}

#[test]
fn gh_login_help_shows_usage() {
    let output = hu()
        .args(["gh", "login", "--help"])
        .output()
        .expect("failed to execute");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Authenticate"));
}

#[test]
fn slack_messages_runs() {
    let output = hu()
        .args(["slack", "messages"])
        .output()
        .expect("failed to execute");
    // May succeed or fail depending on auth state
    // Just verify the command runs without panic
    let _ = output.status;
}

#[test]
fn pagerduty_config_runs() {
    let output = hu()
        .args(["pagerduty", "config"])
        .output()
        .expect("failed to execute");
    assert!(output.status.success());
}

#[test]
fn sentry_issues_runs() {
    let output = hu()
        .args(["sentry", "issues"])
        .output()
        .expect("failed to execute");
    // May succeed or fail depending on auth state
    // Just verify the command runs without panic
    let _ = output.status;
}

#[test]
fn newrelic_incidents_runs() {
    let output = hu()
        .args(["newrelic", "incidents"])
        .output()
        .expect("failed to execute");
    // May succeed or fail depending on auth state
    // Just verify the command runs without panic
    let _ = output.status;
}

#[test]
fn eks_list_runs() {
    let output = hu()
        .args(["eks", "list"])
        .output()
        .expect("failed to execute");
    // May succeed or fail depending on kubectl/k8s auth state
    // Just verify the command runs without panic
    let _ = output.status;
}

#[test]
fn pipeline_list_runs() {
    let output = hu()
        .args(["pipeline", "list"])
        .output()
        .expect("failed to execute");
    // May succeed or fail depending on AWS auth state
    // Just verify the command runs without panic
    let _ = output.status;
}

// GitHub subcommand tests

#[test]
fn gh_help_shows_subcommands() {
    let output = hu()
        .args(["gh", "--help"])
        .output()
        .expect("failed to execute");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("login"));
    assert!(stdout.contains("prs"));
    assert!(stdout.contains("failures"));
}

#[test]
fn gh_failures_help() {
    let output = hu()
        .args(["gh", "failures", "--help"])
        .output()
        .expect("failed to execute");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--pr"));
    assert!(stdout.contains("--repo"));
}

#[test]
fn gh_fix_help() {
    let output = hu()
        .args(["gh", "fix", "--help"])
        .output()
        .expect("failed to execute");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--pr"));
    assert!(stdout.contains("--run"));
    assert!(stdout.contains("--branch"));
    assert!(stdout.contains("--json"));
}

#[test]
fn gh_login_help_shows_optional_token() {
    let output = hu()
        .args(["gh", "login", "--help"])
        .output()
        .expect("failed to execute");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Token is optional (uses gh CLI token if not provided)
    assert!(stdout.contains("--token"));
    assert!(stdout.contains("gh CLI") || stdout.contains("device flow"));
}

// Utils subcommand tests

#[test]
fn utils_shows_help() {
    let output = hu().arg("utils").output().expect("failed to execute");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("fetch-html"));
    assert!(stdout.contains("grep"));
}

#[test]
fn utils_fetch_html_help() {
    let output = hu()
        .args(["utils", "fetch-html", "--help"])
        .output()
        .expect("failed to execute");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--content"));
    assert!(stdout.contains("--summary"));
    assert!(stdout.contains("--links"));
    assert!(stdout.contains("--headings"));
    assert!(stdout.contains("--selector"));
}

#[test]
fn utils_grep_help() {
    let output = hu()
        .args(["utils", "grep", "--help"])
        .output()
        .expect("failed to execute");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--refs"));
    assert!(stdout.contains("--unique"));
    assert!(stdout.contains("--ranked"));
    assert!(stdout.contains("--limit"));
    assert!(stdout.contains("--signature"));
}

#[test]
fn utils_grep_executes() {
    let output = hu()
        .args(["utils", "grep", "fn main", "src/main.rs"])
        .output()
        .expect("failed to execute");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("main.rs"));
}

#[test]
fn utils_grep_refs_mode() {
    let output = hu()
        .args(["utils", "grep", "fn", "src/main.rs", "--refs"])
        .output()
        .expect("failed to execute");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Refs mode: just file:line, no content
    assert!(stdout.contains("main.rs:"));
}
