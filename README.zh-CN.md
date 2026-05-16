# nohupx

[English](README.md)

`nohupx` 是一个轻量级的 nohup-like 命令运行器，内置任务结束通知。
它会运行长命令、保存完整日志，并在命令结束后通过 email、通用 webhook、飞书、企业微信、钉钉、Slack、Discord、ntfy、Telegram 等渠道发送运行结果。

`nohupx` 是单二进制 Rust CLI 工具，不依赖 Apprise、Python 或外部通知命令。

## 安装

安装最新 Linux x86_64 Release：

```bash
curl -fsSL https://raw.githubusercontent.com/firezl/nohupx/main/scripts/install.sh | sh
```

安装指定版本：

```bash
curl -fsSL https://raw.githubusercontent.com/firezl/nohupx/main/scripts/install.sh | NOHUPX_VERSION=0.1.0 sh
```

安装到自定义目录：

```bash
curl -fsSL https://raw.githubusercontent.com/firezl/nohupx/main/scripts/install.sh | NOHUPX_INSTALL_DIR=/usr/local/bin sh
```

从源码安装：

```bash
cargo install --path .
```

或者手动构建并复制二进制：

```bash
cargo build --release
cp target/release/nohupx ~/.local/bin/
```

## 发布

仓库内置 GitHub Actions release workflow。推送版本 tag 后，会自动构建 Linux、macOS、Windows 二进制并发布到 GitHub Releases：

```bash
git tag v0.1.0
git push origin v0.1.0
```

也可以在 GitHub Actions 页面手动运行 workflow 并输入 tag，或者从 GitHub 页面发布 Release。若 Release 已存在，workflow 会把构建产物上传到已有 Release。

workflow 会上传 Linux/macOS 的 `.tar.gz`、Windows 的 `.zip`，以及 SHA-256 校验文件。

## 快速开始

```bash
# 1. 先运行一次。nohupx 会自动创建默认配置。
nohupx -- echo hello

# 2. 编辑配置。
vim ~/.config/nohupx/config.toml

# 3. 测试通知渠道。
nohupx test email

# 4. 后台运行长命令。
nohupx -d -- python train.py
```

如果没有启用任何通知 target，`nohupx` 仍然会正常运行命令并保存日志，只是不会发送通知。

## 配置

默认配置文件路径：

```text
~/.config/nohupx/config.toml
```

默认日志目录：

```text
~/.local/state/nohupx/logs
```

配置示例：

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

建议使用 `NOHUPX_SMTP_PASSWORD`、`NOHUPX_TELEGRAM_BOT_TOKEN` 等环境变量保存邮箱密码或 bot token。不要把密钥明文提交到 Git 仓库。

email 支持常见 SMTP 模式：

```toml
# STARTTLS
smtp_port = 587

# SMTPS / implicit TLS
smtp_port = 465
```

## 使用

前台运行：

```bash
nohupx -- python train.py
nohupx run -- python train.py
```

后台运行：

```bash
nohupx -d -- python train.py
nohupx --name exp01 -d -- python run_exp.py
```

只在失败时通知：

```bash
nohupx --only-fail -- make build
```

本次运行不发送通知：

```bash
nohupx --no-notify -- python train.py
```

查看最近日志：

```bash
nohupx logs
nohupx logs -n 20
```

测试通知渠道：

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

`nohupx test <channel>` 里的 `channel` 可以是 target name，也可以是 target type，例如 `email`、`webhook`、`feishu`、`wecom`、`dingtalk`、`slack`、`discord`、`ntfy`、`telegram`。`nohupx test all` 会测试所有启用的 target。

## 说明

detached 模式是轻量后台运行模式，不是完整的进程管理器。如果需要复杂任务管理，请使用 systemd、tmux、screen、Slurm 等工具。
