pub mod attachment;
pub mod dingtalk;
pub mod discord;
pub mod email;
pub mod feishu;
pub mod ntfy;
pub mod slack;
pub mod telegram;
pub mod template;
pub mod webhook;
pub mod wecom;

use std::path::Path;

use anyhow::{Context, Error, Result};
use chrono::Local;

use crate::cli::TestArgs;
use crate::config::{Config, NotifyConfig, NotifyTargetConfig};
use crate::notify::template::{render_for_target, TemplateScenario};
use crate::secret;

pub use template::{RenderedMessage, TemplateContext};

pub fn send_all(config: &NotifyConfig, ctx: &TemplateContext) -> Vec<(String, Error)> {
    let mut errors = Vec::new();
    if !config.enabled {
        return errors;
    }

    for target in config.targets.iter().filter(|target| target.enabled()) {
        if let Err(err) = send_target(config, target, ctx, TemplateScenario::Run) {
            errors.push((target.display_name(), err));
        }
    }
    errors
}

pub fn send_target(
    config: &NotifyConfig,
    target: &NotifyTargetConfig,
    ctx: &TemplateContext,
    scenario: TemplateScenario,
) -> anyhow::Result<()> {
    let rendered = render_for_target(config, target, ctx, scenario)?;
    if rendered.attach_log && !target.supports_attachments() {
        eprintln!(
            "Warning: attach_log is enabled for {}, but this channel does not support attachments; attachment skipped",
            target.display_name()
        );
    }
    match target {
        NotifyTargetConfig::Email { .. } => email::send(config, target, &rendered),
        NotifyTargetConfig::Webhook { .. } => webhook::send(config, target, &rendered),
        NotifyTargetConfig::Feishu { .. } => feishu::send(target, &rendered),
        NotifyTargetConfig::Wecom { .. } => wecom::send(target, &rendered),
        NotifyTargetConfig::Dingtalk { .. } => dingtalk::send(target, &rendered),
        NotifyTargetConfig::Slack { .. } => slack::send(target, &rendered),
        NotifyTargetConfig::Discord { .. } => discord::send(config, target, &rendered),
        NotifyTargetConfig::Ntfy { .. } => ntfy::send(config, target, &rendered),
        NotifyTargetConfig::Telegram { .. } => telegram::send(config, target, &rendered),
    }
}

pub(crate) fn http_client(target: &NotifyTargetConfig) -> Result<reqwest::blocking::Client> {
    let (proxy, proxy_env) = target.proxy_parts();
    let proxy = resolve_optional_secret(proxy, proxy_env, None, "proxy URL")?;
    let mut builder = reqwest::blocking::Client::builder();
    if let Some(proxy) = proxy {
        builder = builder.proxy(
            reqwest::Proxy::all(&proxy)
                .with_context(|| format!("invalid proxy URL for {}", target.display_name()))?,
        );
    }
    builder.build().context("failed to build HTTP client")
}

pub(crate) fn resolve_required_secret(
    inline: Option<&str>,
    env: Option<&str>,
    secret_key: Option<&str>,
    label: &str,
) -> Result<String> {
    resolve_optional_secret(inline, env, secret_key, label)?.with_context(|| {
        format!("missing {label}; set inline value, environment variable, or encrypted secret")
    })
}

pub(crate) fn resolve_optional_secret(
    inline: Option<&str>,
    env: Option<&str>,
    secret_key: Option<&str>,
    label: &str,
) -> Result<Option<String>> {
    if let Some(key) = secret_key {
        return Ok(Some(secret::get(key).with_context(|| {
            format!(
                "failed to resolve {label} secret {key:?}. Check the encrypted secrets store or use the matching *_env field or inline field for this target"
            )
        })?));
    }

    if let Some(var) = env {
        let value =
            std::env::var(var).with_context(|| format!("environment variable {var} is not set"))?;
        return Ok(Some(value));
    }

    Ok(inline.map(ToOwned::to_owned))
}

pub(crate) fn merged_text(msg: &RenderedMessage) -> String {
    format!("{}\n\n{}", msg.title, msg.body)
}

#[derive(Debug)]
pub struct TargetMatch<'a> {
    pub targets: Vec<&'a NotifyTargetConfig>,
    pub disabled: Vec<&'a NotifyTargetConfig>,
    pub found_any: bool,
}

pub fn match_targets<'a>(
    targets: &'a [NotifyTargetConfig],
    channel: &str,
    include_disabled: bool,
) -> TargetMatch<'a> {
    let candidates: Vec<&NotifyTargetConfig> = if channel == "all" {
        targets.iter().collect()
    } else {
        let by_name: Vec<_> = targets
            .iter()
            .filter(|target| target.name() == Some(channel))
            .collect();
        if !by_name.is_empty() {
            by_name
        } else {
            targets
                .iter()
                .filter(|target| target.type_name() == channel)
                .collect()
        }
    };

    let found_any = !candidates.is_empty();
    let mut enabled_targets = Vec::new();
    let mut disabled = Vec::new();
    for target in candidates {
        if target.enabled() || include_disabled {
            enabled_targets.push(target);
        } else {
            disabled.push(target);
        }
    }

    TargetMatch {
        targets: enabled_targets,
        disabled,
        found_any,
    }
}

