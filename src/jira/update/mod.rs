use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use super::client::{JiraApi, JiraClient};
use super::types::{IssueUpdate, Transition};

#[cfg(test)]
mod tests;

/// Arguments for update command
#[derive(Debug, Clone)]
pub struct UpdateArgs {
    pub key: String,
    pub summary: Option<String>,
    pub status: Option<String>,
    pub assign: Option<String>,
    pub body: Option<String>,
    pub body_adf: Option<PathBuf>,
}

/// Run the jira update command
pub async fn run(args: UpdateArgs) -> Result<()> {
    let client = JiraClient::new().await?;
    let output = process_update(&client, &args).await?;
    print!("{}", output);
    Ok(())
}

/// Process update command (business logic, testable)
pub async fn process_update(client: &impl JiraApi, args: &UpdateArgs) -> Result<String> {
    let mut output = String::new();
    let mut changes_made = false;

    // Resolve --body-adf to an ADF Value up-front so we can short-circuit
    // on a missing or malformed file before we touch the network.
    let description_adf = match &args.body_adf {
        Some(path) => Some(load_adf(path)?),
        None => None,
    };

    // Handle field updates
    let has_field_updates = args.summary.is_some()
        || args.assign.is_some()
        || args.body.is_some()
        || description_adf.is_some();
    if has_field_updates {
        let assignee = match &args.assign {
            Some(a) if a == "me" => {
                let user = client.get_current_user().await?;
                Some(user.account_id)
            }
            Some(a) => Some(a.clone()),
            None => None,
        };

        let update = IssueUpdate {
            summary: args.summary.clone(),
            description: args.body.clone(),
            description_adf,
            assignee,
        };

        client.update_issue(&args.key, &update).await?;
        changes_made = true;

        if let Some(summary) = &args.summary {
            output.push_str(&format!(
                "\x1b[32m\u{2713}\x1b[0m Updated summary: \"{}\"\n",
                summary
            ));
        }
        if args.body.is_some() {
            output.push_str("\x1b[32m\u{2713}\x1b[0m Updated description\n");
        }
        if args.body_adf.is_some() {
            output.push_str("\x1b[32m\u{2713}\x1b[0m Updated description (raw ADF)\n");
        }
        if args.assign.is_some() {
            output.push_str("\x1b[32m\u{2713}\x1b[0m Updated assignee\n");
        }
    }

    // Handle status transition
    if let Some(target_status) = &args.status {
        let transitions = client.get_transitions(&args.key).await?;
        let transition = find_transition(&transitions, target_status)?;

        client.transition_issue(&args.key, &transition.id).await?;
        changes_made = true;

        output.push_str(&format!(
            "\x1b[32m\u{2713}\x1b[0m Transitioned to: {}\n",
            transition.name
        ));
    }

    if !changes_made {
        bail!("No changes specified. Use --summary, --body, --body-adf, --status, or --assign.");
    }

    Ok(output)
}

/// Find a transition by name (case-insensitive)
fn find_transition<'a>(transitions: &'a [Transition], target: &str) -> Result<&'a Transition> {
    let target_lower = target.to_lowercase();

    // Exact match first
    if let Some(t) = transitions
        .iter()
        .find(|t| t.name.to_lowercase() == target_lower)
    {
        return Ok(t);
    }

    // Partial match
    if let Some(t) = transitions
        .iter()
        .find(|t| t.name.to_lowercase().contains(&target_lower))
    {
        return Ok(t);
    }

    // Build error message with available transitions
    let available: Vec<_> = transitions.iter().map(|t| t.name.as_str()).collect();
    bail!(
        "Status '{}' not found. Available transitions: {}",
        target,
        available.join(", ")
    )
}

/// Read an ADF document from a file and validate it has the expected
/// `{"type": "doc", "version": 1, "content": [...]}` shape. Returning
/// early here gives the user a clear error before any HTTP round-trip.
pub(crate) fn load_adf(path: &Path) -> Result<serde_json::Value> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read ADF file {}", path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&raw)
        .with_context(|| format!("Invalid JSON in ADF file {}", path.display()))?;
    if value["type"].as_str() != Some("doc") {
        bail!(
            "ADF file {} is missing top-level \"type\": \"doc\"",
            path.display()
        );
    }
    if !value["content"].is_array() {
        bail!(
            "ADF file {} is missing top-level \"content\" array",
            path.display()
        );
    }
    Ok(value)
}
