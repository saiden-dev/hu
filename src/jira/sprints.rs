use anyhow::{bail, Result};
use std::collections::HashMap;

use super::client::JiraClient;

/// Sprint info extracted from issue custom fields.
#[derive(Debug, Clone)]
struct SprintInfo {
    id: i64,
    name: String,
    state: String,
    start_date: Option<String>,
    end_date: Option<String>,
    goal: Option<String>,
    board_id: Option<i64>,
}

/// Run the sprints command — list sprints via JQL.
pub async fn run(state: &str) -> Result<()> {
    let valid = ["active", "future", "closed"];
    if !valid.contains(&state) {
        bail!("Invalid state '{}'. Use: {}", state, valid.join(", "));
    }

    let client = JiraClient::new().await?;

    let jql = match state {
        "active" => "sprint in openSprints()",
        "future" => "sprint in futureSprints()",
        "closed" => "sprint in closedSprints() ORDER BY updated DESC",
        _ => unreachable!(),
    };

    // Sprint data lives in a custom field
    let sprint_field = find_sprint_field_id(&client).await?;

    // Search issues with the sprint field
    let raw = client.search_raw(jql, &[&sprint_field], 50).await?;

    // Extract unique sprints from all issues
    let mut sprints: HashMap<i64, SprintInfo> = HashMap::new();
    if let Some(issue_arr) = raw["issues"].as_array() {
        for issue in issue_arr {
            if let Some(sprint_arr) = issue["fields"][&sprint_field].as_array() {
                for s in sprint_arr {
                    let id = s["id"].as_i64().unwrap_or(0);
                    let sprint_state = s["state"].as_str().unwrap_or("");
                    if id > 0 && !sprints.contains_key(&id) && sprint_state == state {
                        sprints.insert(
                            id,
                            SprintInfo {
                                id,
                                name: s["name"].as_str().unwrap_or("?").to_string(),
                                state: sprint_state.to_string(),
                                start_date: s["startDate"].as_str().map(String::from),
                                end_date: s["endDate"].as_str().map(String::from),
                                goal: s["goal"]
                                    .as_str()
                                    .filter(|g| !g.is_empty())
                                    .map(String::from),
                                board_id: s["boardId"].as_i64(),
                            },
                        );
                    }
                }
            }
        }
    }

    if sprints.is_empty() {
        println!("No {} sprints found", state);
        return Ok(());
    }

    let mut sorted: Vec<_> = sprints.into_values().collect();
    sorted.sort_by_key(|s| s.id);

    println!(
        "\x1b[1mSprints\x1b[0m ({} found, filter: {})\n",
        sorted.len(),
        state
    );

    for sprint in &sorted {
        let color = match sprint.state.as_str() {
            "active" => "\x1b[32m",
            "future" => "\x1b[34m",
            "closed" => "\x1b[90m",
            _ => "\x1b[0m",
        };
        println!("  {}{}\x1b[0m  {}", color, sprint.state, sprint.name);
        if let Some(start) = &sprint.start_date {
            let end = sprint.end_date.as_deref().unwrap_or("?");
            let start = start.split('T').next().unwrap_or(start);
            let end = end.split('T').next().unwrap_or(end);
            println!("    {} → {}", start, end);
        }
        if let Some(goal) = &sprint.goal {
            println!("    Goal: {}", goal);
        }
        if let Some(board_id) = sprint.board_id {
            println!("    Board ID: {}", board_id);
        }
        println!();
    }

    Ok(())
}

/// Find the sprint custom field ID.
/// Jira instances often have multiple "Sprint" fields — prefer the
/// well-known customfield_10020.
async fn find_sprint_field_id(client: &JiraClient) -> Result<String> {
    let fields = client.list_fields().await?;
    let mut candidates: Vec<String> = Vec::new();
    for field in fields {
        let name = field["name"].as_str().unwrap_or("");
        let id = field["id"].as_str().unwrap_or("");
        if name == "Sprint" && id.starts_with("customfield_") {
            candidates.push(id.to_string());
        }
    }
    if candidates.is_empty() {
        bail!("Sprint custom field not found");
    }
    if candidates.contains(&"customfield_10020".to_string()) {
        Ok("customfield_10020".to_string())
    } else {
        Ok(candidates.remove(0))
    }
}
