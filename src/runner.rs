use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Local};

use crate::cli::{RunArgs, RunFlags};
use crate::config::{user_facing_path, Config};
use crate::log;
use crate::notify::{self, NotifyMessage};

#[derive(Debug, Clone)]
pub struct RunOptions {
    pub name: Option<String>,
    pub command: Vec<String>,
    pub detach: bool,
    pub only_fail: Option<bool>,
    pub tail_lines: Option<usize>,
    pub no_notify: bool,
    pub config_path: Option<PathBuf>,
    pub internal_run: bool,
    pub log_path: Option<PathBuf>,
}

impl RunOptions {
    pub fn from_args(args: RunArgs, config_path: Option<PathBuf>, internal_run: bool) -> Self {
        Self::from_flags(args.flags, args.command, config_path, internal_run)
    }

    pub fn from_flags(
        flags: RunFlags,
        command: Vec<String>,
        config_path: Option<PathBuf>,
        internal_run: bool,
    ) -> Self {
        Self {
            name: flags.name,
            command,
            detach: flags.detach,
            only_fail: flags.only_fail.then_some(true),
            tail_lines: flags.tail_lines,
            no_notify: flags.no_notify,
            config_path,
            internal_run,
            log_path: flags.log_path,
        }
    }
}

pub struct RunResult {
    pub name: Option<String>,
    pub command: String,
    pub exit_code: i32,
    pub success: bool,
    pub started_at: DateTime<Local>,
    pub finished_at: DateTime<Local>,
    pub duration_seconds: u64,
    pub host: String,
    pub log_path: PathBuf,
    pub tail_lines: usize,
    pub tail: String,
}

