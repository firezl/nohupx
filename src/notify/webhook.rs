use anyhow::{bail, Context, Result};
use serde_json::json;

use crate::config::NotifyTargetConfig;
use crate::notify::NotifyMessage;

pub fn send(target: &NotifyTargetConfig, msg: &NotifyMessage) -> Result<()> {
    let NotifyTargetConfig::Webhook { url, .. } = target else {
        bail!("not a webhook target");
    };

    let client = reqwest::blocking::Client::new();
    client
        .post(url)
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
