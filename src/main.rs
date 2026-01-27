use clap::Parser;

#[derive(Parser)]
#[command(name = "hu")]
#[command(about = "Dev workflow CLI", long_about = None)]
#[command(version)]
struct Cli {
    // Future: #[command(subcommand)] cmd: Commands
}

fn main() -> anyhow::Result<()> {
    let _cli = Cli::parse();
    println!("hu - dev workflow CLI");
    Ok(())
}
