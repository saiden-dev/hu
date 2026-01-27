use serde::{Deserialize, Serialize};

/// GitHub Device Flow: initial code request response
#[derive(Debug, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    #[allow(dead_code)]
    pub expires_in: u64,
    pub interval: u64,
}

/// GitHub Device Flow: token polling response
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum TokenResponse {
    Success {
        access_token: String,
        #[allow(dead_code)]
        token_type: String,
        #[allow(dead_code)]
        scope: String,
    },
    Pending {
        error: String,
        error_description: Option<String>,
        #[allow(dead_code)]
        interval: Option<u64>,
    },
}

/// Pull request data for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    pub html_url: String,
    pub state: String,
    pub repo_full_name: String,
    pub created_at: String,
    pub updated_at: String,
}

/// GitHub user info
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub login: String,
    pub id: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_code_response_parses() {
        let json = r#"{
            "device_code": "abc123",
            "user_code": "ABCD-1234",
            "verification_uri": "https://github.com/login/device",
            "expires_in": 900,
            "interval": 5
        }"#;

        let resp: DeviceCodeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.device_code, "abc123");
        assert_eq!(resp.user_code, "ABCD-1234");
        assert_eq!(resp.verification_uri, "https://github.com/login/device");
        assert_eq!(resp.expires_in, 900);
        assert_eq!(resp.interval, 5);
    }

    #[test]
    fn token_response_success_parses() {
        let json = r#"{
            "access_token": "gho_xxxx",
            "token_type": "bearer",
            "scope": "repo"
        }"#;

        let resp: TokenResponse = serde_json::from_str(json).unwrap();
        match resp {
            TokenResponse::Success {
                access_token,
                token_type,
                scope,
            } => {
                assert_eq!(access_token, "gho_xxxx");
                assert_eq!(token_type, "bearer");
                assert_eq!(scope, "repo");
            }
            TokenResponse::Pending { .. } => panic!("Expected Success variant"),
        }
    }

    #[test]
    fn token_response_pending_parses() {
        let json = r#"{
            "error": "authorization_pending",
            "error_description": "The authorization request is still pending."
        }"#;

        let resp: TokenResponse = serde_json::from_str(json).unwrap();
        match resp {
            TokenResponse::Pending {
                error,
                error_description,
                ..
            } => {
                assert_eq!(error, "authorization_pending");
                assert_eq!(
                    error_description.unwrap(),
                    "The authorization request is still pending."
                );
            }
            TokenResponse::Success { .. } => panic!("Expected Pending variant"),
        }
    }

    #[test]
    fn pull_request_serializes() {
        let pr = PullRequest {
            number: 123,
            title: "Fix bug".to_string(),
            html_url: "https://github.com/org/repo/pull/123".to_string(),
            state: "open".to_string(),
            repo_full_name: "org/repo".to_string(),
            created_at: "2024-01-15T10:00:00Z".to_string(),
            updated_at: "2024-01-15T12:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&pr).unwrap();
        assert!(json.contains("Fix bug"));
        assert!(json.contains("org/repo"));
    }

    #[test]
    fn user_parses() {
        let json = r#"{"login": "testuser", "id": 12345}"#;
        let user: User = serde_json::from_str(json).unwrap();
        assert_eq!(user.login, "testuser");
        assert_eq!(user.id, 12345);
    }
}
