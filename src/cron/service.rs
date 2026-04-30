use anyhow::{Context, Result};
use chrono::{Datelike, Local, Timelike};
use std::process::Command;

use super::types::{CronJob, Schedule, HU_MARKER};

/// Minutes to add to current time for scheduling
const TIME_OFFSET_MINUTES: u32 = 5;

/// Read the current user's crontab
pub fn read_crontab() -> Result<String> {
    let output = Command::new("crontab")
        .arg("-l")
        .output()
        .context("Failed to execute crontab -l")?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        // No crontab for user is not an error
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("no crontab") {
            Ok(String::new())
        } else {
            anyhow::bail!("crontab -l failed: {}", stderr.trim());
        }
    }
}

/// Write a new crontab
pub fn write_crontab(content: &str) -> Result<()> {
    use std::io::Write;
    use std::process::Stdio;

    let mut child = Command::new("crontab")
        .arg("-")
        .stdin(Stdio::piped())
        .spawn()
        .context("Failed to spawn crontab")?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(content.as_bytes())
            .context("Failed to write to crontab stdin")?;
    }

    let status = child.wait().context("Failed to wait for crontab")?;
    if !status.success() {
        anyhow::bail!("crontab failed with status: {}", status);
    }

    Ok(())
}

/// Parse crontab content into jobs
pub fn parse_crontab(content: &str) -> Vec<CronJob> {
    let mut jobs = Vec::new();
    let mut pending_marker: Option<String> = None;

    for line in content.lines() {
        let trimmed = line.trim();

        // Check for hu marker comment
        if let Some(stripped) = trimmed.strip_prefix(HU_MARKER) {
            pending_marker = Some(stripped.trim().to_string());
            continue;
        }

        // Skip empty lines and other comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            pending_marker = None;
            continue;
        }

        // Parse cron line
        if let Some(job) = parse_cron_line(trimmed, pending_marker.take()) {
            jobs.push(job);
        }
    }

    jobs
}

/// Parse a single cron line
fn parse_cron_line(line: &str, marker: Option<String>) -> Option<CronJob> {
    // Handle @reboot
    if let Some(stripped) = line.strip_prefix("@reboot") {
        let command = stripped.trim().to_string();
        return Some(CronJob {
            expression: "@reboot".to_string(),
            command,
            schedule_name: marker.clone(),
            is_hu_job: marker.is_some(),
        });
    }

    // Standard cron: min hour dom mon dow command
    let parts: Vec<&str> = line.splitn(6, char::is_whitespace).collect();
    if parts.len() < 6 {
        return None;
    }

    let expression = parts[..5].join(" ");
    let command = parts[5].trim().to_string();

    Some(CronJob {
        expression,
        command,
        schedule_name: marker.clone(),
        is_hu_job: marker.is_some(),
    })
}

/// Get the scheduled time (now + offset)
pub fn get_schedule_time() -> (u32, u32, u32, u32) {
    let now = Local::now();
    let minute = (now.minute() + TIME_OFFSET_MINUTES) % 60;
    let hour = if now.minute() + TIME_OFFSET_MINUTES >= 60 {
        (now.hour() + 1) % 24
    } else {
        now.hour()
    };
    let day_of_month = now.day();
    let day_of_week = now.weekday().num_days_from_sunday();

    (minute, hour, day_of_month, day_of_week)
}

/// Add a new cron job
pub fn add_job(schedule: Schedule, command: &str) -> Result<CronJob> {
    let (minute, hour, day_of_month, day_of_week) = get_schedule_time();
    let expression = schedule.to_cron(minute, hour, day_of_month, day_of_week);

    let job = CronJob {
        expression: expression.clone(),
        command: command.to_string(),
        schedule_name: Some(schedule.display_name().to_string()),
        is_hu_job: true,
    };

    // Read existing crontab
    let mut crontab = read_crontab()?;

    // Ensure trailing newline
    if !crontab.is_empty() && !crontab.ends_with('\n') {
        crontab.push('\n');
    }

    // Add marker and job
    crontab.push_str(&format!("{} {}\n", HU_MARKER, schedule.display_name()));
    crontab.push_str(&format!("{} {}\n", expression, command));

    // Write back
    write_crontab(&crontab)?;

    Ok(job)
}

/// List all cron jobs
pub fn list_jobs(hu_only: bool) -> Result<Vec<CronJob>> {
    let crontab = read_crontab()?;
    let jobs = parse_crontab(&crontab);

    if hu_only {
        Ok(jobs.into_iter().filter(|j| j.is_hu_job).collect())
    } else {
        Ok(jobs)
    }
}

