//! New Relic NerdGraph client

use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;

use super::config::{load_config, NewRelicConfig};
use super::types::{Incident, Issue};

#[cfg(test)]
mod tests;

const NERDGRAPH_URL: &str = "https://api.newrelic.com/graphql";

/// Trait for New Relic API operations (enables testing with mocks)
pub trait NewRelicApi {
    /// List recent issues
    fn list_issues(&self, limit: usize) -> impl Future<Output = Result<Vec<Issue>>> + Send;

    /// List recent incidents
    fn list_incidents(&self, limit: usize) -> impl Future<Output = Result<Vec<Incident>>> + Send;

    /// Run NRQL query
    fn run_nrql(&self, nrql: &str) -> impl Future<Output = Result<Vec<serde_json::Value>>> + Send;
}
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

    /// Get config reference (for testing)
    #[cfg(test)]
    pub fn config(&self) -> &NewRelicConfig {
        &self.config
    }
}

impl NewRelicApi for NewRelicClient {
    async fn list_issues(&self, limit: usize) -> Result<Vec<Issue>> {
        NewRelicClient::list_issues(self, limit).await
    }

    async fn list_incidents(&self, limit: usize) -> Result<Vec<Incident>> {
        NewRelicClient::list_incidents(self, limit).await
    }

    async fn run_nrql(&self, nrql: &str) -> Result<Vec<serde_json::Value>> {
        NewRelicClient::run_nrql(self, nrql).await
    }
}
