use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct DiskUsage {
    pub filesystem: String,
    pub mount_point: String,
    pub fs_type: String,
    pub total: u64,
    pub used: u64,
    pub available: u64,
    pub use_percent: f64,
}

impl DiskUsage {
    /// Alias for available space (for convenience)
    #[allow(dead_code)]
    pub fn free(&self) -> u64 {
        self.available
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disk_usage_free() {
        let usage = DiskUsage {
            filesystem: "/dev/sda1".to_string(),
            mount_point: "/".to_string(),
            fs_type: "ext4".to_string(),
            total: 1000,
            used: 600,
            available: 400,
            use_percent: 60.0,
        };
        assert_eq!(usage.free(), 400);
    }

    #[test]
    fn disk_usage_serialize() {
        let usage = DiskUsage {
            filesystem: "test".to_string(),
            mount_point: "/mnt".to_string(),
            fs_type: "tmpfs".to_string(),
            total: 100,
            used: 50,
            available: 50,
            use_percent: 50.0,
        };
        let json = serde_json::to_string(&usage).unwrap();
        assert!(json.contains("tmpfs"));
    }
}
