# nohupx

`nohupx` is a lightweight nohup-like command runner with notifications.
It runs long commands, saves complete logs, and sends the final result through configured channels such as email, generic webhooks, Feishu, WeCom, and Telegram.

It is a single Rust binary and does not depend on Apprise, Python, or external notification tools.

## Installation

```bash
cargo install --path .
```

Or build and copy the binary:

```bash
cargo build --release
cp target/release/nohupx ~/.local/bin/
```

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
type = "telegram"
name = "my-telegram"
enabled = true
bot_token_env = "NOHUPX_TELEGRAM_BOT_TOKEN"
chat_id = "12345678"
```

For email passwords and bot tokens, prefer environment variables such as `NOHUPX_SMTP_PASSWORD` and `NOHUPX_TELEGRAM_BOT_TOKEN`. Avoid writing secrets directly in `config.toml`, and do not commit your config file to a Git repository.

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
nohupx test all
nohupx test example-email --include-disabled
```

The channel in `nohupx test <channel>` can be a target name or a target type, such as `email`, `webhook`, `feishu`, `wecom`, or `telegram`. `nohupx test all` tests all enabled targets.

## Notes

Detached mode is a lightweight background run mode, not a full process supervisor. For complex job management, use systemd, tmux, screen, Slurm, or a similar tool.
