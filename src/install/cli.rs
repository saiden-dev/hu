use clap::{Args, Subcommand};

#[derive(Subcommand)]
pub enum InstallCommand {
    /// Install hooks and commands to Claude Code configuration
    Run(InstallArgs),

    /// Show what would be installed without making changes
    Preview(InstallArgs),

    /// List available components
    List,
}

#[derive(Args)]
pub struct InstallArgs {
    /// Install to global ~/.claude directory (default)
    #[arg(short, long, conflicts_with = "local")]
    pub global: bool,

    /// Install to current project's .claude directory
    #[arg(short, long)]
    pub local: bool,

    /// Override existing files
    #[arg(short, long)]
    pub force: bool,

    /// Install only hooks (shell scripts)
    #[arg(long, conflicts_with = "commands_only")]
    pub hooks_only: bool,

    /// Install only commands (slash command documentation)
    #[arg(long)]
    pub commands_only: bool,

    /// Specific components to install (e.g., "hooks/pre-read", "commands/hu/read")
    #[arg(value_name = "COMPONENT")]
    pub components: Vec<String>,
}

impl InstallArgs {
    pub fn target_dir(&self) -> TargetDir {
        if self.local {
            TargetDir::Local
        } else {
            TargetDir::Global
        }
    }

    pub fn install_hooks(&self) -> bool {
        !self.commands_only
    }

    pub fn install_commands(&self) -> bool {
        !self.hooks_only
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetDir {
    Global,
    Local,
}

impl TargetDir {
    pub fn path(&self) -> std::path::PathBuf {
        match self {
            TargetDir::Global => dirs::home_dir()
                .expect("could not find home directory")
                .join(".claude"),
            TargetDir::Local => std::env::current_dir()
                .expect("could not get current directory")
                .join(".claude"),
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            TargetDir::Global => "~/.claude (global)",
            TargetDir::Local => "./.claude (local)",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_dir_defaults_to_global() {
        let args = InstallArgs {
            global: false,
            local: false,
            force: false,
            hooks_only: false,
            commands_only: false,
            components: vec![],
        };
        assert_eq!(args.target_dir(), TargetDir::Global);
    }

    #[test]
    fn target_dir_local_when_specified() {
        let args = InstallArgs {
            global: false,
            local: true,
            force: false,
            hooks_only: false,
            commands_only: false,
            components: vec![],
        };
        assert_eq!(args.target_dir(), TargetDir::Local);
    }

    #[test]
    fn install_both_by_default() {
        let args = InstallArgs {
            global: false,
            local: false,
            force: false,
            hooks_only: false,
            commands_only: false,
            components: vec![],
        };
        assert!(args.install_hooks());
        assert!(args.install_commands());
    }

    #[test]
    fn hooks_only_excludes_commands() {
        let args = InstallArgs {
            global: false,
            local: false,
            force: false,
            hooks_only: true,
            commands_only: false,
            components: vec![],
        };
        assert!(args.install_hooks());
        assert!(!args.install_commands());
    }

    #[test]
    fn commands_only_excludes_hooks() {
        let args = InstallArgs {
            global: false,
            local: false,
            force: false,
            hooks_only: false,
            commands_only: true,
            components: vec![],
        };
        assert!(!args.install_hooks());
        assert!(args.install_commands());
    }
}
