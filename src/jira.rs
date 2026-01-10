use anyhow::{bail, Context, Result};
use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl,
    Scope, TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::PathBuf;

use crate::utils::{print_error, print_header, print_success, spinner};

const AUTH_URL: &str = "https://auth.atlassian.com/authorize";
const TOKEN_URL: &str = "https://auth.atlassian.com/oauth/token";
const CALLBACK_PORT: u16 = 8765;

// ==================== Helpers ====================

fn strip_html(html: &str) -> String {
    let mut result = html
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">");

    // Simple HTML tag stripper
    let mut output = String::with_capacity(result.len());
    let mut in_tag = false;
    for c in result.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => output.push(c),
            _ => {}
        }
    }
    result = output;

    // Collapse multiple newlines
    while result.contains("\n\n\n") {
        result = result.replace("\n\n\n", "\n\n");
    }
    result.trim().to_string()
}

// ==================== Config ====================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JiraConfig {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub cloud_id: Option<String>,
    pub site_url: Option<String>,
}

fn get_jira_token_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir().context("Could not determine config directory")?;
    Ok(config_dir.join("hu").join("jira_token.json"))
}

pub fn load_jira_config() -> Result<JiraConfig> {
    let path = get_jira_token_path()?;
    if path.exists() {
        let content = std::fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&content)?)
    } else {
        Ok(JiraConfig::default())
    }
}

pub fn save_jira_config(config: &JiraConfig) -> Result<()> {
    let path = get_jira_token_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(config)?;
    std::fs::write(&path, content)?;
    Ok(())
}

// ==================== OAuth 2.0 Flow ====================

fn create_oauth_client(config: &JiraConfig) -> Result<BasicClient> {
    let client_id = config
        .client_id
        .as_ref()
        .context("Jira client_id not configured. Run: hu jira setup")?;
    let client_secret = config
        .client_secret
        .as_ref()
        .context("Jira client_secret not configured. Run: hu jira setup")?;

    let client = BasicClient::new(
        ClientId::new(client_id.clone()),
        Some(ClientSecret::new(client_secret.clone())),
        AuthUrl::new(AUTH_URL.to_string())?,
        Some(TokenUrl::new(TOKEN_URL.to_string())?),
    )
    .set_redirect_uri(RedirectUrl::new(format!(
        "http://localhost:{}/callback",
        CALLBACK_PORT
    ))?);

    Ok(client)
}

pub async fn login(config: &mut JiraConfig) -> Result<()> {
    let client = create_oauth_client(config)?;

    // Generate PKCE challenge
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    // Build authorization URL
    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("read:jira-work".to_string()))
        .add_scope(Scope::new("read:jira-user".to_string()))
        .add_scope(Scope::new("offline_access".to_string()))
        .add_extra_param("audience", "api.atlassian.com")
        .add_extra_param("prompt", "consent")
        .set_pkce_challenge(pkce_challenge)
        .url();

    print_header("Jira OAuth Login");
    println!("Opening browser for authentication...");
    println!();

    // Open browser
    if open::that(auth_url.as_str()).is_err() {
        println!("Could not open browser. Please visit this URL manually:");
        println!("{}", auth_url);
    }

    // Start local server to receive callback
    let listener = TcpListener::bind(format!("127.0.0.1:{}", CALLBACK_PORT))
        .context("Failed to bind to callback port")?;

    println!("Waiting for authorization...");
    println!("(Listening on http://localhost:{})", CALLBACK_PORT);

    let code = wait_for_callback(&listener, &csrf_token)?;

    let spin = spinner("Exchanging code for token...");

    // Exchange code for token
    let token_result = client
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(pkce_verifier)
        .request_async(async_http_client)
        .await
        .context("Failed to exchange code for token")?;

    spin.finish_and_clear();

    config.access_token = Some(token_result.access_token().secret().clone());
    if let Some(refresh) = token_result.refresh_token() {
        config.refresh_token = Some(refresh.secret().clone());
    }

    // Get accessible resources (cloud ID)
    let spin = spinner("Fetching Jira site info...");
    fetch_cloud_id(config).await?;
    spin.finish_and_clear();

    save_jira_config(config)?;
    print_success("Logged in to Jira successfully!");

    if let Some(site) = &config.site_url {
        println!("  Site: {}", site);
    }

    Ok(())
}

