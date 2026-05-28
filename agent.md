# nohupx Agent Notes

This document is a handoff guide for future coding agents or maintainers working on `nohupx`.

## Project Summary

`nohupx` is a lightweight Rust CLI tool: "nohup enhanced".

It runs long commands like `nohup`, saves complete logs, captures the exit code, duration, host, command, and log tail, then sends completion notifications through configured channels.

Core usage:

```bash
nohupx -- python train.py
nohupx run -- python train.py
nohupx -d -- python train.py
nohupx --only-fail -- make build
nohupx --name exp01 -d -- python run_exp.py
nohupx logs
nohupx test email
nohupx test all
nohupx secret set telegram/main
```

Important product constraints:

- Single binary.
- No Python dependency.
- No Apprise dependency.
- No Web UI.
- No daemon/scheduler/database/plugin system.
- No `nohupx init`; config is created automatically on first run.
- First version should stay lightweight, stable, and maintainable.

## Current Stack

Rust stable, edition 2021.

Main dependencies:

- `anyhow`
- `clap` with derive
- `chrono`
- `serde` with derive
- `toml`
- `shellexpand`
- `dirs`
- `reqwest` blocking/json/rustls
- `serde_json`
- `lettre`
- `keyring-core`
- platform keyring stores:
  - Linux: `linux-keyutils-keyring-store`
  - macOS: `apple-native-keyring-store`
  - Windows: `windows-native-keyring-store`
- `rpassword`

## Repository Structure

```text
nohupx/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── README.zh-CN.md
├── agent.md
├── scripts/
│   └── install.sh
├── .github/
│   └── workflows/
│       └── release.yml
└── src/
    ├── main.rs
    ├── cli.rs
    ├── config.rs
    ├── runner.rs
    ├── log.rs
    ├── detach.rs
    ├── secret.rs
    └── notify/
        ├── mod.rs
        ├── email.rs
        ├── webhook.rs
        ├── feishu.rs
        ├── wecom.rs
        ├── dingtalk.rs
        ├── slack.rs
        ├── discord.rs
        ├── ntfy.rs
        └── telegram.rs
```

## Default Paths

Config:

```text
~/.config/nohupx/config.toml
```

Logs:

```text
~/.local/state/nohupx/logs
```

Environment variable prefix:

```text
NOHUPX_
```

Examples:

```text
NOHUPX_SMTP_PASSWORD
NOHUPX_TELEGRAM_BOT_TOKEN
NOHUPX_PROXY
```

## CLI Overview

Supported command forms:

```bash
nohupx [OPTIONS] -- <COMMAND>...
nohupx run [OPTIONS] -- <COMMAND>...
nohupx logs [OPTIONS]
nohupx test <CHANNEL> [OPTIONS]
nohupx secret <COMMAND>
```

Run options:

- `-d, --detach`
- `--name <NAME>`
- `--only-fail`
- `--tail-lines <N>`
- `--no-notify`
- `--config <PATH>`

Logs:

- `nohupx logs`
- `nohupx logs -n 20`

Test notification:

- `nohupx test email`
- `nohupx test example-email`
- `nohupx test all`
- `nohupx test example-email --include-disabled`

Secrets:

- `nohupx secret set <KEY>`
- `nohupx secret set <KEY> --value <VALUE>`
- `nohupx secret get <KEY>`
- `nohupx secret get <KEY> --show`
- `nohupx secret list`
- `nohupx secret delete <KEY>`

## Config Model

Top-level config sections:

```toml
[run]
default_detach = false
shell = false

[log]
dir = "~/.local/state/nohupx/logs"
tail_lines = 80

[notify]
enabled = true
only_fail = false
```

Targets are `[[notify.targets]]` entries using serde's internally tagged enum with `type`.

All targets support:

```toml
name = "target-name"
enabled = true
```

If `enabled` is omitted, treat it as `true`.

Default generated targets are all `enabled = false`.

## Notification Channels

Implemented channels:

- `email`
- `webhook`
- `feishu`
- `wecom`
- `dingtalk`
- `slack`
- `discord`
- `ntfy`
- `telegram`

### Email

Uses `lettre`.

Supports:

```toml
smtp_host = "smtp.qq.com"
smtp_port = 465
username = "xxx@qq.com"
password_secret = "email/password" # or password_env / password
from = "xxx@qq.com"
to = ["xxx@qq.com"]
```

SMTP behavior:

- `smtp_port = 465`: SMTPS / implicit TLS
- `smtp_port = 587`: STARTTLS
- other ports: STARTTLS path

Email does not support proxy currently.

### HTTP-like Channels

These use `reqwest::blocking`:

- `webhook`
- `feishu`
- `wecom`
- `dingtalk`
- `slack`
- `discord`
- `ntfy`
- `telegram`

They support per-target proxy:

```toml
proxy = "http://127.0.0.1:7890"
proxy_env = "NOHUPX_PROXY"
```

Proxy deliberately does **not** support `proxy_secret`.

Proxy priority:

```text
proxy_env > proxy
```

## Secret Handling

Secret values can come from three compatible sources:

