use super::service::format_size;
use super::types::DiskUsage;
use comfy_table::{
    presets::UTF8_FULL_CONDENSED, Attribute, Cell, Color, ContentArrangement, Table,
};

pub fn format_table(disks: &[DiskUsage]) -> String {
    if disks.is_empty() {
        return String::from("No filesystems found");
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("Filesystem").add_attribute(Attribute::Dim),
            Cell::new("Type").add_attribute(Attribute::Dim),
            Cell::new("Size").add_attribute(Attribute::Dim),
            Cell::new("Used").add_attribute(Attribute::Dim),
            Cell::new("Avail").add_attribute(Attribute::Dim),
            Cell::new("Use%").add_attribute(Attribute::Dim),
            Cell::new("Mounted on").add_attribute(Attribute::Dim),
        ]);

    for disk in disks {
        let use_color = usage_color(disk.use_percent);
        let bar = usage_bar(disk.use_percent);

        table.add_row(vec![
            Cell::new(&disk.filesystem),
            Cell::new(&disk.fs_type).add_attribute(Attribute::Dim),
            Cell::new(format_size(disk.total)).fg(Color::Cyan),
            Cell::new(format_size(disk.used)).fg(use_color),
            Cell::new(format_size(disk.available)).fg(Color::Green),
            Cell::new(format!("{:>5.1}% {}", disk.use_percent, bar)).fg(use_color),
            Cell::new(&disk.mount_point).add_attribute(Attribute::Bold),
        ]);
    }

    table.to_string()
}

pub fn format_json(disks: &[DiskUsage]) -> String {
    serde_json::to_string_pretty(disks).unwrap_or_else(|_| "[]".to_string())
}

fn usage_color(percent: f64) -> Color {
    if percent >= 90.0 {
        Color::Red
    } else if percent >= 75.0 {
        Color::Yellow
    } else {
        Color::Green
    }
}

fn usage_bar(percent: f64) -> String {
    const WIDTH: usize = 10;
    let filled = ((percent / 100.0) * WIDTH as f64).round() as usize;
    let empty = WIDTH.saturating_sub(filled);
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_disk(mount: &str, total: u64, used: u64) -> DiskUsage {
        let available = total.saturating_sub(used);
        let use_percent = if total > 0 {
            (used as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        DiskUsage {
            filesystem: "/dev/sda1".to_string(),
            mount_point: mount.to_string(),
            fs_type: "ext4".to_string(),
            total,
            used,
            available,
            use_percent,
        }
    }

    #[test]
    fn format_table_empty() {
        let disks: Vec<DiskUsage> = vec![];
        let output = format_table(&disks);
        assert!(output.contains("No filesystems"));
    }

    #[test]
    fn format_table_single() {
        let disks = vec![make_disk("/", 1024 * 1024 * 1024, 512 * 1024 * 1024)];
        let output = format_table(&disks);
        assert!(output.contains("Filesystem"));
        assert!(output.contains("/dev/sda1"));
    }

    #[test]
    fn format_json_works() {
        let disks = vec![make_disk("/", 1000, 500)];
        let output = format_json(&disks);
        assert!(output.contains("mount_point"));
        assert!(output.contains("ext4"));
    }

    #[test]
    fn usage_color_green() {
        let color = usage_color(50.0);
        assert!(matches!(color, Color::Green));
    }

    #[test]
    fn usage_color_yellow() {
        let color = usage_color(80.0);
        assert!(matches!(color, Color::Yellow));
    }

    #[test]
    fn usage_color_red() {
        let color = usage_color(95.0);
        assert!(matches!(color, Color::Red));
    }

    #[test]
    fn usage_bar_empty() {
        let bar = usage_bar(0.0);
        assert!(bar.contains("░░░░░░░░░░"));
    }

    #[test]
    fn usage_bar_full() {
        let bar = usage_bar(100.0);
        assert!(bar.contains("██████████"));
    }

    #[test]
    fn usage_bar_half() {
        let bar = usage_bar(50.0);
        assert!(bar.contains("█████"));
        assert!(bar.contains("░░░░░"));
    }
}