fn wait_for_callback(listener: &TcpListener, expected_state: &CsrfToken) -> Result<String> {
    let mut stream = listener
        .incoming()
        .flatten()
        .next()
        .context("No callback received")?;

    let mut reader = BufReader::new(&stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    // Parse the request
    let redirect_url = request_line
        .split_whitespace()
        .nth(1)
        .context("Invalid request")?;

    // Extract code and state from query params
    let url = url::Url::parse(&format!("http://localhost{}", redirect_url))?;
    let mut code = None;
    let mut state = None;

    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "code" => code = Some(value.to_string()),
            "state" => state = Some(value.to_string()),
            _ => {}
        }
    }

    // Validate state
    let state = state.context("Missing state parameter")?;
    if state != *expected_state.secret() {
        bail!("CSRF state mismatch");
    }

    let code = code.context("Missing authorization code")?;

    // Send response
    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
        <html><body><h1>Success!</h1><p>You can close this window.</p></body></html>";
    stream.write_all(response.as_bytes())?;

    Ok(code)
}

async fn fetch_cloud_id(config: &mut JiraConfig) -> Result<()> {
    let token = config.access_token.as_ref().context("No access token")?;

    let client = reqwest::Client::new();
    let response: Vec<AccessibleResource> = client
        .get("https://api.atlassian.com/oauth/token/accessible-resources")
        .bearer_auth(token)
        .send()
        .await?
        .json()
        .await?;

    if let Some(resource) = response.first() {
        config.cloud_id = Some(resource.id.clone());
        config.site_url = Some(resource.url.clone());
    } else {
        bail!("No accessible Jira sites found");
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
struct AccessibleResource {
    id: String,
    url: String,
}

// ==================== Jira API ====================

#[derive(Debug, Deserialize)]
pub struct JiraIssue {
    pub key: String,
    pub fields: IssueFields,
}

#[derive(Debug, Deserialize)]
pub struct IssueFields {
    pub summary: String,
    pub status: Option<Status>,
    pub assignee: Option<User>,
    pub reporter: Option<User>,
    pub priority: Option<Priority>,
    pub issuetype: Option<IssueType>,
}

#[derive(Debug, Deserialize)]
pub struct Status {
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
pub struct Priority {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct IssueType {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct SearchResult {
    pub issues: Vec<JiraIssue>,
    /// Total count (not returned by new /search/jql endpoint)
    #[serde(default)]
    pub total: Option<u32>,
}

// ==================== Project Types ====================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JiraProject {
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    pub lead: Option<User>,
    pub project_type_key: Option<String>,
    pub style: Option<String>,
}

pub async fn get_project(config: &JiraConfig, key: &str) -> Result<JiraProject> {
    let token = config.access_token.as_ref().context("Not logged in")?;
    let cloud_id = config.cloud_id.as_ref().context("No cloud ID")?;

    let client = reqwest::Client::new();
    let url = format!(
        "https://api.atlassian.com/ex/jira/{}/rest/api/3/project/{}",
        cloud_id, key
    );

    let response = client.get(&url).bearer_auth(token).send().await?;

    if response.status() == 401 {
        bail!("Unauthorized. Try: hu jira login");
    }

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        bail!("Jira API error ({}): {}", status, text);
    }

    let project: JiraProject = response.json().await?;
    Ok(project)
}

pub async fn list_projects(config: &JiraConfig) -> Result<Vec<JiraProject>> {
    let token = config.access_token.as_ref().context("Not logged in")?;
    let cloud_id = config.cloud_id.as_ref().context("No cloud ID")?;

    let client = reqwest::Client::new();
    let url = format!(
        "https://api.atlassian.com/ex/jira/{}/rest/api/3/project/search",
        cloud_id
    );

    let response = client
        .get(&url)
        .bearer_auth(token)
        .query(&[("maxResults", "100"), ("orderBy", "name")])
        .send()
        .await?;

    if response.status() == 401 {
        bail!("Unauthorized. Try: hu jira login");
    }

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        bail!("Jira API error ({}): {}", status, text);
    }

    #[derive(Deserialize)]
    struct ProjectSearchResult {
        values: Vec<JiraProject>,
    }

    let result: ProjectSearchResult = response.json().await?;
    Ok(result.values)
}

pub fn display_projects(projects: &[JiraProject]) {
    use colored::Colorize;
    use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, Color, Table};

    if projects.is_empty() {
        println!("No projects found");
        return;
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Key").fg(Color::Cyan),
            Cell::new("Name").fg(Color::White),
            Cell::new("Type").fg(Color::Magenta),
            Cell::new("Lead").fg(Color::Green),
        ]);

    for project in projects {
        let project_type = project
            .project_type_key
            .as_deref()
            .unwrap_or("-");
        let lead = project
            .lead
            .as_ref()
            .map(|l| l.display_name.as_str())
            .unwrap_or("-");

        table.add_row(vec![
            Cell::new(&project.key).fg(Color::Cyan),
            Cell::new(&project.name).fg(Color::White),
            Cell::new(project_type).fg(Color::Magenta),
            Cell::new(lead).fg(Color::Green),
        ]);
    }

    println!();
    println!("{}", format!("Found {} projects", projects.len()).dimmed());
    println!("{table}");
    println!();
}

