use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum JiraCommand {
    /// Authenticate with Jira via OAuth 2.0
    Auth,

    /// List my tickets in current sprint
    Tickets,

    /// Show all issues in current sprint
    Sprint,

    /// Search tickets using JQL
    Search {
        /// JQL query (e.g., "project = PROJ AND status = 'In Progress'")
        query: String,
    },

    /// Show ticket details
    Show {
        /// Ticket key (e.g., PROJ-123)
        key: String,
    },

    /// Update a ticket
    Update {
        /// Ticket key (e.g., PROJ-123)
        key: String,

        /// New summary/title
        #[arg(long)]
        summary: Option<String>,

        /// New status (transition)
        #[arg(long)]
        status: Option<String>,

        /// Assign to user (account ID or "me")
        #[arg(long)]
        assign: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    // Helper to build a command for testing
    fn build_cmd() -> clap::Command {
        #[derive(clap::Parser)]
        struct TestCli {
            #[command(subcommand)]
            cmd: JiraCommand,
        }
        TestCli::command()
    }

    #[test]
    fn parses_auth() {
        let cmd = build_cmd();
        let matches = cmd.try_get_matches_from(["test", "auth"]);
        assert!(matches.is_ok());
    }

    #[test]
    fn parses_tickets() {
        let cmd = build_cmd();
        let matches = cmd.try_get_matches_from(["test", "tickets"]);
        assert!(matches.is_ok());
    }

    #[test]
    fn parses_sprint() {
        let cmd = build_cmd();
        let matches = cmd.try_get_matches_from(["test", "sprint"]);
        assert!(matches.is_ok());
    }

    #[test]
    fn parses_search() {
        let cmd = build_cmd();
        let matches = cmd.try_get_matches_from(["test", "search", "project = TEST"]);
        assert!(matches.is_ok());
    }

    #[test]
    fn parses_show() {
        let cmd = build_cmd();
        let matches = cmd.try_get_matches_from(["test", "show", "PROJ-123"]);
        assert!(matches.is_ok());
    }

    #[test]
    fn parses_update_with_summary() {
        let cmd = build_cmd();
        let matches =
            cmd.try_get_matches_from(["test", "update", "PROJ-123", "--summary", "New title"]);
        assert!(matches.is_ok());
    }

    #[test]
    fn parses_update_with_status() {
        let cmd = build_cmd();
        let matches = cmd.try_get_matches_from(["test", "update", "PROJ-123", "--status", "Done"]);
        assert!(matches.is_ok());
    }

    #[test]
    fn parses_update_with_assign() {
        let cmd = build_cmd();
        let matches = cmd.try_get_matches_from(["test", "update", "PROJ-123", "--assign", "me"]);
        assert!(matches.is_ok());
    }

    #[test]
    fn parses_update_with_all_options() {
        let cmd = build_cmd();
        let matches = cmd.try_get_matches_from([
            "test",
            "update",
            "PROJ-123",
            "--summary",
            "New title",
            "--status",
            "In Progress",
            "--assign",
            "user123",
        ]);
        assert!(matches.is_ok());
    }

    #[test]
    fn update_requires_key() {
        let cmd = build_cmd();
        let matches = cmd.try_get_matches_from(["test", "update", "--summary", "Title"]);
        assert!(matches.is_err());
    }

    #[test]
    fn search_requires_query() {
        let cmd = build_cmd();
        let matches = cmd.try_get_matches_from(["test", "search"]);
        assert!(matches.is_err());
    }

    #[test]
    fn show_requires_key() {
        let cmd = build_cmd();
        let matches = cmd.try_get_matches_from(["test", "show"]);
        assert!(matches.is_err());
    }

    #[test]
    fn jira_command_debug() {
        let cmd = JiraCommand::Auth;
        let debug_str = format!("{:?}", cmd);
        assert!(debug_str.contains("Auth"));
    }

    #[test]
    fn tickets_command_debug() {
        let cmd = JiraCommand::Tickets;
        let debug_str = format!("{:?}", cmd);
        assert!(debug_str.contains("Tickets"));
    }

    #[test]
    fn sprint_command_debug() {
        let cmd = JiraCommand::Sprint;
        let debug_str = format!("{:?}", cmd);
        assert!(debug_str.contains("Sprint"));
    }

    #[test]
    fn search_command_debug() {
        let cmd = JiraCommand::Search {
            query: "test".to_string(),
        };
        let debug_str = format!("{:?}", cmd);
        assert!(debug_str.contains("Search"));
    }

    #[test]
    fn show_command_debug() {
        let cmd = JiraCommand::Show {
            key: "X-1".to_string(),
        };
        let debug_str = format!("{:?}", cmd);
        assert!(debug_str.contains("Show"));
    }

    #[test]
    fn update_command_debug() {
        let cmd = JiraCommand::Update {
            key: "X-1".to_string(),
            summary: Some("S".to_string()),
            status: None,
            assign: None,
        };
        let debug_str = format!("{:?}", cmd);
        assert!(debug_str.contains("Update"));
    }
}
