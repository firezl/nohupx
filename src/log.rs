use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Local};

use crate::config::{expand_path, Config};

pub fn resolve_log_dir(config: &Config) -> Result<PathBuf> {
    expand_path(&config.log.dir)
}

pub fn make_log_path(
    log_dir: &Path,
    started_at: DateTime<Local>,
    name: Option<&str>,
    command: &[String],
) -> PathBuf {
    let raw = name.map(ToOwned::to_owned).unwrap_or_else(|| {
        command
            .first()
            .cloned()
            .unwrap_or_else(|| "command".to_string())
    });
    let safe = sanitize_filename_part(&raw);
    let stamp = started_at.format("%Y%m%d-%H%M%S");
    log_dir.join(format!("{stamp}-{safe}.log"))
}

pub fn sanitize_filename_part(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            out.push(ch);
        } else {
            out.push('_');
        }
        if out.len() >= 80 {
            break;
        }
    }
    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        "command".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn tail_lines(path: &Path, n: usize) -> Result<String> {
    if n == 0 {
        return Ok(String::new());
    }
    let file =
        fs::File::open(path).with_context(|| format!("failed to open log {}", path.display()))?;
    let reader = io::BufReader::new(file);
    let mut lines = Vec::new();
    for line in reader.lines() {
        lines.push(line?);
        if lines.len() > n {
            lines.remove(0);
        }
    }
    Ok(lines.join("\n"))
}

pub fn print_recent_logs(config: &Config, n: usize) -> Result<()> {
    let dir = resolve_log_dir(config)?;
    fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create log directory {}", dir.display()))?;
    let mut entries = Vec::new();
    for entry in fs::read_dir(&dir).with_context(|| format!("failed to read {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let modified = entry.metadata()?.modified().ok();
        entries.push((modified, path));
    }
    entries.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));
    for (_modified, path) in entries.into_iter().take(n) {
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy())
            .unwrap_or_default();
        println!("{name}\t{}", path.display());
    }
    Ok(())
}

pub fn write_header(
    file: &mut fs::File,
    command: &str,
    name: Option<&str>,
    started_at: DateTime<Local>,
    host: &str,
    log_path: &Path,
) -> Result<()> {
    writeln!(file, "$ {command}")?;
    if let Some(name) = name {
        writeln!(file, "Name: {name}")?;
    }
    writeln!(
        file,
        "Started at: {}",
        started_at.format("%Y-%m-%d %H:%M:%S")
    )?;
    writeln!(file, "Host: {host}")?;
    writeln!(file, "Log: {}", log_path.display())?;
    writeln!(
        file,
        "--------------------------------------------------------------------------------"
    )?;
    file.flush()?;
    Ok(())
}

pub fn write_footer(
    file: &mut fs::File,
    finished_at: DateTime<Local>,
    exit_code: i32,
    duration_seconds: u64,
    abnormal: bool,
) -> Result<()> {
    writeln!(
        file,
        "\n--------------------------------------------------------------------------------"
    )?;
    writeln!(
        file,
        "Finished at: {}",
        finished_at.format("%Y-%m-%d %H:%M:%S")
    )?;
    writeln!(file, "Exit code: {exit_code}")?;
    if abnormal {
        writeln!(file, "Status: terminated abnormally")?;
    }
    writeln!(file, "Duration: {duration_seconds}s")?;
    file.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn tail_keeps_last_n_lines() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        fs::write(tmp.path(), "a\nb\nc\nd\n").unwrap();
        assert_eq!(tail_lines(tmp.path(), 2).unwrap(), "c\nd");
    }

    #[test]
    fn log_path_uses_sanitized_name() {
        let dir = PathBuf::from("/tmp/logs");
        let started = Local.with_ymd_and_hms(2026, 5, 15, 19, 30, 1).unwrap();
        let path = make_log_path(&dir, started, Some("train resnet!*"), &[]);
        assert!(path.ends_with("20260515-193001-train_resnet.log"));
    }
}
