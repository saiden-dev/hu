use super::*;
use serde_json::json;

#[test]
fn get_token_returns_option() {
    // Just verify get_token doesn't panic
    let token = get_token();
    assert!(token.is_some() || token.is_none());
}

// parse_ci_status tests
#[test]
fn parse_ci_status_success_from_runs() {
    let runs = vec![json!({"status": "completed", "conclusion": "success"})];
    assert_eq!(parse_ci_status("pending", Some(&runs)), CiStatus::Success);
}

#[test]
fn parse_ci_status_failed_from_runs() {
    let runs = vec![
        json!({"status": "completed", "conclusion": "success"}),
        json!({"status": "completed", "conclusion": "failure"}),
    ];
    assert_eq!(parse_ci_status("pending", Some(&runs)), CiStatus::Failed);
}

#[test]
fn parse_ci_status_pending_from_runs() {
    let runs = vec![
        json!({"status": "completed", "conclusion": "success"}),
        json!({"status": "in_progress", "conclusion": null}),
    ];
    assert_eq!(parse_ci_status("pending", Some(&runs)), CiStatus::Pending);
}

#[test]
fn parse_ci_status_empty_runs_pending() {
    let runs: Vec<serde_json::Value> = vec![];
    assert_eq!(parse_ci_status("pending", Some(&runs)), CiStatus::Pending);
}

#[test]
fn parse_ci_status_no_runs_uses_state() {
    assert_eq!(parse_ci_status("success", None), CiStatus::Success);
    assert_eq!(parse_ci_status("failure", None), CiStatus::Failed);
    assert_eq!(parse_ci_status("error", None), CiStatus::Failed);
    assert_eq!(parse_ci_status("pending", None), CiStatus::Pending);
    assert_eq!(parse_ci_status("unknown", None), CiStatus::Unknown);
}

#[test]
fn parse_state_string_all_cases() {
    assert_eq!(parse_state_string("success"), CiStatus::Success);
    assert_eq!(parse_state_string("pending"), CiStatus::Pending);
    assert_eq!(parse_state_string("failure"), CiStatus::Failed);
    assert_eq!(parse_state_string("error"), CiStatus::Failed);
    assert_eq!(parse_state_string("other"), CiStatus::Unknown);
}

// extract_failed_jobs tests
#[test]
fn extract_failed_jobs_filters_failures() {
    let jobs = json!({
        "jobs": [
            {"id": 1, "name": "build", "conclusion": "success"},
            {"id": 2, "name": "test", "conclusion": "failure"},
            {"id": 3, "name": "lint", "conclusion": "failure"},
        ]
    });
    let failed = extract_failed_jobs(&jobs);
    assert_eq!(failed.len(), 2);
    assert_eq!(failed[0], (2, "test".to_string()));
    assert_eq!(failed[1], (3, "lint".to_string()));
}

#[test]
fn extract_failed_jobs_empty_when_all_success() {
    let jobs = json!({
        "jobs": [
            {"id": 1, "name": "build", "conclusion": "success"},
        ]
    });
    assert!(extract_failed_jobs(&jobs).is_empty());
}

#[test]
fn extract_failed_jobs_handles_missing_jobs() {
    let jobs = json!({});
    assert!(extract_failed_jobs(&jobs).is_empty());
}

#[test]
fn extract_failed_jobs_handles_null_jobs() {
    let jobs = json!({"jobs": null});
    assert!(extract_failed_jobs(&jobs).is_empty());
}

// extract_run_id tests
#[test]
fn extract_run_id_finds_first() {
    let runs = json!({
        "workflow_runs": [
            {"id": 123},
            {"id": 456},
        ]
    });
    assert_eq!(extract_run_id(&runs), Some(123));
}

#[test]
fn extract_run_id_empty_array() {
    let runs = json!({"workflow_runs": []});
    assert_eq!(extract_run_id(&runs), None);
}

#[test]
fn extract_run_id_missing_key() {
    let runs = json!({});
    assert_eq!(extract_run_id(&runs), None);
}

