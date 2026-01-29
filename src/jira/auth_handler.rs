use anyhow::Result;

use super::auth;

/// Run the jira auth command
pub async fn run() -> Result<()> {
    println!("Opening browser for Jira authorization...");
    let name = auth::login().await?;
    println!("\x1b[32m\u{2713}\x1b[0m Logged in as {}", name);
    Ok(())
}

#[cfg(test)]
mod tests {
    // Auth handler is thin and delegates to auth module
    // Integration testing would require mocking the browser and OAuth flow
    // Pure function tests are in auth.rs

    #[test]
    fn module_compiles() {
        // Verify the module structure is correct
        assert!(true);
    }
}
