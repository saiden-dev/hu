pub mod cli;
mod handlers;
mod server;
mod tools;
mod types;

pub use cli::McpCommand;

use anyhow::Result;

/// Run an MCP subcommand.
pub async fn run_command(cmd: McpCommand) -> Result<()> {
    match cmd {
        McpCommand::Serve => server::run().await,
        McpCommand::List => list_tools(),
    }
}

/// Print all registered MCP tools to stdout.
fn list_tools() -> Result<()> {
    let tool_defs = tools::all_tools();
    for tool in &tool_defs {
        println!("{}: {}", tool.name, tool.description);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_tools_succeeds() {
        // list_tools prints to stdout — just verify no panic/error
        assert!(list_tools().is_ok());
    }
}
