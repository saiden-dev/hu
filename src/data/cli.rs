use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum DataCommand {
    /// Sync Claude Code data to local database
    Sync {
        /// Force full resync
        #[arg(short, long)]
        force: bool,

        /// Quiet output
        #[arg(short, long)]
        quiet: bool,
    },

    /// Show data configuration
    Config {
        /// Output as JSON
        #[arg(short, long)]
        json: bool,
    },

    /// Session operations
    Session {
        #[command(subcommand)]
        cmd: SessionCommand,
    },

    /// Usage statistics
    Stats {
        /// Output as JSON
        #[arg(short, long)]
        json: bool,

        /// Today only
        #[arg(short, long)]
        today: bool,
    },

    /// Todo operations
    Todos {
        #[command(subcommand)]
        cmd: TodosCommand,
    },

    /// Search messages
    Search {
        /// Search query
        query: String,

        /// Max results
        #[arg(short = 'n', long, default_value = "20")]
        limit: i64,

        /// Output as JSON
        #[arg(short, long)]
        json: bool,
    },

    /// Tool usage statistics
    Tools {
        /// Show detail for specific tool
        #[arg(short, long)]
        tool: Option<String>,

        /// Output as JSON
        #[arg(short, long)]
        json: bool,
    },

    /// Extract errors from debug logs
    Errors {
        /// Days to look back
        #[arg(short, long, default_value = "7")]
        recent: u32,

        /// Output as JSON
        #[arg(short, long)]
        json: bool,
    },

    /// Pricing analysis
    Pricing {
        /// Subscription tier
        #[arg(short, long, default_value = "max20x")]
        subscription: String,

        /// Billing day of month
        #[arg(short, long, default_value = "6")]
        billing_day: u32,

        /// Output as JSON
        #[arg(short, long)]
        json: bool,
    },

    /// Branch activity statistics
    Branches {
        /// Filter by branch name
        #[arg(short, long)]
        branch: Option<String>,

        /// Max results
        #[arg(short, long, default_value = "20")]
        limit: i64,

        /// Output as JSON
        #[arg(short, long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum SessionCommand {
    /// List sessions
    List {
        /// Filter by project
        #[arg(short, long)]
        project: Option<String>,

        /// Max results
        #[arg(short = 'n', long, default_value = "20")]
        limit: i64,

        /// Output as JSON
        #[arg(short, long)]
        json: bool,
    },

    /// Read session messages
    Read {
        /// Session ID (or prefix)
        id: String,

        /// Output as JSON
        #[arg(short, long)]
        json: bool,
    },

    /// Show current session
    Current {
        /// Output as JSON
        #[arg(short, long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum TodosCommand {
    /// List all todos
    List {
        /// Filter by status
        #[arg(short, long)]
        status: Option<String>,

        /// Output as JSON
        #[arg(short, long)]
        json: bool,
    },

    /// Show pending todos
    Pending {
        /// Filter by project
        #[arg(short, long)]
        project: Option<String>,

        /// Output as JSON
        #[arg(short, long)]
        json: bool,
    },
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    #[derive(Parser)]
    struct TestCli {
        #[command(subcommand)]
        cmd: super::DataCommand,
    }

    #[test]
    fn parse_sync() {
        let cli = TestCli::try_parse_from(["test", "sync"]).unwrap();
        assert!(matches!(
            cli.cmd,
            super::DataCommand::Sync {
                force: false,
                quiet: false
            }
        ));
    }

    #[test]
    fn parse_sync_force() {
        let cli = TestCli::try_parse_from(["test", "sync", "-f"]).unwrap();
        assert!(matches!(
            cli.cmd,
            super::DataCommand::Sync { force: true, .. }
        ));
    }

    #[test]
    fn parse_config() {
        let cli = TestCli::try_parse_from(["test", "config"]).unwrap();
        assert!(matches!(
            cli.cmd,
            super::DataCommand::Config { json: false }
        ));
    }

    #[test]
    fn parse_config_json() {
        let cli = TestCli::try_parse_from(["test", "config", "-j"]).unwrap();
        assert!(matches!(cli.cmd, super::DataCommand::Config { json: true }));
    }

    #[test]
    fn parse_session_list() {
        let cli = TestCli::try_parse_from(["test", "session", "list"]).unwrap();
        assert!(matches!(
            cli.cmd,
            super::DataCommand::Session {
                cmd: super::SessionCommand::List { .. }
            }
        ));
    }

    #[test]
    fn parse_session_list_with_project() {
        let cli = TestCli::try_parse_from(["test", "session", "list", "-p", "myproj"]).unwrap();
        if let super::DataCommand::Session {
            cmd: super::SessionCommand::List { project, .. },
        } = cli.cmd
        {
            assert_eq!(project, Some("myproj".to_string()));
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn parse_session_read() {
        let cli = TestCli::try_parse_from(["test", "session", "read", "abc-123"]).unwrap();
        if let super::DataCommand::Session {
            cmd: super::SessionCommand::Read { id, .. },
        } = cli.cmd
        {
            assert_eq!(id, "abc-123");
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn parse_session_current() {
        let cli = TestCli::try_parse_from(["test", "session", "current"]).unwrap();
        assert!(matches!(
            cli.cmd,
            super::DataCommand::Session {
                cmd: super::SessionCommand::Current { .. }
            }
        ));
    }

    #[test]
    fn parse_stats() {
        let cli = TestCli::try_parse_from(["test", "stats"]).unwrap();
        assert!(matches!(cli.cmd, super::DataCommand::Stats { .. }));
    }

    #[test]
    fn parse_stats_today() {
        let cli = TestCli::try_parse_from(["test", "stats", "-t"]).unwrap();
        if let super::DataCommand::Stats { today, .. } = cli.cmd {
            assert!(today);
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn parse_todos_list() {
        let cli = TestCli::try_parse_from(["test", "todos", "list"]).unwrap();
        assert!(matches!(
            cli.cmd,
            super::DataCommand::Todos {
                cmd: super::TodosCommand::List { .. }
            }
        ));
    }

    #[test]
    fn parse_todos_pending() {
        let cli = TestCli::try_parse_from(["test", "todos", "pending"]).unwrap();
        assert!(matches!(
            cli.cmd,
            super::DataCommand::Todos {
                cmd: super::TodosCommand::Pending { .. }
            }
        ));
    }

    #[test]
    fn parse_search() {
        let cli = TestCli::try_parse_from(["test", "search", "hello"]).unwrap();
        if let super::DataCommand::Search { query, limit, .. } = cli.cmd {
            assert_eq!(query, "hello");
            assert_eq!(limit, 20);
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn parse_search_with_limit() {
        let cli = TestCli::try_parse_from(["test", "search", "hello", "-n", "5"]).unwrap();
        if let super::DataCommand::Search { limit, .. } = cli.cmd {
            assert_eq!(limit, 5);
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn parse_tools() {
        let cli = TestCli::try_parse_from(["test", "tools"]).unwrap();
        assert!(matches!(
            cli.cmd,
            super::DataCommand::Tools { tool: None, .. }
        ));
    }

    #[test]
    fn parse_tools_specific() {
        let cli = TestCli::try_parse_from(["test", "tools", "-t", "Read"]).unwrap();
        if let super::DataCommand::Tools { tool, .. } = cli.cmd {
            assert_eq!(tool, Some("Read".to_string()));
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn parse_errors() {
        let cli = TestCli::try_parse_from(["test", "errors"]).unwrap();
        if let super::DataCommand::Errors { recent, .. } = cli.cmd {
            assert_eq!(recent, 7);
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn parse_pricing() {
        let cli = TestCli::try_parse_from(["test", "pricing"]).unwrap();
        if let super::DataCommand::Pricing {
            subscription,
            billing_day,
            ..
        } = cli.cmd
        {
            assert_eq!(subscription, "max20x");
            assert_eq!(billing_day, 6);
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn parse_branches() {
        let cli = TestCli::try_parse_from(["test", "branches"]).unwrap();
        assert!(matches!(cli.cmd, super::DataCommand::Branches { .. }));
    }

    #[test]
    fn parse_branches_with_filter() {
        let cli = TestCli::try_parse_from(["test", "branches", "-b", "feature"]).unwrap();
        if let super::DataCommand::Branches { branch, .. } = cli.cmd {
            assert_eq!(branch, Some("feature".to_string()));
        } else {
            panic!("wrong variant");
        }
    }
}