```text
*_secret > *_env > plaintext field
```

Supported secret fields:

```text
email password:      password_secret / password_env / password
telegram token:      bot_token_secret / bot_token_env / bot_token
webhook-like URL:    webhook_secret / webhook_env / webhook
generic webhook URL: url_secret / url_env / url
ntfy URL:            url_secret / url_env / url
```

System keyring support is implemented in `src/secret.rs`.

Backends:

- Windows Credential Manager
- macOS Keychain
- Linux keyutils

Do not reintroduce `*_file` secret storage unless the product direction changes. The user explicitly rejected file-based secret storage as inconvenient.

## Runner Behavior

`runner::run_command`:

- creates log directory if missing
- creates a timestamped log path
- writes metadata header
- starts child process
- captures stdout and stderr with two reader threads
- tees output to terminal and log file
- waits for child
- appends footer with finish time, exit code, duration
- reads tail lines
- sends notification unless skipped
- returns the child exit code

Important:

- Do not swallow stdout/stderr.
- Do not let notification failure change the command exit code.
- If `status.code()` is `None`, use exit code `1`.

## Detach Behavior

`-d/--detach` is intentionally lightweight:

1. Parent computes log path.
2. Parent launches current executable again with hidden `--internal-run`.
3. Parent redirects child stdio to null.
4. Parent prints:

```text
Started detached job.
Log: <path>
```

5. Parent exits `0`.
6. Internal child runs the actual command and sends notification at the end.

Do not add complex fork/setsid/daemon supervision unless the product scope changes.

## Log Format

Log filename:

```text
YYYYMMDD-HHMMSS-<name-or-command>.log
```

Header:

```text
$ python train.py --epochs 100
Name: train-resnet
Started at: 2026-05-15 19:30:01
Host: lab-server
Log: /home/user/.local/state/nohupx/logs/...
--------------------------------------------------------------------------------
```

Footer:

```text
--------------------------------------------------------------------------------
Finished at: ...
Exit code: ...
Duration: ...s
```

## Notification Matching

`nohupx test <channel>` matching rules:

1. `all`: all enabled targets.
2. Exact `name` match has priority.
3. If no name match, match by `type`.
4. Disabled targets are skipped unless `--include-disabled`.

Exit codes:

- all tests success: `0`
- any failure: `1`
- no match: `1`
- only disabled matches without `--include-disabled`: `1`

## Release Workflow

GitHub Actions workflow:

```text
.github/workflows/release.yml
```

Triggers:

- push tag `v*`
- published GitHub Release
- manual `workflow_dispatch` with tag input

Builds:

- Linux
- macOS
- Windows

Artifacts:

- Linux/macOS: `.tar.gz`
- Windows: `.zip`
- `.sha256` checksum files

The publish job uploads to an existing Release if it exists, or creates it otherwise. It must pass `--repo "$GITHUB_REPOSITORY"` to `gh release ...` because the publish job does not checkout a git repo.

## Install Script

Linux install script:

```text
scripts/install.sh
```

User command:

```bash
curl -fsSL https://raw.githubusercontent.com/firezl/nohupx/main/scripts/install.sh | sh
```

Supports:

```bash
NOHUPX_VERSION=0.1.0
NOHUPX_INSTALL_DIR=/usr/local/bin
NOHUPX_REPO=firezl/nohupx
```

Current Linux installer supports `x86_64-unknown-linux-gnu`.

## Documentation

English README:

```text
README.md
```

Simplified Chinese README:

```text
README.zh-CN.md
```

Keep both in sync when changing user-facing behavior.

Both READMEs currently include:

- install
- release
- quick start
- config example
- secret handling
- proxy handling
- per-channel config help
- usage examples
- notes about detached mode

## Validation Commands

Before finishing changes, run:

```bash
cargo fmt -- --check
cargo check
cargo test
cargo clippy -- -D warnings
```

Current test coverage includes:

- config parsing
- default config generation
- tilde path expansion
- tail lines
- log path generation
- notification target matching
- SMTP 465/587 mode selection
- inline/env secret resolution

## Important Design Decisions

- No `nohupx init`.
- Config is auto-created on first use.
- Targets in default config are disabled.
- No notification target enabled is not an error.
- Notification failures print to stderr but do not alter child exit code.
- `shell = false` is the default.
- `shell = true` uses `sh -c`; this remains Linux-oriented.
- Proxy only applies to HTTP-based notification backends.
- Proxy does not use system keyring.
- Secret file storage was removed by product decision.
- Environment variables remain supported for backward compatibility.
- Plaintext config secrets are explicitly supported for ease of use.

## Common Pitfalls

- `nohupx -- <COMMAND>` must keep working; it is the primary UX.
- `nohupx run -- <COMMAND>` must behave the same as the root run form.
- Do not break clap external-subcommand parsing.
- Do not add async/Tokio unless there is a strong reason.
- Do not turn notification backends into a plugin framework.
- Do not make `nohupx` depend on external notification commands.
- Do not make release workflow depend on a checked-out git repository in the publish job unless a checkout step is added.