pub fn run_command(opts: RunOptions, config: &Config, config_path: &Path) -> Result<i32> {
    if opts.command.is_empty() {
        bail!("No command provided");
    }

    let started_at = Local::now();
    let host = hostname();
    let command_string = opts.command.join(" ");
    let log_path = match &opts.log_path {
        Some(path) => path.clone(),
        None => {
            let log_dir = log::resolve_log_dir(config)?;
            log::make_log_path(&log_dir, started_at, opts.name.as_deref(), &opts.command)
        }
    };

    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create log directory {}", parent.display()))?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .with_context(|| format!("failed to open log {}", log_path.display()))?;
    log::write_header(
        &mut file,
        &command_string,
        opts.name.as_deref(),
        started_at,
        &host,
        &log_path,
    )?;

    let log_file = Arc::new(Mutex::new(file));
    let mut child = build_command(&opts.command, config.run.shell)
        .stdin(if opts.internal_run {
            Stdio::null()
        } else {
            Stdio::inherit()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to start command: {command_string}"))?;

    let stdout = child.stdout.take().context("failed to capture stdout")?;
    let stderr = child.stderr.take().context("failed to capture stderr")?;
    let stdout_log = Arc::clone(&log_file);
    let stderr_log = Arc::clone(&log_file);

    let out_handle = thread::spawn(move || tee_reader(stdout, stdout_log, StreamKind::Stdout));
    let err_handle = thread::spawn(move || tee_reader(stderr, stderr_log, StreamKind::Stderr));

    let status = child.wait().context("failed to wait for child process")?;
    out_handle
        .join()
        .map_err(|_| anyhow::anyhow!("stdout thread panicked"))??;
    err_handle
        .join()
        .map_err(|_| anyhow::anyhow!("stderr thread panicked"))??;

    let finished_at = Local::now();
    let duration_seconds = (finished_at - started_at).num_seconds().max(0) as u64;
    let abnormal = status.code().is_none();
    let exit_code = status.code().unwrap_or(1);

    {
        let mut file = log_file
            .lock()
            .map_err(|_| anyhow::anyhow!("log file lock poisoned"))?;
        log::write_footer(
            &mut file,
            finished_at,
            exit_code,
            duration_seconds,
            abnormal,
        )?;
    }

    let tail_lines = opts.tail_lines.unwrap_or(config.log.tail_lines);
    let tail = log::tail_lines(&log_path, tail_lines).unwrap_or_default();
    let result = RunResult {
        name: opts.name.clone(),
        command: command_string,
        exit_code,
        success: exit_code == 0,
        started_at,
        finished_at,
        duration_seconds,
        host,
        log_path,
        tail_lines,
        tail,
    };

    maybe_notify(&opts, config, config_path, &result);
    Ok(exit_code)
}

fn maybe_notify(opts: &RunOptions, config: &Config, config_path: &Path, result: &RunResult) {
    if opts.no_notify {
        return;
    }

    let only_fail = opts.only_fail.unwrap_or(config.notify.only_fail);
    if only_fail && result.success {
        return;
    }

    if !config.notify.enabled {
        eprintln!("Notification disabled by config.");
        return;
    }

    if !config.notify.targets.iter().any(|target| target.enabled()) {
        eprintln!("No enabled notification targets. Skipped notification.");
        eprintln!(
            "Edit {} to enable one.",
            user_facing_path(
                &opts
                    .config_path
                    .clone()
                    .unwrap_or_else(|| config_path.to_path_buf())
            )
        );
        return;
    }

    let msg = NotifyMessage::from_run_result(result);
    let errors = notify::send_all(&config.notify, &msg);
    if !errors.is_empty() {
        eprintln!("Some notifications failed:");
        for (target, err) in errors {
            eprintln!("- {target}: {err:#}");
        }
    }
}

fn build_command(args: &[String], shell: bool) -> Command {
    if shell {
        let mut command = Command::new("sh");
        command.arg("-c").arg(args.join(" "));
        command
    } else {
        let mut command = Command::new(&args[0]);
        command.args(&args[1..]);
        command
    }
}

enum StreamKind {
    Stdout,
    Stderr,
}

fn tee_reader<R: Read>(
    mut reader: R,
    log_file: Arc<Mutex<fs::File>>,
    kind: StreamKind,
) -> Result<()> {
    let mut buf = [0_u8; 8192];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        match kind {
            StreamKind::Stdout => {
                let mut out = std::io::stdout().lock();
                out.write_all(&buf[..n])?;
                out.flush()?;
            }
            StreamKind::Stderr => {
                let mut err = std::io::stderr().lock();
                err.write_all(&buf[..n])?;
                err.flush()?;
            }
        }
        let mut file = log_file
            .lock()
            .map_err(|_| anyhow::anyhow!("log file lock poisoned"))?;
        file.write_all(&buf[..n])?;
        file.flush()?;
    }
    Ok(())
}

fn hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            fs::read_to_string("/etc/hostname")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| "unknown-host".to_string())
}

impl NotifyMessage {
    pub fn from_run_result(result: &RunResult) -> Self {
        let action = if result.success { "finished" } else { "failed" };
        let icon = if result.success { "✅" } else { "❌" };
        let title = if let Some(name) = &result.name {
            format!("{icon} {name} {action} on {}", result.host)
        } else {
            format!("{icon} Command {action} on {}", result.host)
        };
        let display_name = result.name.as_deref().unwrap_or("-");
        let body = format!(
            "Name:\n{display_name}\n\nCommand:\n{}\n\nExit code:\n{}\n\nDuration:\n{}s\n\nStarted at:\n{}\n\nFinished at:\n{}\n\nHost:\n{}\n\nLog:\n{}\n\nLast {} lines:\n{}",
            result.command,
            result.exit_code,
            result.duration_seconds,
            result.started_at.format("%Y-%m-%d %H:%M:%S"),
            result.finished_at.format("%Y-%m-%d %H:%M:%S"),
            result.host,
            result.log_path.display(),
            result.tail_lines,
            result.tail
        );

        Self {
            title,
            body,
            success: result.success,
            exit_code: result.exit_code,
            command: result.command.clone(),
            host: result.host.clone(),
            duration_seconds: result.duration_seconds,
            log_path: result.log_path.clone(),
        }
    }
}
