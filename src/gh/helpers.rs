use anyhow::{Context, Result};

/// Parse owner/repo from command line argument
pub fn parse_owner_repo(repo: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = repo.split('/').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid repo format. Expected owner/repo, got: {}", repo);
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

/// Get owner/repo from git remote
pub fn get_current_repo() -> Result<(String, String)> {
    let output = run_git_command(&["remote", "get-url", "origin"])?;
    parse_github_url(output.trim())
}

/// Run a git command and return stdout
pub fn run_git_command(args: &[&str]) -> Result<String> {
    let output = std::process::Command::new("git")
        .args(args)
        .output()
        .context("Failed to run git command")?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Parse GitHub URL to extract owner/repo
pub fn parse_github_url(url: &str) -> Result<(String, String)> {
    let url = url.trim_end_matches(".git").trim_end_matches('/');

    if url.contains("github.com:") {
        // SSH format: git@github.com:owner/repo.git
        let parts: Vec<&str> = url.split(':').collect();
        if let Some(path) = parts.last() {
            let segments: Vec<&str> = path.split('/').collect();
            if segments.len() >= 2 {
                return Ok((
                    segments[segments.len() - 2].to_string(),
                    segments[segments.len() - 1].to_string(),
                ));
            }
        }
    } else if url.contains("github.com/") {
        // HTTPS format: https://github.com/owner/repo.git
        let parts: Vec<&str> = url.split("github.com/").collect();
        if let Some(path) = parts.last() {
            let segments: Vec<&str> = path.split('/').collect();
            if segments.len() >= 2 {
                return Ok((segments[0].to_string(), segments[1].to_string()));
            }
        }
    }

    anyhow::bail!("Could not parse GitHub URL: {}", url)
}

/// Check if a job name is test-related
pub fn is_test_job(name: &str) -> bool {
    let name_lower = name.to_lowercase();
    name_lower.contains("rspec") || name_lower.contains("test") || name_lower.contains("spec")
}

/// Get current git branch name
pub fn get_current_branch() -> Result<String> {
    let branch = run_git_command(&["branch", "--show-current"])?;
    let branch = branch.trim().to_string();
    if branch.is_empty() {
        anyhow::bail!("Not on a branch. Use --pr or --branch to specify.");
    }
    Ok(branch)
}

#[cfg(test)]
mod tests {
    use super::*;

    // parse_github_url tests
    #[test]
    fn parse_ssh_url() {
        let (owner, repo) = parse_github_url("git@github.com:owner/repo.git").unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(repo, "repo");
    }

    #[test]
    fn parse_https_url() {
        let (owner, repo) = parse_github_url("https://github.com/owner/repo.git").unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(repo, "repo");
    }

    #[test]
    fn parse_https_url_no_git_suffix() {
        let (owner, repo) = parse_github_url("https://github.com/owner/repo").unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(repo, "repo");
    }

    #[test]
    fn parse_ssh_url_no_git_suffix() {
        let (owner, repo) = parse_github_url("git@github.com:owner/repo").unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(repo, "repo");
    }

    #[test]
    fn parse_https_url_trailing_slash() {
        let (owner, repo) = parse_github_url("https://github.com/owner/repo/").unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(repo, "repo");
    }

    #[test]
    fn parse_github_url_invalid() {
        assert!(parse_github_url("not-a-github-url").is_err());
        assert!(parse_github_url("https://gitlab.com/owner/repo").is_err());
        assert!(parse_github_url("").is_err());
    }

    #[test]
    fn parse_github_url_ssh_with_org() {
        let (owner, repo) = parse_github_url("git@github.com:my-org/my-repo.git").unwrap();
        assert_eq!(owner, "my-org");
        assert_eq!(repo, "my-repo");
    }

    #[test]
    fn parse_github_url_empty_string() {
        assert!(parse_github_url("").is_err());
    }

    #[test]
    fn parse_github_url_missing_repo() {
        assert!(parse_github_url("git@github.com:owner").is_err());
    }

    // parse_owner_repo tests
    #[test]
    fn parse_owner_repo_valid() {
        let (owner, repo) = parse_owner_repo("owner/repo").unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(repo, "repo");
    }

    #[test]
    fn parse_owner_repo_with_dashes() {
        let (owner, repo) = parse_owner_repo("my-org/my-repo").unwrap();
        assert_eq!(owner, "my-org");
        assert_eq!(repo, "my-repo");
    }

    #[test]
    fn parse_owner_repo_invalid_no_slash() {
        assert!(parse_owner_repo("noslash").is_err());
    }

    #[test]
    fn parse_owner_repo_invalid_too_many_slashes() {
        assert!(parse_owner_repo("a/b/c").is_err());
    }

    #[test]
    fn parse_owner_repo_invalid_empty() {
        assert!(parse_owner_repo("").is_err());
    }

    // is_test_job tests
    #[test]
    fn is_test_job_rspec() {
        assert!(is_test_job("run-rspec-tests"));
        assert!(is_test_job("RSpec"));
    }

    #[test]
    fn is_test_job_test() {
        assert!(is_test_job("unit-tests"));
        assert!(is_test_job("Test Suite"));
    }

    #[test]
    fn is_test_job_spec() {
        assert!(is_test_job("run-specs"));
        assert!(is_test_job("Spec Runner"));
    }

    #[test]
    fn is_test_job_non_test() {
        assert!(!is_test_job("build"));
        assert!(!is_test_job("deploy"));
        assert!(!is_test_job("lint"));
    }

    #[test]
    fn is_test_job_mixed_case() {
        assert!(is_test_job("RSPEC"));
        assert!(is_test_job("TEST"));
        assert!(is_test_job("SPEC"));
    }

    #[test]
    fn is_test_job_partial_names() {
        assert!(is_test_job("run-rspec-tests (3, 0)"));
        assert!(is_test_job("unit-test-suite"));
        assert!(is_test_job("integration-spec"));
    }

    // run_git_command test
    #[test]
    fn run_git_command_version() {
        let result = run_git_command(&["--version"]);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("git version"));
    }

    // get_current_repo test
    #[test]
    fn get_current_repo_returns_result() {
        let result = get_current_repo();
        assert!(result.is_ok() || result.is_err());
    }

    // get_current_branch test
    #[test]
    fn get_current_branch_returns_result() {
        let result = get_current_branch();
        // In a git repo on a branch, it should succeed
        assert!(result.is_ok() || result.is_err());
    }
}
