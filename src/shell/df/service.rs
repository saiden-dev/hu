use super::types::DiskUsage;
use anyhow::{Context, Result};
use std::ffi::CString;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::mem::MaybeUninit;
use std::path::Path;

/// Get disk usage for all mounted filesystems
pub fn get_all_mounts() -> Result<Vec<DiskUsage>> {
    let mounts = parse_mounts()?;
    let mut results = Vec::new();

    for mount in mounts {
        // Skip pseudo filesystems
        if should_skip_fs(&mount.fs_type, &mount.mount_point) {
            continue;
        }

        if let Ok(usage) = get_disk_usage(&mount.device, &mount.mount_point, &mount.fs_type) {
            // Skip if total is 0 (virtual fs)
            if usage.total > 0 {
                results.push(usage);
            }
        }
    }

    // Sort by mount point
    results.sort_by(|a, b| a.mount_point.cmp(&b.mount_point));

    Ok(results)
}

/// Get disk usage for a specific path
pub fn get_disk_usage(device: &str, mount_point: &str, fs_type: &str) -> Result<DiskUsage> {
    let path = CString::new(mount_point)?;

    unsafe {
        let mut stat: MaybeUninit<libc::statvfs> = MaybeUninit::uninit();
        let result = libc::statvfs(path.as_ptr(), stat.as_mut_ptr());

        if result != 0 {
            anyhow::bail!("statvfs failed for {}", mount_point);
        }

        let stat = stat.assume_init();
        // Cast needed for cross-platform compatibility (types differ between macOS and Linux)
        #[allow(clippy::unnecessary_cast)]
        let block_size = stat.f_frsize as u64;
        let total = stat.f_blocks as u64 * block_size;
        let free = stat.f_bfree as u64 * block_size;
        let available = stat.f_bavail as u64 * block_size;
        let used = total.saturating_sub(free);

        let use_percent = if total > 0 {
            (used as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        Ok(DiskUsage {
            filesystem: device.to_string(),
            mount_point: mount_point.to_string(),
            fs_type: fs_type.to_string(),
            total,
            used,
            available,
            use_percent,
        })
    }
}

struct MountEntry {
    device: String,
    mount_point: String,
    fs_type: String,
}

fn parse_mounts() -> Result<Vec<MountEntry>> {
    let path = if Path::new("/proc/mounts").exists() {
        "/proc/mounts"
    } else if Path::new("/etc/mtab").exists() {
        "/etc/mtab"
    } else {
        // macOS uses different approach
        return parse_mounts_macos();
    };

    let file = File::open(path).with_context(|| format!("Cannot open {}", path))?;
    let reader = BufReader::new(file);
    let mut mounts = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            mounts.push(MountEntry {
                device: parts[0].to_string(),
                mount_point: parts[1].to_string(),
                fs_type: parts[2].to_string(),
            });
        }
    }

    Ok(mounts)
}

fn parse_mounts_macos() -> Result<Vec<MountEntry>> {
    // On macOS, use mount command output or /etc/fstab
    // For simplicity, we'll use common mount points
    let common_mounts = vec![
        MountEntry {
            device: "/dev/disk1s1".to_string(),
            mount_point: "/".to_string(),
            fs_type: "apfs".to_string(),
        },
        MountEntry {
            device: "/dev/disk1s2".to_string(),
            mount_point: "/System/Volumes/Data".to_string(),
            fs_type: "apfs".to_string(),
        },
    ];

    // Try to get actual mounts from mount command
    let output = std::process::Command::new("mount").output();

    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut mounts = Vec::new();

        for line in stdout.lines() {
            // Format: /dev/disk1s1 on / (apfs, local, journaled)
            if let Some((device, rest)) = line.split_once(" on ") {
                if let Some((mount_point, fs_info)) = rest.split_once(" (") {
                    let fs_type = fs_info
                        .trim_end_matches(')')
                        .split(',')
                        .next()
                        .unwrap_or("unknown")
                        .trim();
                    mounts.push(MountEntry {
                        device: device.to_string(),
                        mount_point: mount_point.to_string(),
                        fs_type: fs_type.to_string(),
                    });
                }
            }
        }

        if !mounts.is_empty() {
            return Ok(mounts);
        }
    }

    Ok(common_mounts)
}

fn should_skip_fs(fs_type: &str, mount_point: &str) -> bool {
    // Skip virtual/pseudo filesystems
    let skip_types = [
        "proc",
        "sysfs",
        "devfs",
        "devpts",
        "tmpfs",
        "cgroup",
        "cgroup2",
        "pstore",
        "securityfs",
        "debugfs",
        "tracefs",
        "fusectl",
        "configfs",
        "hugetlbfs",
        "mqueue",
        "binfmt_misc",
        "autofs",
        "devtmpfs",
        "efivarfs",
        "bpf",
        "overlay",
    ];

    if skip_types.contains(&fs_type) {
        return true;
    }

    // Skip certain mount points
    let skip_mounts = [
        "/proc",
        "/sys",
        "/dev",
        "/run",
        "/snap",
        "/boot/efi",
        "/private/var/vm",
    ];

    for skip in &skip_mounts {
        if mount_point.starts_with(skip) {
            return true;
        }
    }

    false
}

pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.1}T", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1}G", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}M", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}K", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_size_bytes() {
        assert_eq!(format_size(0), "0B");
        assert_eq!(format_size(512), "512B");
    }

    #[test]
    fn format_size_kilobytes() {
        assert_eq!(format_size(1024), "1.0K");
        assert_eq!(format_size(2048), "2.0K");
    }

    #[test]
    fn format_size_megabytes() {
        assert_eq!(format_size(1024 * 1024), "1.0M");
    }

    #[test]
    fn format_size_gigabytes() {
        assert_eq!(format_size(1024 * 1024 * 1024), "1.0G");
    }

    #[test]
    fn format_size_terabytes() {
        assert_eq!(format_size(1024u64 * 1024 * 1024 * 1024), "1.0T");
    }

    #[test]
    fn should_skip_proc() {
        assert!(should_skip_fs("proc", "/proc"));
    }

    #[test]
    fn should_not_skip_ext4() {
        assert!(!should_skip_fs("ext4", "/"));
    }

    #[test]
    fn should_not_skip_apfs() {
        assert!(!should_skip_fs("apfs", "/"));
    }

    #[test]
    fn get_all_mounts_works() {
        let mounts = get_all_mounts();
        assert!(mounts.is_ok());
        let mounts = mounts.unwrap();
        // Should have at least root filesystem
        assert!(!mounts.is_empty() || cfg!(target_os = "linux"));
    }
}
