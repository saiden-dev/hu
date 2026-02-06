mod cli;
mod templates;
mod types;

pub use cli::InstallCommand;

use std::fs;
use std::os::unix::fs::PermissionsExt;

use anyhow::{bail, Context, Result};
use comfy_table::{presets::UTF8_FULL_CONDENSED, Cell, Color, Table};

use cli::{InstallArgs, TargetDir};
use templates::{get_components, COMPONENTS};
use types::{Component, ComponentKind, ComponentStatus, InstallStatus};

pub async fn run_command(cmd: InstallCommand) -> Result<()> {
    match cmd {
        InstallCommand::Run(args) => run_install(args, false),
        InstallCommand::Preview(args) => run_install(args, true),
        InstallCommand::List => list_components(),
    }
}

fn list_components() -> Result<()> {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL_CONDENSED);
    table.set_header(vec!["ID", "Type", "Description"]);

    for component in COMPONENTS {
        table.add_row(vec![
            Cell::new(component.id),
            Cell::new(component.kind.label()),
            Cell::new(component.description),
        ]);
    }

    println!("{table}");
    println!();
    println!("Hooks:    {}", templates::get_hooks().len());
    println!("Commands: {}", templates::get_commands().len());
    Ok(())
}

fn run_install(args: InstallArgs, preview: bool) -> Result<()> {
    let target = args.target_dir();
    let base_dir = target.path();

    // Filter components based on args
    let components: Vec<&Component> = if !args.components.is_empty() {
        // User specified specific components
        let mut selected = Vec::new();
        for id in &args.components {
            match COMPONENTS.iter().find(|c| c.id == id.as_str()) {
                Some(c) => selected.push(c),
                None => bail!("Unknown component: {}", id),
            }
        }
        selected
    } else {
        // Use flags to filter
        get_components(args.install_hooks(), args.install_commands())
    };

    if components.is_empty() {
        println!("No components selected for installation.");
        return Ok(());
    }

    // Check status of each component
    let statuses: Vec<ComponentStatus> = components
        .iter()
        .map(|c| check_component_status(c, &base_dir))
        .collect();

    // Display status table
    print_status_table(&statuses, &target);

    // Determine what to install
    let to_install: Vec<_> = statuses
        .iter()
        .filter(|s| {
            matches!(s.status, InstallStatus::Missing)
                || (args.force && matches!(s.status, InstallStatus::Modified))
        })
        .collect();

    let to_skip: Vec<_> = statuses
        .iter()
        .filter(|s| !args.force && matches!(s.status, InstallStatus::Modified))
        .collect();

    if !to_skip.is_empty() {
        println!();
        println!(
            "Skipping {} modified component(s). Use --force to override.",
            to_skip.len()
        );
    }

    if to_install.is_empty() {
        println!();
        println!("Nothing to install. All components are current.");
        return Ok(());
    }

    if preview {
        println!();
        println!(
            "Preview mode. Would install {} component(s):",
            to_install.len()
        );
        for status in &to_install {
            println!("  {} {}", status.status.symbol(), status.component.id);
        }
        return Ok(());
    }

    // Install components
    println!();
    println!("Installing {} component(s)...", to_install.len());

    let has_hooks = to_install
        .iter()
        .any(|s| s.component.kind == ComponentKind::Hook);

    for status in &to_install {
        install_component(status.component, &base_dir)?;
        println!("  ✓ {}", status.component.id);
    }

    // Update settings.json if we installed hooks
    if has_hooks {
        update_settings_json(&base_dir)?;
        println!("  ✓ Updated settings.json with hook configuration");
    }

    println!();
    println!("Installation complete.");

    // Check if hu CLI is available
    if !is_hu_available() {
        println!();
        println!("Warning: 'hu' CLI not found in PATH.");
        println!("Hooks require 'hu' to be installed. Run:");
        println!("  cargo install --path ~/Projects/hu");
    }

    Ok(())
}

