use super::*;

// Mock implementations for testing
struct MockBraveApi {
    results: Vec<SearchResult>,
}

#[async_trait::async_trait]
impl BraveSearchApi for MockBraveApi {
    async fn search(&self, _query: &str, count: usize) -> Result<Vec<SearchResult>> {
        Ok(self.results.iter().take(count).cloned().collect())
    }
}

struct MockFetcher {
    content: String,
}

#[async_trait::async_trait]
impl HttpFetcher for MockFetcher {
    async fn fetch(&self, _url: &str) -> Result<String> {
        Ok(self.content.clone())
    }
}

struct FailingFetcher;

#[async_trait::async_trait]
impl HttpFetcher for FailingFetcher {
    async fn fetch(&self, url: &str) -> Result<String> {
        bail!("Failed to fetch {}", url)
    }
}

fn sample_results() -> Vec<SearchResult> {
    vec![
        SearchResult {
            title: "Result One".to_string(),
            url: "https://example.com/one".to_string(),
            description: "First result description".to_string(),
        },
        SearchResult {
            title: "Result Two".to_string(),
            url: "https://example.com/two".to_string(),
            description: "Second result description".to_string(),
        },
        SearchResult {
            title: "Result Three".to_string(),
            url: "https://example.com/three".to_string(),
            description: "".to_string(),
        },
    ]
}

#[test]
fn search_result_debug() {
    let result = SearchResult {
        title: "Test".to_string(),
        url: "https://test.com".to_string(),
        description: "Desc".to_string(),
    };
    let debug = format!("{:?}", result);
    assert!(debug.contains("SearchResult"));
    assert!(debug.contains("Test"));
}

#[test]
fn search_result_clone() {
    let result = SearchResult {
        title: "Test".to_string(),
        url: "https://test.com".to_string(),
        description: "Desc".to_string(),
    };
    let cloned = result.clone();
    assert_eq!(cloned.title, result.title);
    assert_eq!(cloned.url, result.url);
}

#[test]
fn web_results_debug() {
    let results = WebResults { results: vec![] };
    let debug = format!("{:?}", results);
    assert!(debug.contains("WebResults"));
}

#[test]
fn brave_search_response_debug() {
    let response = BraveSearchResponse { web: None };
    let debug = format!("{:?}", response);
    assert!(debug.contains("BraveSearchResponse"));
}

#[test]
fn fetched_result_debug() {
    let result = FetchedResult {
        title: "Test".to_string(),
        url: "https://test.com".to_string(),
        description: "Desc".to_string(),
        content: Some("Content".to_string()),
    };
    let debug = format!("{:?}", result);
    assert!(debug.contains("FetchedResult"));
}

#[test]
fn brave_client_new() {
    let client = BraveSearchClient::new("test_key".to_string());
    assert_eq!(client.api_key, "test_key");
}

#[test]
fn brave_client_from_credentials() {
    let creds = BraveCredentials {
        api_key: "creds_key".to_string(),
    };
    let client = BraveSearchClient::from_credentials(&creds);
    assert_eq!(client.api_key, "creds_key");
}

#[test]
fn default_http_fetcher_new() {
    let fetcher = DefaultHttpFetcher::new();
    let _ = format!("{:?}", fetcher.http);
}

#[test]
fn default_http_fetcher_default() {
    let fetcher = DefaultHttpFetcher::default();
    let _ = format!("{:?}", fetcher.http);
}

#[tokio::test]
async fn search_and_fetch_without_content() {
    let api = MockBraveApi {
        results: sample_results(),
    };
    let fetcher = MockFetcher {
        content: "<p>Test</p>".to_string(),
    };

    let results = search_and_fetch(&api, &fetcher, "test", 2, false)
        .await
        .unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].title, "Result One");
    assert!(results[0].content.is_none());
}

