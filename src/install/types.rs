use std::path::{Path, PathBuf};

/// A component that can be installed
#[derive(Debug, Clone)]
pub struct Component {
    pub id: &'static str,
    pub kind: ComponentKind,
    pub description: &'static str,
    pub path: &'static str,
    pub content: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentKind {
    Hook,
    Command,
}

impl ComponentKind {
    pub fn label(&self) -> &'static str {
        match self {
            ComponentKind::Hook => "hook",
            ComponentKind::Command => "command",
        }
    }
}

impl Component {
    pub fn target_path(&self, base_dir: &Path) -> PathBuf {
        base_dir.join(self.path)
    }
}

/// Result of checking a component's install status
#[derive(Debug, Clone)]
pub struct ComponentStatus {
    pub component: &'static Component,
    pub status: InstallStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallStatus {
    /// Not installed
    Missing,
    /// Installed and matches
    Current,
    /// Installed but content differs
    Modified,
}

impl InstallStatus {
    pub fn symbol(&self) -> &'static str {
        match self {
            InstallStatus::Missing => "○",
            InstallStatus::Current => "✓",
            InstallStatus::Modified => "◐",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            InstallStatus::Missing => "missing",
            InstallStatus::Current => "current",
            InstallStatus::Modified => "modified",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_kind_labels() {
        assert_eq!(ComponentKind::Hook.label(), "hook");
        assert_eq!(ComponentKind::Command.label(), "command");
    }

    #[test]
    fn install_status_symbols() {
        assert_eq!(InstallStatus::Missing.symbol(), "○");
        assert_eq!(InstallStatus::Current.symbol(), "✓");
        assert_eq!(InstallStatus::Modified.symbol(), "◐");
    }

    #[test]
    fn target_path_combines_base_and_component_path() {
        let component = Component {
            id: "test",
            kind: ComponentKind::Hook,
            description: "Test hook",
            path: "hooks/test.sh",
            content: "#!/bin/bash\necho test",
        };
        let base = PathBuf::from("/home/user/.claude");
        assert_eq!(
            component.target_path(&base),
            PathBuf::from("/home/user/.claude/hooks/test.sh")
        );
    }
}