fn check_component_status(
    component: &'static Component,
    base_dir: &std::path::Path,
) -> ComponentStatus {
    let target_path = component.target_path(base_dir);

    let status = if !target_path.exists() {
        InstallStatus::Missing
    } else {
        match fs::read_to_string(&target_path) {
            Ok(content) if content == component.content => InstallStatus::Current,
            _ => InstallStatus::Modified,
        }
    };

    ComponentStatus { component, status }
}

fn print_status_table(statuses: &[ComponentStatus], target: &TargetDir) {
    println!("Target: {}", target.display_name());
    println!();

    let mut table = Table::new();
    table.load_preset(UTF8_FULL_CONDENSED);
    table.set_header(vec!["", "Component", "Status"]);

    for status in statuses {
        let (symbol, color) = match status.status {
            InstallStatus::Missing => ("○", Color::Yellow),
            InstallStatus::Current => ("✓", Color::Green),
            InstallStatus::Modified => ("◐", Color::Cyan),
        };

        table.add_row(vec![
            Cell::new(symbol).fg(color),
            Cell::new(status.component.id),
            Cell::new(status.status.label()).fg(color),
        ]);
    }

    println!("{table}");
}

fn install_component(component: &Component, base_dir: &std::path::Path) -> Result<()> {
    let target_path = component.target_path(base_dir);

    // Create parent directories
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }

    // Write content
    fs::write(&target_path, component.content)
        .with_context(|| format!("Failed to write {}", target_path.display()))?;

    // Make hooks executable
    if component.kind == ComponentKind::Hook {
        let mut perms = fs::metadata(&target_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&target_path, perms)?;
    }

    Ok(())
}

fn update_settings_json(base_dir: &std::path::Path) -> Result<()> {
    let settings_path = base_dir.join("settings.json");

    // Read existing settings or create new
    let mut settings: serde_json::Value = if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)?;
        serde_json::from_str(&content)?
    } else {
        serde_json::json!({})
    };

    // Ensure env section exists with defaults
    if settings.get("env").is_none() {
        settings["env"] = serde_json::json!({
            "HU_MAX_FILE_LINES": "500",
            "HU_MAX_GREP_RESULTS": "20",
            "HU_CLEANUP_DAYS": "7"
        });
    }

    // Build hooks configuration
    let hooks_dir = if base_dir.ends_with(".claude") {
        base_dir.display().to_string()
    } else {
        format!("{}", base_dir.display())
    };

    // Use ~ for home directory in paths for portability
    let hooks_prefix = if hooks_dir.starts_with(&dirs::home_dir().unwrap().display().to_string()) {
        "~/.claude"
    } else {
        "./.claude"
    };

    let hooks_config = serde_json::json!({
        "PreToolUse": [
            {
                "matcher": "Read",
                "hooks": [{
                    "type": "command",
                    "command": format!("{}/hooks/hu/pre-read.sh", hooks_prefix),
                    "timeout": 5000
                }]
            },
            {
                "matcher": "Grep",
                "hooks": [{
                    "type": "command",
                    "command": format!("{}/hooks/hu/pre-grep.sh", hooks_prefix),
                    "timeout": 5000
                }]
            },
            {
                "matcher": "WebFetch",
                "hooks": [{
                    "type": "command",
                    "command": format!("{}/hooks/hu/pre-webfetch.sh", hooks_prefix),
                    "timeout": 5000
                }]
            },
            {
                "matcher": "WebSearch",
                "hooks": [{
                    "type": "command",
                    "command": format!("{}/hooks/hu/pre-websearch.sh", hooks_prefix),
                    "timeout": 5000
                }]
            }
        ],
        "SessionStart": [{
            "hooks": [{
                "type": "command",
                "command": format!("{}/hooks/hu/session-start.sh", hooks_prefix),
                "timeout": 30000
            }]
        }],
        "SessionEnd": [{
            "hooks": [{
                "type": "command",
                "command": format!("{}/hooks/hu/session-end.sh", hooks_prefix),
                "timeout": 10000
            }]
        }]
    });

    settings["hooks"] = hooks_config;

    // Write back with pretty formatting
    let content = serde_json::to_string_pretty(&settings)?;
    fs::write(&settings_path, content)?;

    Ok(())
}

