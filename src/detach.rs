use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result};
use chrono::Local;

use crate::config::Config;
use crate::log;
use crate::runner::RunOptions;

pub fn start_detached(opts: &RunOptions, config: &Config, config_path: &Path) -> Result<PathBuf> {
    let started_at = Local::now();
    let log_dir = log::resolve_log_dir(config)?;
    fs::create_dir_all(&log_dir)
        .with_context(|| format!("failed to create log directory {}", log_dir.display()))?;
    let log_path = opts.log_path.clone().unwrap_or_else(|| {
        log::make_log_path(&log_dir, started_at, opts.name.as_deref(), &opts.command)
    });

    let exe = std::env::current_exe().context("failed to get current executable path")?;
    let mut command = Command::new(exe);
    command
        .arg("--config")
        .arg(config_path)
        .arg("--internal-run")
        .arg("--log-path")
        .arg(&log_path);

    if let Some(name) = &opts.name {
        command.arg("--name").arg(name);
    }
    if opts.only_fail == Some(true) {
        command.arg("--only-fail");
    }
    if let Some(tail_lines) = opts.tail_lines {
        command.arg("--tail-lines").arg(tail_lines.to_string());
    }
    if opts.no_notify {
        command.arg("--no-notify");
    }
    command.arg("--").args(&opts.command);

    let null_out = OpenOptions::new()
        .write(true)
        .open(if cfg!(windows) { "NUL" } else { "/dev/null" })
        .context("failed to open null device")?;
    let null_err = null_out
        .try_clone()
        .context("failed to clone null device")?;

    command
        .stdin(Stdio::null())
        .stdout(Stdio::from(null_out))
        .stderr(Stdio::from(null_err))
        .spawn()
        .context("failed to start detached nohupx process")?;

    Ok(log_path)
}