// extract_pr_number_from_list tests
#[test]
fn extract_pr_number_from_list_finds_first() {
    let prs = json!([{"number": 42}, {"number": 99}]);
    assert_eq!(extract_pr_number_from_list(&prs), Some(42));
}

#[test]
fn extract_pr_number_from_list_single() {
    let prs = json!([{"number": 7}]);
    assert_eq!(extract_pr_number_from_list(&prs), Some(7));
}

#[test]
fn extract_pr_number_from_list_empty() {
    let prs = json!([]);
    assert_eq!(extract_pr_number_from_list(&prs), None);
}

#[test]
fn extract_pr_number_from_list_missing_number() {
    let prs = json!([{"title": "no number"}]);
    assert_eq!(extract_pr_number_from_list(&prs), None);
}

#[test]
fn extract_pr_number_from_list_not_array() {
    let prs = json!({"number": 42});
    assert_eq!(extract_pr_number_from_list(&prs), None);
}

#[test]
fn extract_pr_number_from_list_null() {
    let prs = json!(null);
    assert_eq!(extract_pr_number_from_list(&prs), None);
}

#[test]
fn clean_ci_line_removes_timestamp() {
    let line = "2026-01-27T18:51:46.1029380Z      Failure/Error: some code";
    assert_eq!(clean_ci_line(line), "Failure/Error: some code");
}

#[test]
fn clean_ci_line_preserves_line_without_timestamp() {
    let line = "  some regular line  ";
    assert_eq!(clean_ci_line(line), "some regular line");
}

#[test]
fn clean_ci_line_handles_empty() {
    assert_eq!(clean_ci_line(""), "");
    assert_eq!(clean_ci_line("   "), "");
}

#[test]
fn parse_test_failures_extracts_rspec_failures() {
    let logs = r#"
2026-01-27T18:51:46.1025638Z Failures:
2026-01-27T18:51:46.1026049Z
2026-01-27T18:51:46.1027821Z   1) MyClass does something
2026-01-27T18:51:46.1029380Z      Failure/Error: expect(result).to eq(expected)
2026-01-27T18:51:46.1167230Z        expected: 42
2026-01-27T18:51:46.1168761Z      # ./spec/my_class_spec.rb:10:in `block'
2026-01-27T18:51:46.1174151Z
2026-01-27T18:51:46.1253383Z Failed examples:
2026-01-27T18:51:46.1255271Z rspec ./spec/my_class_spec.rb:8 # MyClass does something
"#;
    let failures = parse_test_failures(logs);
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].spec_file, "./spec/my_class_spec.rb:8");
    assert!(failures[0]
        .failure_text
        .contains("expect(result).to eq(expected)"));
    assert!(failures[0].failure_text.contains("expected: 42"));
}

#[test]
fn parse_test_failures_handles_multiple_failures() {
    let logs = r#"
Failures:

  1) First test fails
     Failure/Error: assert false
       error one
     # ./spec/first_spec.rb:5

  2) Second test fails
     Failure/Error: raise "boom"
       error two
     # ./spec/second_spec.rb:10

Failed examples:

rspec ./spec/first_spec.rb:3 # First test fails
rspec ./spec/second_spec.rb:8 # Second test fails
"#;
    let failures = parse_test_failures(logs);
    assert_eq!(failures.len(), 2);
    assert_eq!(failures[0].spec_file, "./spec/first_spec.rb:3");
    assert_eq!(failures[1].spec_file, "./spec/second_spec.rb:8");
    assert!(failures[0].failure_text.contains("assert false"));
    assert!(failures[1].failure_text.contains("raise \"boom\""));
}

#[test]
fn parse_test_failures_handles_no_failures() {
    let logs = "All tests passed!\n0 failures";
    let failures = parse_test_failures(logs);
    assert!(failures.is_empty());
}

#[test]
fn parse_test_failures_handles_empty_logs() {
    let failures = parse_test_failures("");
    assert!(failures.is_empty());
}

#[test]
fn parse_test_failures_deduplicates() {
    let logs = r#"
Failures:

  1) Test fails
     Failure/Error: fail
     # ./spec/test_spec.rb:5

