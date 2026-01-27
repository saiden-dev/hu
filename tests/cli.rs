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
        "dashboard",
        "jira",
        "gh",
        "slack",
        "pagerduty",
        "sentry",
        "newrelic",
        "eks",
    ];
    for cmd in commands {
        assert!(stdout.contains(cmd), "help missing command: {}", cmd);
    }
}

#[test]
fn command_aliases_work() {
    // pd -> pagerduty
    let output = hu()
        .args(["pd", "oncall"])
        .output()
        .expect("failed to execute");
    assert!(output.status.success());

    // nr -> newrelic
    let output = hu()
        .args(["nr", "incidents"])
        .output()
        .expect("failed to execute");
    assert!(output.status.success());
}

#[test]
fn invalid_command_fails() {
    let output = hu().arg("invalid").output().expect("failed to execute");

    assert!(!output.status.success(), "expected non-zero exit code");
}

// Test all subcommand executions for coverage

#[test]
fn dashboard_show_runs() {
    let output = hu()
        .args(["dashboard", "show"])
        .output()
        .expect("failed to execute");
    assert!(output.status.success());
}

#[test]
fn jira_tickets_runs() {
    let output = hu()
        .args(["jira", "tickets"])
        .output()
        .expect("failed to execute");
    assert!(output.status.success());
}

#[test]
fn gh_prs_without_auth_shows_error() {
    let output = hu()
        .args(["gh", "prs"])
        .output()
        .expect("failed to execute");
    // Without authentication, should fail with error message
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Not authenticated") || !output.status.success(),
        "gh prs without auth should fail"
    );
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
    assert!(output.status.success());
}

#[test]
fn pagerduty_oncall_runs() {
    let output = hu()
        .args(["pagerduty", "oncall"])
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
    assert!(output.status.success());
}

#[test]
fn newrelic_incidents_runs() {
    let output = hu()
        .args(["newrelic", "incidents"])
        .output()
        .expect("failed to execute");
    assert!(output.status.success());
}

#[test]
fn eks_list_runs() {
    let output = hu()
        .args(["eks", "list"])
        .output()
        .expect("failed to execute");
    assert!(output.status.success());
}