pub fn run_test(config: &Config, config_path: &Path, args: &TestArgs) -> anyhow::Result<i32> {
    let matched = match_targets(&config.notify.targets, &args.channel, args.include_disabled);

    if !matched.found_any {
        eprintln!("No notification target matched: {}", args.channel);
        return Ok(1);
    }

    if matched.targets.is_empty() {
        for target in matched.disabled {
            eprintln!(
                "Target \"{}\" is disabled. Enable it in config.toml first.",
                target.display_name()
            );
            eprintln!(
                "Or run: nohupx test {} --include-disabled",
                target.display_name()
            );
        }
        return Ok(1);
    }

    let host = test_hostname();
    let now = Local::now();
    let mut failed = false;

    for target in matched.targets {
        let target_label = format!("{}/{}", target.display_name(), target.type_name());
        let ctx = TemplateContext::for_test(
            &host,
            &now.format("%Y-%m-%d %H:%M:%S").to_string(),
            config_path,
            &target.display_name(),
            target.type_name(),
            &target_label,
        );

        match send_target(&config.notify, target, &ctx, TemplateScenario::Test) {
            Ok(()) => println!("OK: {}", target.display_name()),
            Err(err) => {
                failed = true;
                eprintln!("FAILED: {}: {err:#}", target.display_name());
            }
        }
    }

    Ok(if failed { 1 } else { 0 })
}

fn test_hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "unknown-host".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TargetTemplateFields;

    fn targets() -> Vec<NotifyTargetConfig> {
        vec![
            NotifyTargetConfig::Email {
                template: TargetTemplateFields::default(),
                name: Some("email-one".to_string()),
                enabled: Some(true),
                smtp_host: "smtp.example.com".to_string(),
                smtp_port: Some(587),
                username: "u".to_string(),
                password_secret: None,
                password_env: None,
                password: Some("p".to_string()),
                from: "a@example.com".to_string(),
                to: vec!["b@example.com".to_string()],
            },
            NotifyTargetConfig::Email {
                template: TargetTemplateFields::default(),
                name: Some("disabled-email".to_string()),
                enabled: Some(false),
                smtp_host: "smtp.example.com".to_string(),
                smtp_port: Some(587),
                username: "u".to_string(),
                password_secret: None,
                password_env: None,
                password: Some("p".to_string()),
                from: "a@example.com".to_string(),
                to: vec!["b@example.com".to_string()],
            },
            NotifyTargetConfig::Webhook {
                template: TargetTemplateFields::default(),
                name: Some("email".to_string()),
                enabled: Some(true),
                url: Some("https://example.com".to_string()),
                url_secret: None,
                url_env: None,
                proxy: None,
                proxy_env: None,
            },
        ]
    }

    #[test]
    fn name_match_has_priority() {
        let all = targets();
        let matched = match_targets(&all, "email", false);
        assert_eq!(matched.targets.len(), 1);
        assert_eq!(matched.targets[0].type_name(), "webhook");
    }

    #[test]
    fn type_match_returns_enabled_targets() {
        let all = targets();
        let matched = match_targets(&all[..2], "email", false);
        assert_eq!(matched.targets.len(), 1);
        assert_eq!(matched.targets[0].display_name(), "email-one");
        assert_eq!(matched.disabled.len(), 1);
    }

    #[test]
    fn all_match_filters_disabled() {
        let all = targets();
        let matched = match_targets(&all, "all", false);
        assert_eq!(matched.targets.len(), 2);
        assert_eq!(matched.disabled.len(), 1);
    }

    #[test]
    fn include_disabled_allows_disabled_targets() {
        let all = targets();
        let matched = match_targets(&all, "disabled-email", true);
        assert_eq!(matched.targets.len(), 1);
        assert!(matched.disabled.is_empty());
    }

    #[test]
    fn resolves_inline_secret() {
        let value = resolve_required_secret(Some("secret"), None, None, "test").unwrap();
        assert_eq!(value, "secret");
    }

    #[test]
    fn resolves_secret_from_env_before_inline() {
        let var = format!(
            "NOHUPX_TEST_SECRET_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        std::env::set_var(&var, "from-env");
        let value =
            resolve_required_secret(Some("inline"), Some(&var), None, "test env secret").unwrap();
        std::env::remove_var(&var);
        assert_eq!(value, "from-env");
    }
}