fn is_hu_available() -> bool {
    std::process::Command::new("hu")
        .arg("--version")
        .output()
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn check_status_missing() {
        let temp = TempDir::new().unwrap();
        let component = &templates::COMPONENTS[0];
        let status = check_component_status(component, &temp.path().to_path_buf());
        assert_eq!(status.status, InstallStatus::Missing);
    }

    #[test]
    fn check_status_current() {
        let temp = TempDir::new().unwrap();
        let component = &templates::COMPONENTS[0];

        // Create the file with matching content
        let target = temp.path().join(component.path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, component.content).unwrap();

        let status = check_component_status(component, &temp.path().to_path_buf());
        assert_eq!(status.status, InstallStatus::Current);
    }

    #[test]
    fn check_status_modified() {
        let temp = TempDir::new().unwrap();
        let component = &templates::COMPONENTS[0];

        // Create the file with different content
        let target = temp.path().join(component.path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, "modified content").unwrap();

        let status = check_component_status(component, &temp.path().to_path_buf());
        assert_eq!(status.status, InstallStatus::Modified);
    }

    #[test]
    fn install_creates_file() {
        let temp = TempDir::new().unwrap();
        let component = &templates::COMPONENTS[0];

        install_component(component, &temp.path().to_path_buf()).unwrap();

        let target = temp.path().join(component.path);
        assert!(target.exists());
        assert_eq!(fs::read_to_string(&target).unwrap(), component.content);
    }

    #[test]
    fn install_hook_is_executable() {
        let temp = TempDir::new().unwrap();
        let hook = templates::get_hooks()[0];

        install_component(hook, &temp.path().to_path_buf()).unwrap();

        let target = temp.path().join(hook.path);
        let perms = fs::metadata(&target).unwrap().permissions();
        assert_eq!(perms.mode() & 0o111, 0o111); // Executable bits set
    }

    #[test]
    fn update_settings_creates_file() {
        let temp = TempDir::new().unwrap();
        update_settings_json(&temp.path().to_path_buf()).unwrap();

        let settings_path = temp.path().join("settings.json");
        assert!(settings_path.exists());

        let content: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&settings_path).unwrap()).unwrap();
        assert!(content.get("hooks").is_some());
        assert!(content.get("env").is_some());
    }

    #[test]
    fn update_settings_preserves_existing() {
        let temp = TempDir::new().unwrap();
        let settings_path = temp.path().join("settings.json");

        // Create existing settings with custom values
        fs::write(
            &settings_path,
            r#"{"model": "opus", "permissions": {"allow": ["Bash"]}}"#,
        )
        .unwrap();

        update_settings_json(&temp.path().to_path_buf()).unwrap();

        let content: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&settings_path).unwrap()).unwrap();

        // Check preserved values
        assert_eq!(content["model"], "opus");
        assert!(content["permissions"]["allow"].as_array().is_some());

        // Check new values added
        assert!(content.get("hooks").is_some());
    }

    #[test]
    fn get_components_with_both() {
        let components = get_components(true, true);
        let has_hooks = components.iter().any(|c| c.kind == ComponentKind::Hook);
        let has_commands = components.iter().any(|c| c.kind == ComponentKind::Command);
        assert!(has_hooks);
        assert!(has_commands);
    }

    #[test]
    fn get_components_hooks_only() {
        let components = get_components(true, false);
        assert!(components.iter().all(|c| c.kind == ComponentKind::Hook));
    }

    #[test]
    fn get_components_commands_only() {
        let components = get_components(false, true);
        assert!(components.iter().all(|c| c.kind == ComponentKind::Command));
    }
}
