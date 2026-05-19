# nohupx

[简体中文](README.zh-CN.md)

`nohupx` is a lightweight nohup-like command runner with notifications.
It runs long commands, saves complete logs, and sends the final result through configured channels such as email, generic webhooks, Feishu, WeCom, DingTalk, Slack, Discord, ntfy, and Telegram.

It is a single Rust binary and does not depend on Apprise, Python, or external notification tools.

## Installation

Install the latest Linux x86_64 release:

```bash
curl -fsSL https://raw.githubusercontent.com/firezl/nohupx/main/scripts/install.sh | sh
```

Install a specific version:

```bash
curl -fsSL https://raw.githubusercontent.com/firezl/nohupx/main/scripts/install.sh | NOHUPX_VERSION=0.1.0 sh
```

Install to a custom directory:

```bash
curl -fsSL https://raw.githubusercontent.com/firezl/nohupx/main/scripts/install.sh | NOHUPX_INSTALL_DIR=/usr/local/bin sh
```

Install from source:

```bash
cargo install --path .
```

Or build and copy the binary:

```bash
cargo build --release
cp target/release/nohupx ~/.local/bin/
```

## Release

This repository includes a GitHub Actions release workflow. Push a version tag to build Linux, macOS, and Windows binaries and publish them to GitHub Releases:

```bash
git tag v0.1.0
git push origin v0.1.0
```

You can also trigger the workflow from the GitHub Actions page with a tag input, or publish a GitHub Release from the web UI. The workflow will upload assets to the existing release if it already exists.

The workflow uploads `.tar.gz` archives for Linux/macOS, a `.zip` archive for Windows, and SHA-256 checksum files.

## Quick Start

```bash
# 1. Run once. nohupx will create default config automatically.
nohupx -- echo hello

# 2. Edit config.
vim ~/.config/nohupx/config.toml

# 3. Test notification channel.
nohupx test email

# 4. Run long command.
nohupx -d -- python train.py
```

If no notification target is enabled, `nohupx` still runs commands and saves logs. It only skips sending notifications.

## Configuration

Default config path:

```text
~/.config/nohupx/config.toml
```

Default log directory:

```text
~/.local/state/nohupx/logs
```

Example:

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

[[notify.targets]]
type = "email"
name = "my-email"
enabled = true
smtp_host = "smtp.qq.com"
smtp_port = 587
username = "xxx@qq.com"
password_env = "NOHUPX_SMTP_PASSWORD"
from = "xxx@qq.com"
to = ["xxx@qq.com"]

[[notify.targets]]
type = "webhook"
name = "my-webhook"
enabled = true
url = "https://example.com/notify"

[[notify.targets]]
type = "feishu"
name = "lab-feishu"
enabled = true
webhook = "https://open.feishu.cn/open-apis/bot/v2/hook/xxxx"

[[notify.targets]]
type = "wecom"
name = "lab-wecom"
enabled = true
webhook = "https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=xxxx"

[[notify.targets]]
type = "dingtalk"
name = "lab-dingtalk"
enabled = true
webhook = "https://oapi.dingtalk.com/robot/send?access_token=xxxx"

[[notify.targets]]
type = "slack"
name = "team-slack"
enabled = true
webhook = "https://hooks.slack.com/services/xxxx/yyyy/zzzz"

[[notify.targets]]
type = "discord"
name = "lab-discord"
enabled = true
webhook = "https://discord.com/api/webhooks/xxxx/yyyy"

[[notify.targets]]
type = "ntfy"
name = "phone-ntfy"
enabled = true
url = "https://ntfy.sh/your-topic"

[[notify.targets]]
type = "telegram"
name = "my-telegram"
enabled = true
bot_token_env = "NOHUPX_TELEGRAM_BOT_TOKEN"
chat_id = "12345678"
```

Secrets can be stored in three compatible ways:

- Plaintext fields in `config.toml`, such as `password`, `bot_token`, `webhook`, or `url`. This is the simplest option.
- Environment variables such as `password_env`, `bot_token_env`, `webhook_env`, `url_env`, and `proxy_env`. This keeps compatibility with earlier nohupx configs and works well in CI, containers, and temporary sessions.
- System keyring references via `*_secret`. This stores the secret in Windows Credential Manager, macOS Keychain, or Linux keyutils.

When more than one source is configured for the same value, nohupx resolves it in this order:

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
HTTP proxy:          proxy_env / proxy
```

To save a secret in the system keyring:

```bash
nohupx secret set email/password
nohupx secret set telegram/main
nohupx secret set slack/lab
```

Then reference it from config:

```toml
[[notify.targets]]
type = "email"
name = "my-email"
enabled = true
smtp_host = "smtp.qq.com"
smtp_port = 465
username = "xxx@qq.com"
password_secret = "email/password"
from = "xxx@qq.com"
to = ["xxx@qq.com"]
```

For HTTP webhook-like targets, the webhook URL itself is often the secret:

