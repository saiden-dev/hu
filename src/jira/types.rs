use serde::{Deserialize, Serialize};

/// Jira user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub account_id: String,
    pub display_name: String,
    pub email_address: Option<String>,
}

/// Jira issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub key: String,
    pub summary: String,
    pub status: String,
    pub issue_type: String,
    pub assignee: Option<String>,
    pub description: Option<String>,
    pub updated: String,
}

/// Fields to update on an issue
#[derive(Debug, Clone, Default)]
pub struct IssueUpdate {
    pub summary: Option<String>,
    pub description: Option<String>,
    pub assignee: Option<String>,
}

/// Issue transition (status change)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    pub id: String,
    pub name: String,
}

/// OAuth configuration for Jira
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: String,
}

/// Accessible Jira Cloud resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibleResource {
    pub id: String,
    pub url: String,
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_clone() {
        let user = User {
            account_id: "123".to_string(),
            display_name: "John Doe".to_string(),
            email_address: Some("john@example.com".to_string()),
        };
        let cloned = user.clone();
        assert_eq!(cloned.account_id, user.account_id);
        assert_eq!(cloned.display_name, user.display_name);
        assert_eq!(cloned.email_address, user.email_address);
    }

    #[test]
    fn user_without_email() {
        let user = User {
            account_id: "456".to_string(),
            display_name: "Jane".to_string(),
            email_address: None,
        };
        assert!(user.email_address.is_none());
    }

    #[test]
    fn user_debug_format() {
        let user = User {
            account_id: "id".to_string(),
            display_name: "name".to_string(),
            email_address: None,
        };
        let debug_str = format!("{:?}", user);
        assert!(debug_str.contains("User"));
    }

    #[test]
    fn user_serialize() {
        let user = User {
            account_id: "123".to_string(),
            display_name: "John".to_string(),
            email_address: Some("john@test.com".to_string()),
        };
        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("account_id"));
        assert!(json.contains("123"));
    }

    #[test]
    fn user_deserialize() {
        let json = r#"{"account_id":"abc","display_name":"Test","email_address":null}"#;
        let user: User = serde_json::from_str(json).unwrap();
        assert_eq!(user.account_id, "abc");
        assert_eq!(user.display_name, "Test");
        assert!(user.email_address.is_none());
    }

    #[test]
    fn issue_clone() {
        let issue = Issue {
            key: "PROJ-123".to_string(),
            summary: "Fix bug".to_string(),
            status: "In Progress".to_string(),
            issue_type: "Bug".to_string(),
            assignee: Some("john".to_string()),
            description: Some("A bug description".to_string()),
            updated: "2024-01-15T10:00:00Z".to_string(),
        };
        let cloned = issue.clone();
        assert_eq!(cloned.key, issue.key);
        assert_eq!(cloned.summary, issue.summary);
        assert_eq!(cloned.status, issue.status);
    }

    #[test]
    fn issue_without_optional_fields() {
        let issue = Issue {
            key: "PROJ-456".to_string(),
            summary: "Task".to_string(),
            status: "Open".to_string(),
            issue_type: "Task".to_string(),
            assignee: None,
            description: None,
            updated: "2024-01-15T12:00:00Z".to_string(),
        };
        assert!(issue.assignee.is_none());
        assert!(issue.description.is_none());
    }

    #[test]
    fn issue_debug_format() {
        let issue = Issue {
            key: "K".to_string(),
            summary: "S".to_string(),
            status: "St".to_string(),
            issue_type: "T".to_string(),
            assignee: None,
            description: None,
            updated: "U".to_string(),
        };
        let debug_str = format!("{:?}", issue);
        assert!(debug_str.contains("Issue"));
    }

    #[test]
    fn issue_serialize() {
        let issue = Issue {
            key: "TEST-1".to_string(),
            summary: "Test issue".to_string(),
            status: "Done".to_string(),
            issue_type: "Story".to_string(),
            assignee: Some("user".to_string()),
            description: Some("desc".to_string()),
            updated: "2024-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&issue).unwrap();
        assert!(json.contains("TEST-1"));
        assert!(json.contains("Test issue"));
    }

    #[test]
    fn issue_deserialize() {
        let json = r#"{
            "key": "X-1",
            "summary": "Sum",
            "status": "Open",
            "issue_type": "Bug",
            "assignee": null,
            "description": null,
            "updated": "2024-01-01T00:00:00Z"
        }"#;
        let issue: Issue = serde_json::from_str(json).unwrap();
        assert_eq!(issue.key, "X-1");
        assert_eq!(issue.summary, "Sum");
    }

    #[test]
    fn issue_update_default() {
        let update = IssueUpdate::default();
        assert!(update.summary.is_none());
        assert!(update.description.is_none());
        assert!(update.assignee.is_none());
    }

    #[test]
    fn issue_update_clone() {
        let update = IssueUpdate {
            summary: Some("New summary".to_string()),
            description: Some("New desc".to_string()),
            assignee: Some("user123".to_string()),
        };
        let cloned = update.clone();
        assert_eq!(cloned.summary, update.summary);
        assert_eq!(cloned.description, update.description);
        assert_eq!(cloned.assignee, update.assignee);
    }

    #[test]
    fn issue_update_debug_format() {
        let update = IssueUpdate::default();
        let debug_str = format!("{:?}", update);
        assert!(debug_str.contains("IssueUpdate"));
    }

    #[test]
    fn issue_update_partial() {
        let update = IssueUpdate {
            summary: Some("Only summary".to_string()),
            description: None,
            assignee: None,
        };
        assert!(update.summary.is_some());
        assert!(update.description.is_none());
    }

    #[test]
    fn transition_clone() {
        let transition = Transition {
            id: "31".to_string(),
            name: "In Progress".to_string(),
        };
        let cloned = transition.clone();
        assert_eq!(cloned.id, transition.id);
        assert_eq!(cloned.name, transition.name);
    }

    #[test]
    fn transition_debug_format() {
        let transition = Transition {
            id: "1".to_string(),
            name: "T".to_string(),
        };
        let debug_str = format!("{:?}", transition);
        assert!(debug_str.contains("Transition"));
    }

    #[test]
    fn transition_serialize() {
        let transition = Transition {
            id: "21".to_string(),
            name: "Done".to_string(),
        };
        let json = serde_json::to_string(&transition).unwrap();
        assert!(json.contains("21"));
        assert!(json.contains("Done"));
    }

    #[test]
    fn transition_deserialize() {
        let json = r#"{"id": "11", "name": "To Do"}"#;
        let transition: Transition = serde_json::from_str(json).unwrap();
        assert_eq!(transition.id, "11");
        assert_eq!(transition.name, "To Do");
    }

    #[test]
    fn oauth_config_clone() {
        let config = OAuthConfig {
            client_id: "id123".to_string(),
            client_secret: "secret456".to_string(),
        };
        let cloned = config.clone();
        assert_eq!(cloned.client_id, config.client_id);
        assert_eq!(cloned.client_secret, config.client_secret);
    }

    #[test]
    fn oauth_config_debug_format() {
        let config = OAuthConfig {
            client_id: "id".to_string(),
            client_secret: "secret".to_string(),
        };
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("OAuthConfig"));
    }

    #[test]
    fn oauth_config_serialize() {
        let config = OAuthConfig {
            client_id: "test_id".to_string(),
            client_secret: "test_secret".to_string(),
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("test_id"));
        assert!(json.contains("test_secret"));
    }

    #[test]
    fn oauth_config_deserialize() {
        let json = r#"{"client_id": "cid", "client_secret": "csec"}"#;
        let config: OAuthConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.client_id, "cid");
        assert_eq!(config.client_secret, "csec");
    }

    #[test]
    fn accessible_resource_clone() {
        let resource = AccessibleResource {
            id: "cloud-123".to_string(),
            url: "https://example.atlassian.net".to_string(),
            name: "Example Site".to_string(),
        };
        let cloned = resource.clone();
        assert_eq!(cloned.id, resource.id);
        assert_eq!(cloned.url, resource.url);
        assert_eq!(cloned.name, resource.name);
    }

    #[test]
    fn accessible_resource_debug_format() {
        let resource = AccessibleResource {
            id: "id".to_string(),
            url: "url".to_string(),
            name: "name".to_string(),
        };
        let debug_str = format!("{:?}", resource);
        assert!(debug_str.contains("AccessibleResource"));
    }

    #[test]
    fn accessible_resource_serialize() {
        let resource = AccessibleResource {
            id: "res-id".to_string(),
            url: "https://test.atlassian.net".to_string(),
            name: "Test Site".to_string(),
        };
        let json = serde_json::to_string(&resource).unwrap();
        assert!(json.contains("res-id"));
        assert!(json.contains("https://test.atlassian.net"));
    }

    #[test]
    fn accessible_resource_deserialize() {
        let json = r#"{"id": "abc", "url": "https://x.atlassian.net", "name": "X"}"#;
        let resource: AccessibleResource = serde_json::from_str(json).unwrap();
        assert_eq!(resource.id, "abc");
        assert_eq!(resource.url, "https://x.atlassian.net");
        assert_eq!(resource.name, "X");
    }
}
