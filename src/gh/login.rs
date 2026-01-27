use anyhow::Result;

use super::auth;

/// Handle the `hu gh login` command
pub async fn run() -> Result<()> {
    // Check if already authenticated
    if auth::is_authenticated() {
        println!("Already logged in. Run `hu gh logout` to re-authenticate.");
        return Ok(());
    }

    match auth::login().await {
        Ok(username) => {
            println!("✓ Logged in as {}", username);
            Ok(())
        }
        Err(e) => {
            eprintln!("✗ Login failed: {}", e);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    // Integration tests for login require mocking the OAuth flow
    // The run() function is tested via CLI integration tests
}