```toml
[[notify.targets]]
type = "slack"
name = "team-slack"
enabled = true
webhook_secret = "slack/lab"

[[notify.targets]]
type = "telegram"
name = "my-telegram"
enabled = true
bot_token_secret = "telegram/main"
chat_id = "12345678"
```

Environment variable config remains fully supported:

```toml
[[notify.targets]]
type = "email"
name = "my-email"
enabled = true
smtp_host = "smtp.qq.com"
smtp_port = 465
username = "xxx@qq.com"
password_env = "NOHUPX_SMTP_PASSWORD"
from = "xxx@qq.com"
to = ["xxx@qq.com"]

[[notify.targets]]
type = "telegram"
name = "my-telegram"
enabled = true
bot_token_env = "NOHUPX_TELEGRAM_BOT_TOKEN"
chat_id = "12345678"
```

Avoid committing plaintext secrets to Git repositories.

Email supports both common SMTP modes:

```toml
# STARTTLS
smtp_port = 587

# SMTPS / implicit TLS
smtp_port = 465
```

HTTP-based notification targets support per-target proxies. This applies to `webhook`, `feishu`, `wecom`, `dingtalk`, `slack`, `discord`, `ntfy`, and `telegram`.

```toml
[[notify.targets]]
type = "telegram"
name = "my-telegram"
enabled = true
bot_token_env = "NOHUPX_TELEGRAM_BOT_TOKEN"
chat_id = "12345678"
proxy = "http://127.0.0.1:7890"

[[notify.targets]]
type = "slack"
name = "team-slack"
enabled = true
webhook_env = "NOHUPX_SLACK_WEBHOOK"
proxy_env = "NOHUPX_PROXY"
```

Proxy fields are intentionally limited to environment variables or plaintext config:

```text
proxy_env > proxy
```

SMTP email proxying is not supported yet; SMTP uses a different transport path from the HTTP notification backends.

### Channel Configuration

All targets support:

```toml
name = "target-name"
enabled = true
```

For HTTP-based targets, add either `proxy` or `proxy_env` when needed:

```toml
proxy = "http://127.0.0.1:7890"
proxy_env = "NOHUPX_PROXY"
```

Email:

```toml
[[notify.targets]]
type = "email"
name = "my-email"
enabled = true
smtp_host = "smtp.qq.com"
smtp_port = 465
username = "xxx@qq.com"
password_secret = "email/password" # or password_env / password
from = "xxx@qq.com"
to = ["xxx@qq.com"]
```

Generic webhook:

```toml
[[notify.targets]]
type = "webhook"
name = "my-webhook"
enabled = true
url_secret = "webhook/main" # or url_env / url
```

Feishu:

```toml
[[notify.targets]]
type = "feishu"
name = "lab-feishu"
enabled = true
webhook_secret = "feishu/lab" # or webhook_env / webhook
```

WeCom:

```toml
[[notify.targets]]
type = "wecom"
name = "lab-wecom"
enabled = true
webhook_secret = "wecom/lab" # or webhook_env / webhook
```

DingTalk:

```toml
[[notify.targets]]
type = "dingtalk"
name = "lab-dingtalk"
enabled = true
webhook_secret = "dingtalk/lab" # or webhook_env / webhook
```

Slack:

```toml
[[notify.targets]]
type = "slack"
name = "team-slack"
enabled = true
webhook_secret = "slack/lab" # or webhook_env / webhook
```

Discord:

```toml
[[notify.targets]]
type = "discord"
name = "lab-discord"
enabled = true
webhook_secret = "discord/lab" # or webhook_env / webhook
```

ntfy:

```toml
[[notify.targets]]
type = "ntfy"
name = "phone-ntfy"
enabled = true
url_secret = "ntfy/topic" # or url_env / url
```

Telegram:

```toml
[[notify.targets]]
type = "telegram"
name = "my-telegram"
enabled = true
bot_token_secret = "telegram/main" # or bot_token_env / bot_token
chat_id = "12345678"
proxy_env = "NOHUPX_PROXY"
```

## Usage

Foreground run:

```bash
nohupx -- python train.py
nohupx run -- python train.py
```

Detached run:

```bash
nohupx -d -- python train.py
nohupx --name exp01 -d -- python run_exp.py
```

Only notify on failure:

```bash
nohupx --only-fail -- make build
```

Disable notifications for one run:

```bash
nohupx --no-notify -- python train.py
```

Show recent logs:

```bash
nohupx logs
nohupx logs -n 20
```

Test notification channels:

```bash
nohupx test email
nohupx test my-email
nohupx test webhook
nohupx test feishu
nohupx test wecom
nohupx test dingtalk
nohupx test slack
nohupx test discord
nohupx test ntfy
nohupx test all
nohupx test example-email --include-disabled
```

The channel in `nohupx test <channel>` can be a target name or a target type, such as `email`, `webhook`, `feishu`, `wecom`, `dingtalk`, `slack`, `discord`, `ntfy`, or `telegram`. `nohupx test all` tests all enabled targets.

## Notes

Detached mode is a lightweight background run mode, not a full process supervisor. For complex job management, use systemd, tmux, screen, Slurm, or a similar tool.
