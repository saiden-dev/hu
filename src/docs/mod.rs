mod cli;
mod display;
mod service;
mod types;

pub use cli::DocsCommand;

use anyhow::Result;

/// Run a docs subcommand
pub async fn run_command(cmd: DocsCommand) -> Result<()> {
    match cmd {
        DocsCommand::Add(args) => run_add(args),
        DocsCommand::Get(args) => run_get(args).await,
        DocsCommand::List(args) => run_list(args),
        DocsCommand::Remove(args) => run_remove(args),
        DocsCommand::Sync(args) => run_sync(args),
    }
}

fn run_add(args: cli::AddArgs) -> Result<()> {
    let path = service::add(&args.topic, args.output.as_deref(), args.no_commit)?;
    println!("{}", display::format_created(&path, &args.topic));
    Ok(())
}

async fn run_get(args: cli::GetArgs) -> Result<()> {
    let path = service::get(
        &args.url,
        args.name.as_deref(),
        args.output.as_deref(),
        args.no_commit,
    )
    .await?;
    println!("\x1b[32m\u{2713}\x1b[0m Fetched to {}", path.display());
    Ok(())
}

fn run_list(args: cli::ListArgs) -> Result<()> {
    let docs = service::list(args.path.as_deref())?;
    println!("{}", display::format_docs(&docs, args.json));
    Ok(())
}

fn run_remove(args: cli::RemoveArgs) -> Result<()> {
    let path = service::remove(&args.file, args.dir.as_deref(), args.no_commit)?;
    println!("{}", display::format_removed(&path));
    Ok(())
}

fn run_sync(args: cli::SyncArgs) -> Result<()> {
    let result = service::sync(args.path.as_deref(), args.no_push, args.message.as_deref())?;
    println!("{}", display::format_sync_result(&result, args.json));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docs_command_exported() {
        let _ = std::any::type_name::<DocsCommand>();
    }
}
