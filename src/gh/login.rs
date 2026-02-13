use anyhow::Result;

use super::auth;
use super::cli::LoginArgs;

/// Handle the `hu gh login` command
pub async fn run(args: LoginArgs) -> Result<()> {
    let username = match args.token {
        Some(token) => auth::login(&token).await?,
        None => auth::device_flow_login().await?,
    };
    println!("{}", format_login_success(&username));
    Ok(())
}

/// Format the login success message (extracted for testability)
pub fn format_login_success(username: &str) -> String {
    format!("✓ Logged in as {}", username)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_login_success_includes_username() {
        let msg = format_login_success("testuser");
        assert!(msg.contains("testuser"));
        assert!(msg.contains("✓"));
        assert!(msg.contains("Logged in as"));
    }

    #[test]
    fn format_login_success_handles_special_chars() {
        let msg = format_login_success("user-name_123");
        assert!(msg.contains("user-name_123"));
    }

    #[test]
    fn login_args_has_token_field() {
        let args = LoginArgs {
            token: Some("test_token".to_string()),
        };
        assert_eq!(args.token, Some("test_token".to_string()));
    }

    #[test]
    fn login_args_token_is_optional() {
        let args = LoginArgs { token: None };
        assert!(args.token.is_none());
    }
}