Failed examples:

rspec ./spec/test_spec.rb:3 # Test fails
rspec ./spec/test_spec.rb:3 # Test fails duplicate
"#;
    let failures = parse_test_failures(logs);
    assert_eq!(failures.len(), 1);
}

#[test]
fn parse_test_failures_mock_error_format() {
    // Test the actual format from the CI logs
    let logs = r#"
2026-01-27T18:51:46.1025638Z Failures:
2026-01-27T18:51:46.1027821Z   1) PricesApiHelper pax value includes pax
2026-01-27T18:51:46.1029380Z      Failure/Error: found_lowest_prices += service.method
2026-01-27T18:51:46.1167230Z        #<InstanceDouble(Packages::Items)> received unexpected message :method
2026-01-27T18:51:46.1168761Z      # ./app/helpers/prices_api_helper.rb:62
2026-01-27T18:51:46.1253383Z Failed examples:
2026-01-27T18:51:46.1255271Z rspec ./spec/helpers/prices_api_helper_spec.rb:289 # PricesApiHelper pax value includes pax
"#;
    let failures = parse_test_failures(logs);
    assert_eq!(failures.len(), 1);
    assert_eq!(
        failures[0].spec_file,
        "./spec/helpers/prices_api_helper_spec.rb:289"
    );
    assert!(failures[0]
        .failure_text
        .contains("received unexpected message"));
}

#[test]
fn parse_test_failures_code_only_when_error_is_stacktrace() {
    let logs = r#"
Failures:

  1) Test with stack trace only
     Failure/Error: some_method_call
     # ./spec/test_spec.rb:5

Failed examples:

rspec ./spec/test_spec.rb:3 # Test with stack trace only
"#;
    let failures = parse_test_failures(logs);
    assert_eq!(failures.len(), 1);
    // Should only have the code line since next line starts with #
    assert_eq!(failures[0].failure_text, "some_method_call");
}

#[test]
fn parse_test_failures_handles_failures_section_only() {
    // Missing "Failed examples:" section
    let logs = r#"
Failures:

  1) Test fails
     Failure/Error: expect(1).to eq(2)
       expected: 2
     # ./spec/test_spec.rb:5
"#;
    let failures = parse_test_failures(logs);
    // No failed examples section means we can't extract spec files
    assert!(failures.is_empty());
}

#[test]
fn parse_test_failures_handles_nested_spec_paths() {
    let logs = r#"
Failures:

  1) Deep path test
     Failure/Error: fail "deep"
       error msg

Failed examples:

rspec ./spec/features/admin/users/permissions_spec.rb:42 # Deep path test
"#;
    let failures = parse_test_failures(logs);
    assert_eq!(failures.len(), 1);
    assert_eq!(
        failures[0].spec_file,
        "./spec/features/admin/users/permissions_spec.rb:42"
    );
}

// extract_workflow_runs tests
#[test]
fn extract_workflow_runs_valid_response() {
    let response = json!({
        "workflow_runs": [
            {
                "id": 100,
                "name": "CI",
                "status": "completed",
                "conclusion": "success",
                "head_branch": "main",
                "html_url": "https://github.com/o/r/actions/runs/100",
                "created_at": "2024-01-15T10:00:00Z",
                "updated_at": "2024-01-15T10:05:00Z",
                "run_number": 42
            },
            {
                "id": 101,
                "name": "Lint",
                "status": "in_progress",
                "conclusion": null,
                "head_branch": "feature",
                "html_url": "https://github.com/o/r/actions/runs/101",
                "created_at": "2024-01-15T11:00:00Z",
                "updated_at": "2024-01-15T11:01:00Z",
                "run_number": 43
            }
        ]
    });
    let runs = extract_workflow_runs(&response);
    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0].id, 100);
    assert_eq!(runs[0].name, "CI");
    assert_eq!(runs[0].conclusion, Some("success".to_string()));
    assert_eq!(runs[0].branch, "main");
    assert_eq!(runs[1].id, 101);
    assert!(runs[1].conclusion.is_none());
}

