mod cli;
mod config;
mod detach;
mod log;
mod notify;
mod runner;

use std::process::ExitCode;

use anyhow::{bail, Result};
use clap::Parser;

use crate::cli::{Cli, Commands};
use crate::config::{load_or_create_config, user_facing_path};
use crate::runner::RunOptions;

fn main() -> ExitCode {
    match try_main() {
        Ok(code) => ExitCode::from(code as u8),
        Err(err) => {
            eprintln!("Error: {err:#}");
            ExitCode::from(1)
        }
    }
}

fn try_main() -> Result<i32> {
    let cli = Cli::parse();
    let (config, config_path, created) = load_or_create_config(cli.config.clone())?;

    if created {
        eprintln!(
            "Created default config at {}",
            user_facing_path(&config_path)
        );
        eprintln!("Please edit it to enable notification channels.");
    }

    match cli.command {
        Some(Commands::Run(args)) => {
            let opts = RunOptions::from_args(args, cli.config, false);
            run_or_detach(opts, &config, &config_path)
        }
        Some(Commands::Logs(args)) => {
            log::print_recent_logs(&config, args.n)?;
            Ok(0)
        }
        Some(Commands::Test(args)) => notify::run_test(&config, &config_path, &args),
        Some(Commands::External(command)) => {
            let opts = RunOptions::from_flags(cli.run, command, cli.config, cli.internal_run);
            run_or_detach(opts, &config, &config_path)
        }
        None => {
            bail!(
                "No command provided. Use `nohupx -- <COMMAND>...` or `nohupx run -- <COMMAND>...`"
            );
        }
    }
}

fn run_or_detach(
    mut opts: RunOptions,
    config: &config::Config,
    config_path: &std::path::Path,
) -> Result<i32> {
    if !opts.internal_run && !opts.detach && config.run.default_detach {
        opts.detach = true;
    }

    if opts.detach && !opts.internal_run {
        let log_path = detach::start_detached(&opts, config, config_path)?;
        println!("Started detached job.");
        println!("Log: {}", log_path.display());
        return Ok(0);
    }

    runner::run_command(opts, config, config_path)
}
