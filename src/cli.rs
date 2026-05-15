use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "nohupx", version, about = "nohup enhanced with notifications")]
pub struct Cli {
    #[arg(long, global = true, value_name = "PATH")]
    pub config: Option<PathBuf>,

    #[arg(long, hide = true)]
    pub internal_run: bool,

    #[command(flatten)]
    pub run: RunFlags,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Run(RunArgs),
    Logs(LogsArgs),
    Test(TestArgs),
    #[command(external_subcommand)]
    External(Vec<String>),
}

#[derive(Debug, Clone, Args, Default)]
pub struct RunFlags {
    #[arg(short = 'd', long)]
    pub detach: bool,

    #[arg(long, value_name = "NAME")]
    pub name: Option<String>,

    #[arg(long)]
    pub only_fail: bool,

    #[arg(long, value_name = "N")]
    pub tail_lines: Option<usize>,

    #[arg(long)]
    pub no_notify: bool,

    #[arg(long, hide = true, value_name = "PATH")]
    pub log_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Args, Default)]
pub struct RunArgs {
    #[command(flatten)]
    pub flags: RunFlags,

    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub command: Vec<String>,
}

#[derive(Debug, Clone, Args)]
pub struct LogsArgs {
    #[arg(short = 'n', default_value_t = 10)]
    pub n: usize,
}

#[derive(Debug, Clone, Args)]
pub struct TestArgs {
    pub channel: String,

    #[arg(long)]
    pub include_disabled: bool,
}