#[test]
fn extract_workflow_runs_empty() {
    let response = json!({"workflow_runs": []});
    assert!(extract_workflow_runs(&response).is_empty());
}

#[test]
fn extract_workflow_runs_missing_key() {
    let response = json!({});
    assert!(extract_workflow_runs(&response).is_empty());
}

#[test]
fn extract_workflow_runs_skips_invalid() {
    let response = json!({
        "workflow_runs": [
            {"name": "no id"},
            {
                "id": 100,
                "name": "Valid",
                "status": "completed",
                "conclusion": "success",
                "head_branch": "main",
                "html_url": "url",
                "created_at": "c",
                "updated_at": "u",
                "run_number": 1
            }
        ]
    });
    let runs = extract_workflow_runs(&response);
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].id, 100);
}

#[test]
fn extract_workflow_runs_null_runs() {
    let response = json!({"workflow_runs": null});
    assert!(extract_workflow_runs(&response).is_empty());
}

// extract_matching_prs tests
#[test]
fn extract_matching_prs_by_title() {
    let response = json!([
        {
            "number": 1,
            "title": "BFR-1234 Fix login",
            "html_url": "https://github.com/o/r/pull/1",
            "state": "open",
            "head": {"ref": "some-branch"},
            "base": {"repo": {"full_name": "o/r"}},
            "created_at": "c",
            "updated_at": "u"
        },
        {
            "number": 2,
            "title": "Unrelated change",
            "html_url": "https://github.com/o/r/pull/2",
            "state": "open",
            "head": {"ref": "other"},
            "base": {"repo": {"full_name": "o/r"}},
            "created_at": "c",
            "updated_at": "u"
        }
    ]);
    let prs = extract_matching_prs(&response, "BFR-1234");
    assert_eq!(prs.len(), 1);
    assert_eq!(prs[0].number, 1);
}

#[test]
fn extract_matching_prs_by_branch() {
    let response = json!([
        {
            "number": 1,
            "title": "Some PR",
            "html_url": "url",
            "state": "open",
            "head": {"ref": "bfr-1234-fix"},
            "base": {"repo": {"full_name": "o/r"}},
            "created_at": "c",
            "updated_at": "u"
        }
    ]);
    let prs = extract_matching_prs(&response, "BFR-1234");
    assert_eq!(prs.len(), 1);
}

#[test]
fn extract_matching_prs_empty() {
    let response = json!([]);
    assert!(extract_matching_prs(&response, "BFR-1234").is_empty());
}

#[test]
fn extract_matching_prs_no_match() {
    let response = json!([
        {
            "number": 1,
            "title": "Unrelated",
            "html_url": "url",
            "state": "open",
            "head": {"ref": "other"},
            "base": {"repo": {"full_name": "o/r"}},
            "created_at": "c",
            "updated_at": "u"
        }
    ]);
    assert!(extract_matching_prs(&response, "BFR-999").is_empty());
}

#[test]
fn extract_matching_prs_not_array() {
    let response = json!({"not": "array"});
    assert!(extract_matching_prs(&response, "query").is_empty());
}

#[test]
fn extract_matching_prs_case_insensitive() {
    let response = json!([
        {
            "number": 1,
            "title": "bfr-1234 lowercase",
            "html_url": "url",
            "state": "open",
            "head": {"ref": "main"},
            "base": {"repo": {"full_name": "o/r"}},
            "created_at": "c",
            "updated_at": "u"
        }
    ]);
    let prs = extract_matching_prs(&response, "BFR-1234");
    assert_eq!(prs.len(), 1);
}

#[test]
fn clean_ci_line_various_timestamps() {
    // Different timestamp formats from CI
    assert_eq!(
        clean_ci_line("2026-01-27T10:00:00.000Z some text"),
        "some text"
    );
    assert_eq!(clean_ci_line("2026-01-27T10:00:00.1234567Z text"), "text");
    assert_eq!(
        clean_ci_line("2020-12-31T23:59:59.9Z end of year"),
        "end of year"
    );
}
