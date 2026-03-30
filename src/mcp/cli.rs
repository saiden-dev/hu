use clap::Subcommand;

/// MCP server subcommands.
#[derive(Debug, Subcommand)]
pub enum McpCommand {
    /// Start MCP server (JSON-RPC over stdio)
    Serve,
    /// List available MCP tools
    List,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct TestCli {
        #[command(subcommand)]
        cmd: McpCommand,
    }

    #[test]
    fn parse_serve() {
        let cli = TestCli::try_parse_from(["test", "serve"]).unwrap();
        assert!(matches!(cli.cmd, McpCommand::Serve));
    }

    #[test]
    fn parse_list() {
        let cli = TestCli::try_parse_from(["test", "list"]).unwrap();
        assert!(matches!(cli.cmd, McpCommand::List));
    }

    #[test]
    fn requires_subcommand() {
        let result = TestCli::try_parse_from(["test"]);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_unknown_subcommand() {
        let result = TestCli::try_parse_from(["test", "unknown"]);
        assert!(result.is_err());
    }

    #[test]
    fn debug_format_serve() {
        let cli = TestCli::try_parse_from(["test", "serve"]).unwrap();
        let debug = format!("{:?}", cli.cmd);
        assert!(debug.contains("Serve"));
    }

    #[test]
    fn debug_format_list() {
        let cli = TestCli::try_parse_from(["test", "list"]).unwrap();
        let debug = format!("{:?}", cli.cmd);
        assert!(debug.contains("List"));
    }
}
