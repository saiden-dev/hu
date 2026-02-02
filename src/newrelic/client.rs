//! New Relic NerdGraph client

use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;

use super::config::{load_config, NewRelicConfig};
use super::types::{Incident, Issue};

const NERDGRAPH_URL: &str = "https://api.newrelic.com/graphql";
const MAX_RETRIES: u32 = 3;
const DEFAULT_RETRY_SECS: u64 = 5;

/// GraphQL request
#[derive(Debug, Serialize, Deserialize)]
struct GraphQLRequest {
    query: String,
    variables: serde_json::Value,
}

/// GraphQL response
#[derive(Debug, Deserialize)]
struct GraphQLResponse<T> {
    data: Option<T>,
    errors: Option<Vec<GraphQLError>>,
}

/// GraphQL error
#[derive(Debug, Deserialize)]
struct GraphQLError {
    message: String,
}

/// New Relic client
pub struct NewRelicClient {
    config: NewRelicConfig,
    http: Client,
}

impl NewRelicClient {
    /// Create a new client
    pub fn new() -> Result<Self> {
        let config = load_config()?;
        let http = Client::builder().user_agent("hu-cli/0.1.0").build()?;
        Ok(Self { config, http })
    }

    /// Get config reference
    pub fn config(&self) -> &NewRelicConfig {
        &self.config
    }

