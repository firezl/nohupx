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

密钥有三种兼容的保存方式：

- 直接写在 `config.toml` 明文字段中，例如 `password`、`bot_token`、`webhook` 或 `url`。这是最简单的方式。
- 使用环境变量字段，例如 `password_env`、`bot_token_env`、`webhook_env`、`url_env`、`proxy_env`。这会继续兼容之前的 nohupx 配置，也适合 CI、容器和临时会话。
- 使用 `*_secret` 引用系统 keyring。密钥会保存到 Windows Credential Manager、macOS Keychain 或 Linux Secret Service。

在 Linux 上，`*_secret` 需要运行环境中有可用的 Secret Service provider，例如 GNOME Keyring 或 KWallet。

同一个值如果同时配置了多个来源，nohupx 按以下优先级读取：

```text
*_secret > *_env > 明文字段
```

支持的密钥字段：

```text
email 密码:         password_secret / password_env / password
telegram token:    bot_token_secret / bot_token_env / bot_token
webhook 类 URL:    webhook_secret / webhook_env / webhook
generic webhook:   url_secret / url_env / url
ntfy URL:          url_secret / url_env / url
HTTP 代理:         proxy_env / proxy
```

保存密钥到系统 keyring：

```bash
nohupx secret set email/password
nohupx secret set telegram/main
nohupx secret set slack/lab
```

然后在配置中引用：

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

对于 HTTP webhook 类渠道，webhook URL 本身通常就是密钥：

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

环境变量配置仍然完整支持：

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

不要把明文密钥提交到 Git 仓库。

email 支持常见 SMTP 模式：

```toml
# STARTTLS
smtp_port = 587

# SMTPS / implicit TLS
smtp_port = 465
```

HTTP 类通知渠道支持按 target 单独设置代理，适用于 `webhook`、`feishu`、`wecom`、`dingtalk`、`slack`、`discord`、`ntfy`、`telegram`。

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

代理字段刻意限制为环境变量或明文配置：

```text
proxy_env > proxy
```

SMTP email 暂不支持代理；它和 HTTP 通知后端使用的是不同的传输路径。

### 各渠道配置帮助

所有 target 都支持：

```toml
name = "target-name"
enabled = true
```

HTTP 类 target 如需代理，可以使用 `proxy` 或 `proxy_env`：

```toml
proxy = "http://127.0.0.1:7890"
proxy_env = "NOHUPX_PROXY"
```

Email：

```toml
[[notify.targets]]
type = "email"
name = "my-email"
enabled = true
smtp_host = "smtp.qq.com"
smtp_port = 465
username = "xxx@qq.com"
password_secret = "email/password" # 或 password_env / password
from = "xxx@qq.com"
to = ["xxx@qq.com"]
```

通用 webhook：

```toml
[[notify.targets]]
type = "webhook"
name = "my-webhook"
enabled = true
url_secret = "webhook/main" # 或 url_env / url
```

飞书：

```toml
[[notify.targets]]
type = "feishu"
name = "lab-feishu"
enabled = true
webhook_secret = "feishu/lab" # 或 webhook_env / webhook
```

企业微信：

```toml
[[notify.targets]]
type = "wecom"
name = "lab-wecom"
enabled = true
webhook_secret = "wecom/lab" # 或 webhook_env / webhook
```

钉钉：

```toml
[[notify.targets]]
type = "dingtalk"
name = "lab-dingtalk"
enabled = true
webhook_secret = "dingtalk/lab" # 或 webhook_env / webhook
```

Slack：

```toml
[[notify.targets]]
type = "slack"
name = "team-slack"
enabled = true
webhook_secret = "slack/lab" # 或 webhook_env / webhook
```

Discord：

```toml
[[notify.targets]]
type = "discord"
name = "lab-discord"
enabled = true
webhook_secret = "discord/lab" # 或 webhook_env / webhook
```

ntfy：

```toml
[[notify.targets]]
type = "ntfy"
name = "phone-ntfy"
enabled = true
url_secret = "ntfy/topic" # 或 url_env / url
```

Telegram：

```toml
[[notify.targets]]
type = "telegram"
name = "my-telegram"
enabled = true
bot_token_secret = "telegram/main" # 或 bot_token_env / bot_token
chat_id = "12345678"
proxy_env = "NOHUPX_PROXY"
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
