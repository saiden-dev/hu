mod cli;
mod docs_index;
mod docs_search;
mod docs_section;
mod fetch_html;
mod grep;
mod signature;
mod web_search;

pub use cli::UtilsCommand;

use anyhow::Result;

/// Run a utils subcommand
#[cfg(not(tarpaulin_include))]
pub async fn run_command(cmd: UtilsCommand) -> Result<()> {
    match cmd {
        UtilsCommand::FetchHtml(args) => fetch_html::run(args).await,
        UtilsCommand::Grep(args) => grep::run(args),
        UtilsCommand::WebSearch(args) => web_search::run(args).await,
        UtilsCommand::DocsIndex(args) => run_docs_index(args),
        UtilsCommand::DocsSearch(args) => run_docs_search(args),
        UtilsCommand::DocsSection(args) => run_docs_section(args),
    }
}

use cli::{DocsIndexArgs, DocsSearchArgs, DocsSectionArgs};

#[cfg(not(tarpaulin_include))]
fn run_docs_index(args: DocsIndexArgs) -> Result<()> {
    let index = docs_index::build_index(&args.path)?;

    if let Some(output) = args.output {
        docs_index::save_index(&index, &output)?;
        println!("Index saved to {}", output);
    } else {
        println!(
            "Indexed {} files, {} sections",
            index.file_count(),
            index.section_count()
        );
        for (path, file) in &index.files {
            println!("\n{}:", path);
            for section in &file.sections {
                let indent = "  ".repeat((section.level - 1) as usize);
                println!(
                    "  {}{} (L{}-{})",
                    indent, section.heading, section.start_line, section.end_line
                );
            }
        }
    }

    Ok(())
}

#[cfg(not(tarpaulin_include))]
fn run_docs_search(args: DocsSearchArgs) -> Result<()> {
    let index = docs_index::load_index(&args.index)?;
    let results = docs_search::search_index(&index, &args.query);
    let output = docs_search::format_results(&results, args.limit);
    println!("{}", output);
    Ok(())
}

#[cfg(not(tarpaulin_include))]
fn run_docs_section(args: DocsSectionArgs) -> Result<()> {
    let content = docs_section::extract_section_from_file(&args.file, &args.heading)?;
    println!("{}", content);
    Ok(())
}
