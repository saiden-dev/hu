mod colors;
mod display;
mod service;
mod types;

use crate::shell::cli::LsArgs;
use anyhow::Result;
use std::path::PathBuf;

pub fn run(args: LsArgs) -> Result<()> {
    let path = args.path.unwrap_or_else(|| PathBuf::from("."));
    let entries = service::list_directory(&path, args.all)?;

    let output = if args.json {
        display::format_json(&entries)
    } else if args.long {
        display::format_long(&entries)
    } else {
        display::format_simple(&entries)
    };

    if !output.is_empty() {
        println!("{}", output);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::TempDir;

    #[test]
    fn run_on_empty_dir() {
        let dir = TempDir::new().unwrap();
        let args = LsArgs {
            path: Some(dir.path().to_path_buf()),
            all: false,
            long: false,
            json: false,
        };
        assert!(run(args).is_ok());
    }

    #[test]
    fn run_with_files() {
        let dir = TempDir::new().unwrap();
        File::create(dir.path().join("test.txt")).unwrap();
        fs::create_dir(dir.path().join("subdir")).unwrap();

        let args = LsArgs {
            path: Some(dir.path().to_path_buf()),
            all: false,
            long: false,
            json: false,
        };
        assert!(run(args).is_ok());
    }

    #[test]
    fn run_long_format() {
        let dir = TempDir::new().unwrap();
        File::create(dir.path().join("file.rs")).unwrap();

        let args = LsArgs {
            path: Some(dir.path().to_path_buf()),
            all: false,
            long: true,
            json: false,
        };
        assert!(run(args).is_ok());
    }

    #[test]
    fn run_json_format() {
        let dir = TempDir::new().unwrap();
        File::create(dir.path().join("data.json")).unwrap();

        let args = LsArgs {
            path: Some(dir.path().to_path_buf()),
            all: false,
            long: false,
            json: true,
        };
        assert!(run(args).is_ok());
    }

    #[test]
    fn run_with_hidden() {
        let dir = TempDir::new().unwrap();
        File::create(dir.path().join(".hidden")).unwrap();

        let args = LsArgs {
            path: Some(dir.path().to_path_buf()),
            all: true,
            long: false,
            json: false,
        };
        assert!(run(args).is_ok());
    }

    #[test]
    fn run_nonexistent_dir_fails() {
        let args = LsArgs {
            path: Some(PathBuf::from("/nonexistent/path/12345")),
            all: false,
            long: false,
            json: false,
        };
        assert!(run(args).is_err());
    }
}