    /// Get API key
    fn api_key(&self) -> Result<&str> {
        self.config
            .api_key
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("New Relic API key not configured"))
    }

    /// Get account ID
    fn account_id(&self) -> Result<i64> {
        self.config
            .account_id
            .ok_or_else(|| anyhow::anyhow!("New Relic account ID not configured"))
    }

    /// List recent issues
    pub async fn list_issues(&self, limit: usize) -> Result<Vec<Issue>> {
        let account_id = self.account_id()?;

        let query = r#"
            query($accountId: Int!, $cursor: String) {
                actor {
                    account(id: $accountId) {
                        aiIssues {
                            issues(cursor: $cursor) {
                                issues {
                                    issueId
                                    title
                                    priority
                                    state
                                    entityNames
                                    createdAt
                                    closedAt
                                    activatedAt
                                }
                                nextCursor
                            }
                        }
                    }
                }
            }
        "#;

        let variables = serde_json::json!({
            "accountId": account_id,
            "cursor": null
        });

        #[derive(Deserialize)]
        struct IssuesResponse {
            actor: Actor,
        }

        #[derive(Deserialize)]
        struct Actor {
            account: Account,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Account {
            ai_issues: AiIssues,
        }

        #[derive(Deserialize)]
        struct AiIssues {
            issues: IssuesData,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct IssuesData {
            issues: Vec<Issue>,
            #[allow(dead_code)]
            next_cursor: Option<String>,
        }

        let response: IssuesResponse = self.execute_graphql(query, variables).await?;
        let mut issues = response.actor.account.ai_issues.issues.issues;
        issues.truncate(limit);
        Ok(issues)
    }

    /// List recent incidents
    pub async fn list_incidents(&self, limit: usize) -> Result<Vec<Incident>> {
        let account_id = self.account_id()?;

        let query = r#"
            query($accountId: Int!, $cursor: String) {
                actor {
                    account(id: $accountId) {
                        aiIssues {
                            incidents(cursor: $cursor) {
                                incidents {
                                    incidentId
                                    title
                                    priority
                                    state
                                    accountIds
                                    createdAt
                                    closedAt
                                }
                                nextCursor
                            }
                        }
                    }
                }
            }
        "#;

        let variables = serde_json::json!({
            "accountId": account_id,
            "cursor": null
        });

        #[derive(Deserialize)]
        struct IncidentsResponse {
            actor: Actor,
        }

        #[derive(Deserialize)]
        struct Actor {
            account: Account,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Account {
            ai_issues: AiIssues,
        }

        #[derive(Deserialize)]
        struct AiIssues {
            incidents: IncidentsData,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct IncidentsData {
            incidents: Vec<Incident>,
            #[allow(dead_code)]
            next_cursor: Option<String>,
        }

        let response: IncidentsResponse = self.execute_graphql(query, variables).await?;
        let mut incidents = response.actor.account.ai_issues.incidents.incidents;
        incidents.truncate(limit);
        Ok(incidents)
    }

    /// Run NRQL query
    pub async fn run_nrql(&self, nrql: &str) -> Result<Vec<serde_json::Value>> {
        let account_id = self.account_id()?;

        let query = r#"
            query($accountId: Int!, $nrql: Nrql!) {
                actor {
                    account(id: $accountId) {
                        nrql(query: $nrql) {
                            results
                        }
                    }
                }
            }
        "#;

        let variables = serde_json::json!({
            "accountId": account_id,
            "nrql": nrql
        });

        #[derive(Deserialize)]
        struct NrqlResponse {
            actor: Actor,
        }

        #[derive(Deserialize)]
        struct Actor {
            account: Account,
        }

        #[derive(Deserialize)]
        struct Account {
            nrql: NrqlData,
        }

        #[derive(Deserialize)]
        struct NrqlData {
            results: Vec<serde_json::Value>,
        }

        let response: NrqlResponse = self.execute_graphql(query, variables).await?;
        Ok(response.actor.account.nrql.results)
    }

    /// Execute GraphQL query
    async fn execute_graphql<T: for<'de> Deserialize<'de>>(
        &self,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<T> {
        let api_key = self.api_key()?.to_string();

        let request = GraphQLRequest {
            query: query.to_string(),
            variables,
        };

        let body = serde_json::to_string(&request)?;

        let mut retries = 0;

        loop {
            let response = self
                .http
                .post(NERDGRAPH_URL)
                .header("Api-Key", &api_key)
                .header("Content-Type", "application/json")
                .body(body.clone())
                .send()
                .await?;

            let status = response.status();

            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                if retries >= MAX_RETRIES {
                    return Err(anyhow::anyhow!(
                        "Rate limited after {} retries",
                        MAX_RETRIES
                    ));
                }

                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(DEFAULT_RETRY_SECS);

                eprintln!(
                    "Rate limited, waiting {} seconds... (retry {}/{})",
                    retry_after,
                    retries + 1,
                    MAX_RETRIES
                );
                sleep(Duration::from_secs(retry_after)).await;
                retries += 1;
                continue;
            }

            if !status.is_success() {
                let body = response.text().await.unwrap_or_default();
                return Err(anyhow::anyhow!("HTTP {}: {}", status.as_u16(), body));
            }

            let text = response.text().await?;
            let gql_response: GraphQLResponse<T> = serde_json::from_str(&text).map_err(|e| {
                anyhow::anyhow!("Parse error: {}: {}", e, &text[..text.len().min(200)])
            })?;

            if let Some(errors) = gql_response.errors {
                let messages: Vec<_> = errors.iter().map(|e| e.message.as_str()).collect();
                return Err(anyhow::anyhow!("GraphQL errors: {}", messages.join(", ")));
            }

            return gql_response
                .data
                .ok_or_else(|| anyhow::anyhow!("No data in response"));
        }
    }

    /// Create client from provided config (for testing)
    #[cfg(test)]
    pub fn with_config(config: NewRelicConfig) -> Result<Self> {
        let http = Client::builder().user_agent("hu-cli/0.1.0").build()?;
        Ok(Self { config, http })
    }
}

/// Parse issues from GraphQL response JSON (pure function, testable)
pub fn parse_issues_response(json: &str) -> Result<Vec<Issue>> {
    #[derive(Deserialize)]
    struct IssuesResponse {
        actor: Actor,
    }

    #[derive(Deserialize)]
    struct Actor {
        account: Account,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Account {
        ai_issues: AiIssues,
    }

    #[derive(Deserialize)]
    struct AiIssues {
        issues: IssuesData,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct IssuesData {
        issues: Vec<Issue>,
        #[allow(dead_code)]
        next_cursor: Option<String>,
    }

    let response: IssuesResponse = serde_json::from_str(json)?;
    Ok(response.actor.account.ai_issues.issues.issues)
}

/// Parse incidents from GraphQL response JSON (pure function, testable)
pub fn parse_incidents_response(json: &str) -> Result<Vec<Incident>> {
    #[derive(Deserialize)]
    struct IncidentsResponse {
        actor: Actor,
    }

    #[derive(Deserialize)]
    struct Actor {
        account: Account,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Account {
        ai_issues: AiIssues,
    }

    #[derive(Deserialize)]
    struct AiIssues {
        incidents: IncidentsData,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct IncidentsData {
        incidents: Vec<Incident>,
        #[allow(dead_code)]
        next_cursor: Option<String>,
    }

    let response: IncidentsResponse = serde_json::from_str(json)?;
    Ok(response.actor.account.ai_issues.incidents.incidents)
}

/// Parse NRQL results from GraphQL response JSON (pure function, testable)
pub fn parse_nrql_response(json: &str) -> Result<Vec<serde_json::Value>> {
    #[derive(Deserialize)]
    struct NrqlResponse {
        actor: Actor,
    }

    #[derive(Deserialize)]
    struct Actor {
        account: Account,
    }

    #[derive(Deserialize)]
    struct Account {
        nrql: NrqlData,
    }

    #[derive(Deserialize)]
    struct NrqlData {
        results: Vec<serde_json::Value>,
    }

    let response: NrqlResponse = serde_json::from_str(json)?;
    Ok(response.actor.account.nrql.results)
}

/// Build GraphQL request body (pure function, testable)
pub fn build_graphql_request(query: &str, variables: serde_json::Value) -> Result<String> {
    let request = GraphQLRequest {
        query: query.to_string(),
        variables,
    };
    Ok(serde_json::to_string(&request)?)
}

/// Parse GraphQL errors from response (pure function, testable)
pub fn parse_graphql_errors(json: &str) -> Option<Vec<String>> {
    #[derive(Deserialize)]
    struct ErrorResponse {
        errors: Option<Vec<GraphQLError>>,
    }

    let response: ErrorResponse = serde_json::from_str(json).ok()?;
    response
        .errors
        .map(|errs| errs.into_iter().map(|e| e.message).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graphql_request_serialize() {
        let request = GraphQLRequest {
            query: "query { test }".to_string(),
            variables: serde_json::json!({"id": 123}),
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("query"));
        assert!(json.contains("variables"));
        assert!(json.contains("test"));
        assert!(json.contains("123"));
    }

    #[test]
    fn test_graphql_request_deserialize() {
        let json = r#"{"query":"query { test }","variables":{"id":456}}"#;
        let request: GraphQLRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.query, "query { test }");
        assert_eq!(request.variables["id"], 456);
    }

    #[test]
    fn test_graphql_request_debug() {
        let request = GraphQLRequest {
            query: "test".to_string(),
            variables: serde_json::json!({}),
        };
        let debug = format!("{:?}", request);
        assert!(debug.contains("GraphQLRequest"));
    }

    #[test]
    fn test_graphql_response_with_data() {
        let json = r#"{"data":{"value":42},"errors":null}"#;
        let response: GraphQLResponse<serde_json::Value> = serde_json::from_str(json).unwrap();
        assert!(response.data.is_some());
        assert!(response.errors.is_none());
        assert_eq!(response.data.unwrap()["value"], 42);
    }

    #[test]
    fn test_graphql_response_with_errors() {
        let json = r#"{"data":null,"errors":[{"message":"Something went wrong"}]}"#;
        let response: GraphQLResponse<serde_json::Value> = serde_json::from_str(json).unwrap();
        assert!(response.data.is_none());
        assert!(response.errors.is_some());
        let errors = response.errors.unwrap();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "Something went wrong");
    }

    #[test]
    fn test_graphql_response_debug() {
        let json = r#"{"data":null,"errors":null}"#;
        let response: GraphQLResponse<serde_json::Value> = serde_json::from_str(json).unwrap();
        let debug = format!("{:?}", response);
        assert!(debug.contains("GraphQLResponse"));
    }

    #[test]
    fn test_graphql_error_debug() {
        let json = r#"{"message":"Error!"}"#;
        let error: GraphQLError = serde_json::from_str(json).unwrap();
        let debug = format!("{:?}", error);
        assert!(debug.contains("GraphQLError"));
        assert!(debug.contains("Error!"));
    }

    #[test]
    fn test_parse_issues_response() {
        let json = r#"{
            "actor": {
                "account": {
                    "aiIssues": {
                        "issues": {
                            "issues": [
                                {
                                    "issueId": "ISS-001",
                                    "title": ["Issue Title"],
                                    "priority": "HIGH",
                                    "state": "ACTIVATED",
                                    "entityNames": ["svc-a"],
                                    "createdAt": 1700000000000,
                                    "closedAt": null,
                                    "activatedAt": 1700000100000
                                }
                            ],
                            "nextCursor": null
                        }
                    }
                }
            }
        }"#;
        let issues = parse_issues_response(json).unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].issue_id, "ISS-001");
        assert_eq!(issues[0].priority, "HIGH");
    }

    #[test]
    fn test_parse_issues_response_empty() {
        let json = r#"{
            "actor": {
                "account": {
                    "aiIssues": {
                        "issues": {
                            "issues": [],
                            "nextCursor": null
                        }
                    }
                }
            }
        }"#;
        let issues = parse_issues_response(json).unwrap();
        assert!(issues.is_empty());
    }

    #[test]
    fn test_parse_issues_response_with_cursor() {
        let json = r#"{
            "actor": {
                "account": {
                    "aiIssues": {
                        "issues": {
                            "issues": [],
                            "nextCursor": "abc123"
                        }
                    }
                }
            }
        }"#;
        let issues = parse_issues_response(json).unwrap();
        assert!(issues.is_empty());
    }

    #[test]
    fn test_parse_issues_response_invalid() {
        let json = r#"{"invalid":"json"}"#;
        let result = parse_issues_response(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_incidents_response() {
        let json = r#"{
            "actor": {
                "account": {
                    "aiIssues": {
                        "incidents": {
                            "incidents": [
                                {
                                    "incidentId": "INC-001",
                                    "title": "Incident Title",
                                    "priority": "CRITICAL",
                                    "state": "CLOSED",
                                    "accountIds": [12345],
                                    "createdAt": 1700000000000,
                                    "closedAt": 1700001000000
                                }
                            ],
                            "nextCursor": null
                        }
                    }
                }
            }
        }"#;
        let incidents = parse_incidents_response(json).unwrap();
        assert_eq!(incidents.len(), 1);
        assert_eq!(incidents[0].incident_id, "INC-001");
        assert_eq!(incidents[0].priority, "CRITICAL");
    }

    #[test]
    fn test_parse_incidents_response_empty() {
        let json = r#"{
            "actor": {
                "account": {
                    "aiIssues": {
                        "incidents": {
                            "incidents": [],
                            "nextCursor": null
                        }
                    }
                }
            }
        }"#;
        let incidents = parse_incidents_response(json).unwrap();
        assert!(incidents.is_empty());
    }

    #[test]
    fn test_parse_incidents_response_invalid() {
        let json = r#"{"malformed":"response"}"#;
        let result = parse_incidents_response(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_nrql_response() {
        let json = r#"{
            "actor": {
                "account": {
                    "nrql": {
                        "results": [
                            {"count": 42, "name": "test1"},
                            {"count": 100, "name": "test2"}
                        ]
                    }
                }
            }
        }"#;
        let results = parse_nrql_response(json).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0]["count"], 42);
        assert_eq!(results[1]["name"], "test2");
    }

    #[test]
    fn test_parse_nrql_response_empty() {
        let json = r#"{
            "actor": {
                "account": {
                    "nrql": {
                        "results": []
                    }
                }
            }
        }"#;
        let results = parse_nrql_response(json).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_parse_nrql_response_invalid() {
        let json = r#"{"not":"valid"}"#;
        let result = parse_nrql_response(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_graphql_request() {
        let query = "query { test }";
        let variables = serde_json::json!({"accountId": 12345});
        let body = build_graphql_request(query, variables).unwrap();

        // Verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(parsed["query"], "query { test }");
        assert_eq!(parsed["variables"]["accountId"], 12345);
    }

    #[test]
    fn test_build_graphql_request_complex_variables() {
        let query = "mutation { create }";
        let variables = serde_json::json!({
            "input": {
                "name": "Test",
                "values": [1, 2, 3]
            }
        });
        let body = build_graphql_request(query, variables).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(parsed["variables"]["input"]["name"], "Test");
    }

    #[test]
    fn test_parse_graphql_errors_present() {
        let json = r#"{"errors":[{"message":"Error 1"},{"message":"Error 2"}]}"#;
        let errors = parse_graphql_errors(json).unwrap();
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0], "Error 1");
        assert_eq!(errors[1], "Error 2");
    }

    #[test]
    fn test_parse_graphql_errors_none() {
        let json = r#"{"data":{"result":"ok"}}"#;
        let errors = parse_graphql_errors(json);
        assert!(errors.is_none());
    }

    #[test]
    fn test_parse_graphql_errors_null() {
        let json = r#"{"errors":null}"#;
        let errors = parse_graphql_errors(json);
        assert!(errors.is_none());
    }

    #[test]
    fn test_parse_graphql_errors_invalid_json() {
        let json = "not valid json";
        let errors = parse_graphql_errors(json);
        assert!(errors.is_none());
    }

    #[test]
    fn test_client_with_config_no_api_key() {
        let config = NewRelicConfig {
            api_key: None,
            account_id: Some(12345),
        };
        let client = NewRelicClient::with_config(config).unwrap();
        assert!(client.api_key().is_err());
        assert!(client.account_id().is_ok());
    }

    #[test]
    fn test_client_with_config_no_account_id() {
        let config = NewRelicConfig {
            api_key: Some("NRAK-test".to_string()),
            account_id: None,
        };
        let client = NewRelicClient::with_config(config).unwrap();
        assert!(client.api_key().is_ok());
        assert!(client.account_id().is_err());
    }

    #[test]
    fn test_client_with_config_both() {
        let config = NewRelicConfig {
            api_key: Some("NRAK-both".to_string()),
            account_id: Some(99999),
        };
        let client = NewRelicClient::with_config(config).unwrap();
        assert_eq!(client.api_key().unwrap(), "NRAK-both");
        assert_eq!(client.account_id().unwrap(), 99999);
    }

    #[test]
    fn test_client_config_ref() {
        let config = NewRelicConfig {
            api_key: Some("NRAK-ref".to_string()),
            account_id: Some(11111),
        };
        let client = NewRelicClient::with_config(config).unwrap();
        let config_ref = client.config();
        assert_eq!(config_ref.api_key, Some("NRAK-ref".to_string()));
        assert_eq!(config_ref.account_id, Some(11111));
    }

    #[test]
    fn test_constants() {
        assert_eq!(NERDGRAPH_URL, "https://api.newrelic.com/graphql");
        assert_eq!(MAX_RETRIES, 3);
        assert_eq!(DEFAULT_RETRY_SECS, 5);
    }
}
