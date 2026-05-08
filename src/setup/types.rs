//! Shared types for the setup module.

// reason: Status + impls land in Phase 0 chunk 0.4 (status table) and Phase 1+ (installers).
// Tests cover them now; suppress dead_code until first runtime caller wires up.
#![allow(dead_code)]

/// Status of a package, dotfile, or SSH artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Already in desired state — no action taken
    Already,
    /// Action performed and re-verified
    Installed,
    /// Action skipped (filtered out, dry-run, conflicts)
    Skipped,
    /// Failed to reach desired state
    Failed,
    /// Not yet checked
    Unknown,
}

impl Status {
    /// Single-character icon for status tables.
    pub fn icon(self) -> &'static str {
        match self {
            Status::Already => "✓",
            Status::Installed => "✓",
            Status::Skipped => "◐",
            Status::Failed => "✗",
            Status::Unknown => "○",
        }
    }

    /// Whether the status represents a desired-state match.
    pub fn is_satisfied(self) -> bool {
        matches!(self, Status::Already | Status::Installed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icons_match_doctrine() {
        assert_eq!(Status::Already.icon(), "✓");
        assert_eq!(Status::Installed.icon(), "✓");
        assert_eq!(Status::Skipped.icon(), "◐");
        assert_eq!(Status::Failed.icon(), "✗");
        assert_eq!(Status::Unknown.icon(), "○");
    }

    #[test]
    fn is_satisfied_only_for_present_states() {
        assert!(Status::Already.is_satisfied());
        assert!(Status::Installed.is_satisfied());
        assert!(!Status::Skipped.is_satisfied());
        assert!(!Status::Failed.is_satisfied());
        assert!(!Status::Unknown.is_satisfied());
    }
}
