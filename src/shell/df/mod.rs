mod display;
mod service;
mod types;

use crate::shell::cli::DfArgs;
use anyhow::Result;

pub fn run(args: DfArgs) -> Result<()> {
    let disks = service::get_all_mounts()?;

    let output = if args.json {
        display::format_json(&disks)
    } else {
        display::format_table(&disks)
    };

    println!("{}", output);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_default() {
        let args = DfArgs { json: false };
        // Should not fail - will return at least root filesystem
        assert!(run(args).is_ok());
    }

    #[test]
    fn run_json() {
        let args = DfArgs { json: true };
        assert!(run(args).is_ok());
    }
}