/// Remove jobs matching a pattern
pub fn remove_jobs(pattern: &str) -> Result<Vec<CronJob>> {
    let crontab = read_crontab()?;
    let jobs = parse_crontab(&crontab);

    let (to_remove, to_keep): (Vec<_>, Vec<_>) = jobs.into_iter().partition(|j| j.matches(pattern));

    if to_remove.is_empty() {
        return Ok(vec![]);
    }

    // Rebuild crontab without removed jobs
    let mut new_crontab = String::new();
    for job in &to_keep {
        if job.is_hu_job {
            if let Some(ref name) = job.schedule_name {
                new_crontab.push_str(&format!("{} {}\n", HU_MARKER, name));
            }
        }
        new_crontab.push_str(&format!("{} {}\n", job.expression, job.command));
    }

    write_crontab(&new_crontab)?;

    Ok(to_remove)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_crontab_empty() {
        let jobs = parse_crontab("");
        assert!(jobs.is_empty());
    }

    #[test]
    fn parse_crontab_single_job() {
        let content = "35 18 * * * echo hello";
        let jobs = parse_crontab(content);
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].expression, "35 18 * * *");
        assert_eq!(jobs[0].command, "echo hello");
        assert!(!jobs[0].is_hu_job);
    }

    #[test]
    fn parse_crontab_with_hu_marker() {
        let content = "# hu: daily\n35 18 * * * hu gh sync ~/docs";
        let jobs = parse_crontab(content);
        assert_eq!(jobs.len(), 1);
        assert!(jobs[0].is_hu_job);
        assert_eq!(jobs[0].schedule_name, Some("daily".to_string()));
    }

    #[test]
    fn parse_crontab_multiple_jobs() {
        let content = "0 * * * * job1\n30 12 * * * job2\n# hu: weekly\n0 9 * * 1 job3";
        let jobs = parse_crontab(content);
        assert_eq!(jobs.len(), 3);
        assert!(!jobs[0].is_hu_job);
        assert!(!jobs[1].is_hu_job);
        assert!(jobs[2].is_hu_job);
    }

    #[test]
    fn parse_crontab_skips_comments() {
        let content = "# This is a comment\n35 18 * * * echo hello\n# Another comment";
        let jobs = parse_crontab(content);
        assert_eq!(jobs.len(), 1);
    }

    #[test]
    fn parse_crontab_reboot() {
        let content = "@reboot /path/to/script.sh";
        let jobs = parse_crontab(content);
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].expression, "@reboot");
        assert_eq!(jobs[0].command, "/path/to/script.sh");
    }

    #[test]
    fn parse_crontab_reboot_with_marker() {
        let content = "# hu: reboot\n@reboot hu gh sync ~/docs";
        let jobs = parse_crontab(content);
        assert_eq!(jobs.len(), 1);
        assert!(jobs[0].is_hu_job);
        assert_eq!(jobs[0].schedule_name, Some("reboot".to_string()));
    }

    #[test]
    fn parse_cron_line_valid() {
        let job = parse_cron_line("35 18 * * * echo test", None).unwrap();
        assert_eq!(job.expression, "35 18 * * *");
        assert_eq!(job.command, "echo test");
    }

    #[test]
    fn parse_cron_line_with_marker() {
        let job = parse_cron_line("35 18 * * * echo test", Some("daily".to_string())).unwrap();
        assert!(job.is_hu_job);
        assert_eq!(job.schedule_name, Some("daily".to_string()));
    }

    #[test]
    fn parse_cron_line_invalid() {
        let job = parse_cron_line("invalid", None);
        assert!(job.is_none());
    }

    #[test]
    fn parse_cron_line_too_short() {
        let job = parse_cron_line("* * * *", None);
        assert!(job.is_none());
    }

    #[test]
    fn get_schedule_time_returns_values() {
        let (minute, hour, dom, dow) = get_schedule_time();
        assert!(minute < 60);
        assert!(hour < 24);
        assert!((1..=31).contains(&dom));
        assert!(dow < 7);
    }

    #[test]
    fn time_offset_is_five() {
        assert_eq!(TIME_OFFSET_MINUTES, 5);
    }

    #[test]
    fn hu_marker_format() {
        assert!(HU_MARKER.starts_with('#'));
        assert!(HU_MARKER.contains("hu"));
    }

    #[test]
    fn parse_crontab_empty_lines() {
        let content = "\n\n35 18 * * * echo hello\n\n";
        let jobs = parse_crontab(content);
        assert_eq!(jobs.len(), 1);
    }

    #[test]
    fn parse_crontab_marker_without_job() {
        // Marker followed by comment should not create a job
        let content = "# hu: daily\n# some comment\n35 18 * * * echo hello";
        let jobs = parse_crontab(content);
        assert_eq!(jobs.len(), 1);
        // The marker was consumed by the comment, so this job is not hu-managed
        assert!(!jobs[0].is_hu_job);
    }

    #[test]
    fn parse_cron_line_command_with_spaces() {
        let job = parse_cron_line("0 0 * * * /bin/bash -c 'echo hello world'", None).unwrap();
        assert_eq!(job.command, "/bin/bash -c 'echo hello world'");
    }

    #[test]
    fn parse_crontab_preserves_command_args() {
        let content = "35 18 * * * hu gh sync ~/Projects/docs --pull";
        let jobs = parse_crontab(content);
        assert_eq!(jobs.len(), 1);
        assert!(jobs[0].command.contains("--pull"));
    }
}