pub fn display_project(project: &JiraProject) {
    use colored::Colorize;

    println!();
    println!(
        "  {}  {}",
        project.key.cyan().bold(),
        project.name.white().bold()
    );
    println!("  {}", "─".repeat(50).dimmed());

    if let Some(desc) = &project.description {
        let clean_desc = strip_html(desc);
        if !clean_desc.trim().is_empty() {
            println!();
            for line in clean_desc.lines() {
                println!("  {}", line.dimmed());
            }
        }
    }

    println!();

    if let Some(lead) = &project.lead {
        println!(
            "  {} {}",
            "Lead:".yellow(),
            lead.display_name.white()
        );
    }

    if let Some(project_type) = &project.project_type_key {
        let type_display = match project_type.as_str() {
            "software" => "Software".magenta(),
            "business" => "Business".blue(),
            "service_desk" => "Service Desk".green(),
            _ => project_type.white(),
        };
        println!("  {} {}", "Type:".yellow(), type_display);
    }

    if let Some(style) = &project.style {
        let style_display = match style.as_str() {
            "next-gen" => "Team-managed".cyan(),
            "classic" => "Company-managed".blue(),
            _ => style.white(),
        };
        println!("  {} {}", "Style:".yellow(), style_display);
    }

    println!();
}

pub async fn get_issue(config: &JiraConfig, key: &str) -> Result<JiraIssue> {
    let token = config.access_token.as_ref().context("Not logged in")?;
    let cloud_id = config.cloud_id.as_ref().context("No cloud ID")?;

    let client = reqwest::Client::new();
    let url = format!(
        "https://api.atlassian.com/ex/jira/{}/rest/api/3/issue/{}",
        cloud_id, key
    );

    let response = client.get(&url).bearer_auth(token).send().await?;

    if response.status() == 401 {
        bail!("Unauthorized. Try: hu jira login");
    }

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        bail!("Jira API error ({}): {}", status, text);
    }

    let issue: JiraIssue = response.json().await?;
    Ok(issue)
}

pub async fn search_issues(
    config: &JiraConfig,
    jql: &str,
    max_results: u32,
) -> Result<SearchResult> {
    let token = config.access_token.as_ref().context("Not logged in")?;
    let cloud_id = config.cloud_id.as_ref().context("No cloud ID")?;

    let client = reqwest::Client::new();
    // Use the new /search/jql endpoint (the old /search endpoint was removed)
    let url = format!(
        "https://api.atlassian.com/ex/jira/{}/rest/api/3/search/jql",
        cloud_id
    );

    let response = client
        .get(&url)
        .bearer_auth(token)
        .query(&[
            ("jql", jql),
            ("maxResults", &max_results.to_string()),
            ("fields", "summary,status,assignee,priority,issuetype"),
        ])
        .send()
        .await?;

    if response.status() == 401 {
        bail!("Unauthorized. Try: hu jira login");
    }

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        bail!("Jira API error ({}): {}", status, text);
    }

    let result: SearchResult = response.json().await?;
    Ok(result)
}

