//! Shared output format type for CLI commands.

/// Output format for CLI commands.
///
/// Most commands support both human-readable table output and
/// machine-readable JSON output (via `-j`/`--json` flags).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OutputFormat {
    /// Human-readable table format
    #[default]
    Table,
    /// JSON format for scripting
    Json,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_table() {
        let format = OutputFormat::default();
        assert_eq!(format, OutputFormat::Table);
    }

    #[test]
    fn clone_preserves_variant() {
        let format = OutputFormat::Json;
        let cloned = format;
        assert_eq!(cloned, OutputFormat::Json);
    }

    #[test]
    fn copy_preserves_variant() {
        let format = OutputFormat::Json;
        let copied = format;
        assert_eq!(copied, OutputFormat::Json);
        // Original is still usable (Copy)
        assert_eq!(format, OutputFormat::Json);
    }

    #[test]
    fn debug_format() {
        assert_eq!(format!("{:?}", OutputFormat::Table), "Table");
        assert_eq!(format!("{:?}", OutputFormat::Json), "Json");
    }

    #[test]
    fn equality() {
        assert_eq!(OutputFormat::Table, OutputFormat::Table);
        assert_eq!(OutputFormat::Json, OutputFormat::Json);
        assert_ne!(OutputFormat::Table, OutputFormat::Json);
    }
}
