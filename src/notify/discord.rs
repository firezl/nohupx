use anyhow::{bail, Context, Result};
use serde_json::json;

use crate::config::NotifyTargetConfig;
use crate::notify::NotifyMessage;

pub fn send(target: &NotifyTargetConfig, msg: &NotifyMessage) -> Result<()> {
    let NotifyTargetConfig::Discord { webhook, .. } = target else {
        bail!("not a discord target");
    };

    let content = format!("{}\n\n{}", msg.title, msg.body);
    reqwest::blocking::Client::new()
        .post(webhook)
        .json(&json!({
            "content": content,
        }))
        .send()
        .with_context(|| format!("failed to POST Discord webhook {webhook}"))?
        .error_for_status()
        .with_context(|| format!("Discord webhook returned error status for {webhook}"))?;
    Ok(())
}
