use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::fs;

use super::cli::WebSearchArgs;
use super::fetch_html::extract_summary;
use crate::util::{load_credentials, BraveCredentials};

#[cfg(test)]
mod tests;

// ============================================================================
// Types
// ============================================================================

/// A single search result from Brave API
#[derive(Debug, Clone, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    #[serde(default)]
    pub description: String,
}

/// Web results container from Brave API
#[derive(Debug, Deserialize)]
pub struct WebResults {
    #[serde(default)]
    pub results: Vec<SearchResult>,
}

/// Top-level Brave API response
#[derive(Debug, Deserialize)]
pub struct BraveSearchResponse {
    #[serde(default)]
    pub web: Option<WebResults>,
}

/// Fetched content for a search result
#[derive(Debug)]
pub struct FetchedResult {
    pub title: String,
    pub url: String,
    pub description: String,
    pub content: Option<String>,
}

// ============================================================================
// Client trait
// ============================================================================

/// Trait for Brave Search API operations
#[async_trait::async_trait]
pub trait BraveSearchApi {
    async fn search(&self, query: &str, count: usize) -> Result<Vec<SearchResult>>;
}

/// Production client for Brave Search
pub struct BraveSearchClient {
    api_key: String,
    http: reqwest::Client,
}

impl BraveSearchClient {
    pub fn new(api_key: String) -> Self {
        let http = reqwest::Client::builder()
            .user_agent("hu-cli/0.1")
            .build()
            .expect("Failed to build HTTP client");
        Self { api_key, http }
    }

    pub fn from_credentials(creds: &BraveCredentials) -> Self {
        Self::new(creds.api_key.clone())
    }
}

#[async_trait::async_trait]
impl BraveSearchApi for BraveSearchClient {
    async fn search(&self, query: &str, count: usize) -> Result<Vec<SearchResult>> {
        let url = format!(
            "https://api.search.brave.com/res/v1/web/search?q={}&count={}",
            urlencoding::encode(query),
            count
        );

        let response = self
            .http
            .get(&url)
            .header("Accept", "application/json")
            .header("X-Subscription-Token", &self.api_key)
            .send()
            .await
            .context("Failed to call Brave Search API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            bail!("Brave API error {}: {}", status, body);
        }

        let data: BraveSearchResponse = response
            .json()
            .await
            .context("Failed to parse Brave API response")?;

        Ok(data.web.map(|w| w.results).unwrap_or_default())
    }
}

// ============================================================================
// HTTP fetcher trait
// ============================================================================

/// Trait for fetching URL content
#[async_trait::async_trait]
pub trait HttpFetcher {
    async fn fetch(&self, url: &str) -> Result<String>;
}

/// Production HTTP fetcher
pub struct DefaultHttpFetcher {
    http: reqwest::Client,
}

impl Default for DefaultHttpFetcher {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultHttpFetcher {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .user_agent("hu-cli/0.1")
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to build HTTP client");
        Self { http }
    }
}

#[async_trait::async_trait]
impl HttpFetcher for DefaultHttpFetcher {
    async fn fetch(&self, url: &str) -> Result<String> {
        let response = self
            .http
            .get(url)
            .send()
            .await
            .with_context(|| format!("Failed to fetch {}", url))?;

        response
            .text()
            .await
            .with_context(|| format!("Failed to read response from {}", url))
    }
}

// ============================================================================
// Service
// ============================================================================

/// Search and optionally fetch content from results
pub async fn search_and_fetch(
    api: &impl BraveSearchApi,
    fetcher: &impl HttpFetcher,
    query: &str,
    count: usize,
    fetch_content: bool,
) -> Result<Vec<FetchedResult>> {
    let results = api.search(query, count).await?;

    let mut fetched = Vec::new();
    for result in results.into_iter().take(count) {
        let content = if fetch_content {
            match fetcher.fetch(&result.url).await {
                Ok(html) => Some(extract_summary(&html)),
                Err(_) => None,
            }
        } else {
            None
        };

        fetched.push(FetchedResult {
            title: result.title,
            url: result.url,
            description: result.description,
            content,
        });
    }

    Ok(fetched)
}

/// Format results as markdown
pub fn format_results(results: &[FetchedResult], include_content: bool) -> String {
    let mut output = String::new();

    for (i, result) in results.iter().enumerate() {
        output.push_str(&format!("## {}. {}\n", i + 1, result.title));
        output.push_str(&format!("**URL:** {}\n\n", result.url));

        if !result.description.is_empty() {
            output.push_str(&format!("> {}\n\n", result.description));
        }

        if include_content {
            if let Some(content) = &result.content {
                output.push_str("### Content\n\n");
                output.push_str(content);
                output.push_str("\n\n");
            } else {
                output.push_str("*Content unavailable*\n\n");
            }
        }

        output.push_str("---\n\n");
    }

    output.trim_end().to_string()
}

// ============================================================================
// Handler
// ============================================================================

/// Handle the `hu utils web-search` command
pub async fn run(args: WebSearchArgs) -> Result<()> {
    let creds = load_credentials()?;
    let brave = creds
        .brave
        .context("Brave API key not configured. Add [brave] section to credentials.toml")?;

    let client = BraveSearchClient::from_credentials(&brave);
    let fetcher = DefaultHttpFetcher::new();

    let fetch_content = !args.list;
    let results =
        search_and_fetch(&client, &fetcher, &args.query, args.results, fetch_content).await?;

    let output = format_results(&results, fetch_content);

    if let Some(path) = args.output {
        fs::write(&path, &output).with_context(|| format!("Failed to write to {}", path))?;
        eprintln!("Written to {}", path);
    } else {
        println!("{}", output);
    }

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================
