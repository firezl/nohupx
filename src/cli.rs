use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

const ROOT_AFTER_HELP: &str = r#"Examples:
  nohupx -- python train.py
  nohupx run -- python train.py
  nohupx -d -- python train.py
  nohupx --only-fail -- make build
  nohupx --name exp01 -d -- python run_exp.py
  nohupx logs
  nohupx test email
  nohupx secret set telegram/main
  nohupx test all

Config:
  If no config exists, nohupx creates ~/.config/nohupx/config.toml automatically.
  Logs are written to ~/.local/state/nohupx/logs by default.
"#;

const RUN_AFTER_HELP: &str = r#"Examples:
  nohupx run -- python train.py
  nohupx run -d -- sh -c "sleep 5; echo done"
  nohupx run --name exp01 --tail-lines 120 -- python run_exp.py

The command after -- is executed directly by default. Set shell = true in config.toml
to run the joined command through sh -c.
"#;

const LOGS_AFTER_HELP: &str = r#"Examples:
  nohupx logs
  nohupx logs -n 20
"#;

const TEST_AFTER_HELP: &str = r#"Examples:
  nohupx test email
  nohupx test example-email
  nohupx test slack
  nohupx test ntfy
  nohupx test all
  nohupx test example-email --include-disabled

CHANNEL can be a target name, a target type, or all.
Supported target types include:
  email, webhook, feishu, wecom, dingtalk, slack, discord, ntfy, telegram
"#;

const SECRET_AFTER_HELP: &str = r#"Examples:
  nohupx secret set telegram/main
  nohupx secret set email/password --value 'smtp-auth-code'
  nohupx secret get telegram/main --show
  nohupx secret list
  nohupx secret delete telegram/main

Use *_secret fields in config.toml to reference saved secrets:
  bot_token_secret = "telegram/main"
  password_secret = "email/password"
  webhook_secret = "slack/lab"
"#;

#[derive(Debug, Parser)]
#[command(
    name = "nohupx",
    version,
    about = "Run long commands like nohup and notify when they finish",
    long_about = "nohupx is a lightweight nohup-like command runner. It saves complete stdout/stderr logs, records exit status and duration, and sends completion notifications through configured channels.",
    override_usage = "nohupx [OPTIONS] -- <COMMAND>...\n       nohupx run [OPTIONS] -- <COMMAND>...\n       nohupx logs [OPTIONS]\n       nohupx test <CHANNEL> [OPTIONS]\n       nohupx secret <COMMAND>",
    after_help = ROOT_AFTER_HELP
)]
pub struct Cli {
    #[arg(
        long,
        global = true,
        value_name = "PATH",
        help = "Use a custom config file",
        long_help = "Use a custom config file instead of ~/.config/nohupx/config.toml"
    )]
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
    #[command(
        about = "Run a command and notify when it finishes",
        long_about = "Run a command, tee stdout/stderr to the terminal and log file, then send completion notifications according to config.toml.",
        override_usage = "nohupx run [OPTIONS] -- <COMMAND>...",
        after_help = RUN_AFTER_HELP
    )]
    Run(RunArgs),
    #[command(
        about = "List recent log files",
        long_about = "List recent nohupx log files from the configured log directory.",
        override_usage = "nohupx logs [OPTIONS]",
        after_help = LOGS_AFTER_HELP
    )]
    Logs(LogsArgs),
    #[command(
        about = "Send a test notification",
        long_about = "Send a test notification to targets matched by name, type, or all.",
        override_usage = "nohupx test <CHANNEL> [OPTIONS]",
        after_help = TEST_AFTER_HELP
    )]
    Test(TestArgs),
    #[command(
        about = "Manage secrets in the system keyring",
        long_about = "Save, read, list, and delete nohupx secrets in the system keyring. Config files can reference these values with *_secret fields.",
        after_help = SECRET_AFTER_HELP
    )]
    Secret(SecretArgs),
    #[command(external_subcommand)]
    External(Vec<String>),
}

#[derive(Debug, Clone, Args, Default)]
pub struct RunFlags {
    #[arg(
        short = 'd',
        long,
        help = "Start nohupx in the background and return immediately"
    )]
    pub detach: bool,

    #[arg(
        long,
        value_name = "NAME",
        help = "Set the task name used in log file names and notification titles"
    )]
    pub name: Option<String>,

    #[arg(long, help = "Send notification only when the command fails")]
    pub only_fail: bool,

    #[arg(
        long,
        value_name = "N",
        help = "Include the last N log lines in notifications"
    )]
    pub tail_lines: Option<usize>,

    #[arg(long, help = "Disable notifications for this run")]
    pub no_notify: bool,

    #[arg(long, hide = true, value_name = "PATH")]
    pub log_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Args, Default)]
pub struct RunArgs {
    #[command(flatten)]
    pub flags: RunFlags,

    #[arg(
        trailing_var_arg = true,
        allow_hyphen_values = true,
        required = true,
        num_args = 1..,
        value_name = "COMMAND",
        help = "Command and arguments to run"
    )]
    pub command: Vec<String>,
}

#[derive(Debug, Clone, Args)]
pub struct LogsArgs {
    #[arg(
        short = 'n',
        value_name = "N",
        default_value_t = 10,
        help = "Number of recent log files to show"
    )]
    pub n: usize,
}

#[derive(Debug, Clone, Args)]
pub struct TestArgs {
    #[arg(
        value_name = "CHANNEL",
        help = "Target name, target type, or all",
        long_help = "Target name, target type such as email/slack/ntfy, or all"
    )]
    pub channel: String,

    #[arg(long, help = "Allow testing disabled targets")]
    pub include_disabled: bool,
}

#[derive(Debug, Clone, Args)]
pub struct SecretArgs {
    #[command(subcommand)]
    pub command: SecretCommand,
}

#[derive(Debug, Clone, Subcommand)]
pub enum SecretCommand {
    #[command(about = "Save a secret in the system keyring")]
    Set(SecretSetArgs),
    #[command(about = "Read a secret from the system keyring")]
    Get(SecretGetArgs),
    #[command(about = "Delete a secret from the system keyring")]
    Delete(SecretDeleteArgs),
    #[command(about = "List known nohupx secret keys")]
    List,
}

#[derive(Debug, Clone, Args)]
pub struct SecretSetArgs {
    #[arg(value_name = "KEY", help = "Secret key, for example telegram/main")]
    pub key: String,

    #[arg(
        long,
        value_name = "VALUE",
        help = "Secret value; omit to type it hidden"
    )]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub struct SecretGetArgs {
    #[arg(value_name = "KEY", help = "Secret key to read")]
    pub key: String,

    #[arg(long, help = "Print the secret value to stdout")]
    pub show: bool,
}

#[derive(Debug, Clone, Args)]
pub struct SecretDeleteArgs {
    #[arg(value_name = "KEY", help = "Secret key to delete")]
    pub key: String,
}
