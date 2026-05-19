use anyhow::{bail, Context, Result};
use serde_json::json;

use crate::config::NotifyTargetConfig;
use crate::notify::{http_client, resolve_required_secret, NotifyMessage};

pub fn send(target: &NotifyTargetConfig, msg: &NotifyMessage) -> Result<()> {
    let NotifyTargetConfig::Webhook {
        url,
        url_env,
        url_secret,
        ..
    } = target
    else {
        bail!("not a webhook target");
    };
    let url = resolve_required_secret(
        url.as_deref(),
        url_env.as_deref(),
        url_secret.as_deref(),
        "webhook URL",
    )?;

    http_client(target)?
        .post(&url)
        .json(&json!({
            "title": msg.title,
            "body": msg.body,
            "success": msg.success,
            "exit_code": msg.exit_code,
            "command": msg.command,
            "host": msg.host,
            "duration_seconds": msg.duration_seconds,
            "log_path": msg.log_path.display().to_string(),
        }))
        .send()
        .with_context(|| format!("failed to POST webhook {url}"))?
        .error_for_status()
        .with_context(|| format!("webhook returned error status for {url}"))?;
    Ok(())
}
