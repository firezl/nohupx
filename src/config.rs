use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

pub const DEFAULT_CONFIG: &str = r#"[run]
default_detach = false
shell = false

[log]
dir = "~/.local/state/nohupx/logs"
tail_lines = 80

[notify]
enabled = true
only_fail = false

[[notify.targets]]
type = "webhook"
name = "example-webhook"
enabled = false
url = "https://example.com/notify"

[[notify.targets]]
type = "email"
name = "example-email"
enabled = false
smtp_host = "smtp.example.com"
smtp_port = 587
username = "your@example.com"
password_env = "NOHUPX_SMTP_PASSWORD"
from = "your@example.com"
to = ["your@example.com"]

[[notify.targets]]
type = "feishu"
name = "example-feishu"
enabled = false
webhook = "https://open.feishu.cn/open-apis/bot/v2/hook/xxxx"

[[notify.targets]]
type = "wecom"
name = "example-wecom"
enabled = false
webhook = "https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=xxxx"
"#;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub run: RunConfig,
    #[serde(default)]
    pub log: LogConfig,
    #[serde(default)]
    pub notify: NotifyConfig,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RunConfig {
    #[serde(default)]
    pub default_detach: bool,
    #[serde(default)]
    pub shell: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LogConfig {
    #[serde(default = "default_log_dir_string")]
    pub dir: String,
    #[serde(default = "default_tail_lines")]
    pub tail_lines: usize,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            dir: default_log_dir_string(),
            tail_lines: default_tail_lines(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct NotifyConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub only_fail: bool,
    #[serde(default)]
    pub targets: Vec<NotifyTargetConfig>,
}

impl Default for NotifyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            only_fail: false,
            targets: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum NotifyTargetConfig {
    #[serde(rename = "email")]
    Email {
        name: Option<String>,
        enabled: Option<bool>,
        smtp_host: String,
        smtp_port: Option<u16>,
        username: String,
        password_env: Option<String>,
        password: Option<String>,
        from: String,
        to: Vec<String>,
    },
    #[serde(rename = "webhook")]
    Webhook {
        name: Option<String>,
        enabled: Option<bool>,
        url: String,
    },
    #[serde(rename = "feishu")]
    Feishu {
        name: Option<String>,
        enabled: Option<bool>,
        webhook: String,
    },
    #[serde(rename = "wecom")]
    Wecom {
        name: Option<String>,
        enabled: Option<bool>,
        webhook: String,
    },
    #[serde(rename = "telegram")]
    Telegram {
        name: Option<String>,
        enabled: Option<bool>,
        bot_token_env: Option<String>,
        bot_token: Option<String>,
        chat_id: String,
    },
}

impl NotifyTargetConfig {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Email { .. } => "email",
            Self::Webhook { .. } => "webhook",
            Self::Feishu { .. } => "feishu",
            Self::Wecom { .. } => "wecom",
            Self::Telegram { .. } => "telegram",
        }
    }

    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Email { name, .. }
            | Self::Webhook { name, .. }
            | Self::Feishu { name, .. }
            | Self::Wecom { name, .. }
            | Self::Telegram { name, .. } => name.as_deref(),
        }
    }

    pub fn display_name(&self) -> String {
        self.name()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| self.type_name().to_string())
    }

    pub fn enabled(&self) -> bool {
        match self {
            Self::Email { enabled, .. }
            | Self::Webhook { enabled, .. }
            | Self::Feishu { enabled, .. }
            | Self::Wecom { enabled, .. }
            | Self::Telegram { enabled, .. } => enabled.unwrap_or(true),
        }
    }
}

pub fn load_or_create_config(path: Option<PathBuf>) -> Result<(Config, PathBuf, bool)> {
    let path = path.unwrap_or_else(default_config_path);
    if !path.exists() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create config directory {}", parent.display())
            })?;
        }
        fs::write(&path, DEFAULT_CONFIG)
            .with_context(|| format!("failed to create default config {}", path.display()))?;
        let config = parse_config(DEFAULT_CONFIG, &path)?;
        return Ok((config, path, true));
    }

    let content = fs::read_to_string(&path)
        .with_context(|| format!("failed to read config {}", path.display()))?;
    let config = parse_config(&content, &path)?;
    Ok((config, path, false))
}

fn parse_config(content: &str, path: &Path) -> Result<Config> {
    toml::from_str(content).with_context(|| format!("invalid config TOML: {}", path.display()))
}

pub fn default_config_path() -> PathBuf {
    home_dir()
        .join(".config")
        .join("nohupx")
        .join("config.toml")
}

pub fn expand_path(path: &str) -> Result<PathBuf> {
    let expanded = shellexpand::full(path)
        .with_context(|| format!("failed to expand path {path:?}"))?
        .into_owned();
    Ok(PathBuf::from(expanded))
}

pub fn user_facing_path(path: &Path) -> String {
    let home = home_dir();
    if let Ok(stripped) = path.strip_prefix(&home) {
        let rest = stripped.to_string_lossy().replace('\\', "/");
        if rest.is_empty() {
            "~".to_string()
        } else {
            format!("~/{rest}")
        }
    } else {
        path.display().to_string()
    }
}

fn default_log_dir_string() -> String {
    "~/.local/state/nohupx/logs".to_string()
}

fn default_tail_lines() -> usize {
    80
}

fn default_true() -> bool {
    true
}

fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_config() {
        let cfg: Config = toml::from_str(DEFAULT_CONFIG).unwrap();
        assert_eq!(cfg.log.tail_lines, 80);
        assert_eq!(cfg.notify.targets.len(), 4);
        assert!(!cfg.notify.targets[0].enabled());
    }

    #[test]
    fn creates_default_config() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("config.toml");
        let (_cfg, actual, created) = load_or_create_config(Some(path.clone())).unwrap();
        assert!(created);
        assert_eq!(actual, path);
        assert!(actual.exists());
    }

    #[test]
    fn expands_tilde_path() {
        let path = expand_path("~/.local/state/nohupx/logs").unwrap();
        assert!(
            path.ends_with(".local/state/nohupx/logs")
                || path.ends_with(".local\\state\\nohupx\\logs")
        );
    }
}