// ==================== Display ====================

pub fn display_issue(issue: &JiraIssue) {
    use colored::Colorize;

    print_header(&format!("{}", issue.key.cyan().bold()));

    println!("  {} {}", "Summary:".dimmed(), issue.fields.summary.white());

    if let Some(status) = &issue.fields.status {
        let status_colored = match status.name.to_lowercase().as_str() {
            "done" | "closed" | "resolved" => status.name.green(),
            "in progress" | "in review" => status.name.yellow(),
            "blocked" => status.name.red(),
            _ => status.name.cyan(),
        };
        println!("  {} {}", "Status:".dimmed(), status_colored);
    }

    if let Some(issue_type) = &issue.fields.issuetype {
        println!("  {} {}", "Type:".dimmed(), issue_type.name.white());
    }

    if let Some(priority) = &issue.fields.priority {
        let priority_colored = match priority.name.to_lowercase().as_str() {
            "highest" | "critical" => priority.name.red().bold(),
            "high" => priority.name.red(),
            "medium" => priority.name.yellow(),
            "low" => priority.name.green(),
            "lowest" => priority.name.dimmed(),
            _ => priority.name.white(),
        };
        println!("  {} {}", "Priority:".dimmed(), priority_colored);
    }

    if let Some(assignee) = &issue.fields.assignee {
        println!(
            "  {} {}",
            "Assignee:".dimmed(),
            assignee.display_name.white()
        );
    }

    if let Some(reporter) = &issue.fields.reporter {
        println!(
            "  {} {}",
            "Reporter:".dimmed(),
            reporter.display_name.white()
        );
    }

    println!();
}

pub fn display_search_results(result: &SearchResult) {
    use colored::Colorize;
    use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, Color, Table};

    if result.issues.is_empty() {
        print_error("No issues found");
        return;
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Key").fg(Color::Cyan),
            Cell::new("Type").fg(Color::Magenta),
            Cell::new("Status").fg(Color::Yellow),
            Cell::new("Summary").fg(Color::White),
            Cell::new("Assignee").fg(Color::Green),
        ]);

    for issue in &result.issues {
        let issue_type = issue
            .fields
            .issuetype
            .as_ref()
            .map(|t| t.name.as_str())
            .unwrap_or("-");
        let status = issue
            .fields
            .status
            .as_ref()
            .map(|s| s.name.as_str())
            .unwrap_or("-");
        let assignee = issue
            .fields
            .assignee
            .as_ref()
            .map(|a| a.display_name.as_str())
            .unwrap_or("Unassigned");
        let summary = if issue.fields.summary.len() > 50 {
            format!("{}...", &issue.fields.summary[..47])
        } else {
            issue.fields.summary.clone()
        };

        table.add_row(vec![
            Cell::new(&issue.key).fg(Color::Cyan),
            Cell::new(issue_type).fg(Color::Magenta),
            Cell::new(status).fg(Color::Yellow),
            Cell::new(summary).fg(Color::White),
            Cell::new(assignee).fg(Color::Green),
        ]);
    }

    println!();
    let count_msg = if let Some(total) = result.total {
        format!("Found {} issues (showing {})", total, result.issues.len())
    } else {
        format!("Showing {} issues", result.issues.len())
    };
    println!("{}", count_msg.dimmed());
    println!("{table}");
    println!();
}

// ==================== Setup ====================

pub fn setup() -> Result<()> {
    use std::io::{stdin, stdout};

    print_header("Jira OAuth Setup");
    println!("Create an OAuth 2.0 app at: https://developer.atlassian.com/console/myapps/");
    println!();

    let mut config = load_jira_config()?;

    print!("Client ID: ");
    stdout().flush()?;
    let mut client_id = String::new();
    stdin().read_line(&mut client_id)?;
    config.client_id = Some(client_id.trim().to_string());

    print!("Client Secret: ");
    stdout().flush()?;
    let mut client_secret = String::new();
    stdin().read_line(&mut client_secret)?;
    config.client_secret = Some(client_secret.trim().to_string());

    save_jira_config(&config)?;
    print_success("Jira credentials saved!");
    println!();
    println!("Now run: hu jira login");

    Ok(())
}