#[tokio::test]
async fn search_and_fetch_with_content() {
    let api = MockBraveApi {
        results: sample_results(),
    };
    let fetcher = MockFetcher {
        content: "<p>Fetched content here</p>".to_string(),
    };

    let results = search_and_fetch(&api, &fetcher, "test", 2, true)
        .await
        .unwrap();

    assert_eq!(results.len(), 2);
    assert!(results[0].content.is_some());
    assert!(results[0].content.as_ref().unwrap().contains("Fetched"));
}

#[tokio::test]
async fn search_and_fetch_handles_fetch_failure() {
    let api = MockBraveApi {
        results: sample_results(),
    };
    let fetcher = FailingFetcher;

    let results = search_and_fetch(&api, &fetcher, "test", 2, true)
        .await
        .unwrap();

    assert_eq!(results.len(), 2);
    assert!(results[0].content.is_none());
}

#[tokio::test]
async fn search_and_fetch_limits_results() {
    let api = MockBraveApi {
        results: sample_results(),
    };
    let fetcher = MockFetcher {
        content: "<p>Test</p>".to_string(),
    };

    let results = search_and_fetch(&api, &fetcher, "test", 1, false)
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
}

#[test]
fn format_results_list_mode() {
    let results = vec![
        FetchedResult {
            title: "Title One".to_string(),
            url: "https://one.com".to_string(),
            description: "Description one".to_string(),
            content: None,
        },
        FetchedResult {
            title: "Title Two".to_string(),
            url: "https://two.com".to_string(),
            description: "".to_string(),
            content: None,
        },
    ];

    let output = format_results(&results, false);
    assert!(output.contains("## 1. Title One"));
    assert!(output.contains("**URL:** https://one.com"));
    assert!(output.contains("> Description one"));
    assert!(output.contains("## 2. Title Two"));
    assert!(!output.contains("### Content"));
}

#[test]
fn format_results_with_content() {
    let results = vec![FetchedResult {
        title: "Title".to_string(),
        url: "https://test.com".to_string(),
        description: "Desc".to_string(),
        content: Some("The actual content".to_string()),
    }];

    let output = format_results(&results, true);
    assert!(output.contains("### Content"));
    assert!(output.contains("The actual content"));
}

#[test]
fn format_results_content_unavailable() {
    let results = vec![FetchedResult {
        title: "Title".to_string(),
        url: "https://test.com".to_string(),
        description: "Desc".to_string(),
        content: None,
    }];

    let output = format_results(&results, true);
    assert!(output.contains("*Content unavailable*"));
}

#[test]
fn format_results_empty() {
    let results: Vec<FetchedResult> = vec![];
    let output = format_results(&results, false);
    assert!(output.is_empty());
}

#[test]
fn brave_search_response_deserialize() {
    let json =
        r#"{"web": {"results": [{"title": "Test", "url": "https://t.com", "description": "D"}]}}"#;
    let response: BraveSearchResponse = serde_json::from_str(json).unwrap();
    assert!(response.web.is_some());
    let web = response.web.unwrap();
    assert_eq!(web.results.len(), 1);
    assert_eq!(web.results[0].title, "Test");
}

#[test]
fn brave_search_response_deserialize_empty() {
    let json = r#"{}"#;
    let response: BraveSearchResponse = serde_json::from_str(json).unwrap();
    assert!(response.web.is_none());
}

#[test]
fn brave_search_response_deserialize_empty_web() {
    let json = r#"{"web": {"results": []}}"#;
    let response: BraveSearchResponse = serde_json::from_str(json).unwrap();
    assert!(response.web.is_some());
    assert!(response.web.unwrap().results.is_empty());
}

#[test]
fn search_result_deserialize_missing_description() {
    let json = r#"{"title": "Test", "url": "https://t.com"}"#;
    let result: SearchResult = serde_json::from_str(json).unwrap();
    assert_eq!(result.title, "Test");
    assert_eq!(result.description, "");
}
