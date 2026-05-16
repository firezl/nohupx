use anyhow::{bail, Context, Result};
use serde_json::json;

use crate::config::NotifyTargetConfig;
use crate::notify::NotifyMessage;

pub fn send(target: &NotifyTargetConfig, msg: &NotifyMessage) -> Result<()> {
    let NotifyTargetConfig::Slack { webhook, .. } = target else {
        bail!("not a slack target");
    };

    let text = format!("{}\n\n{}", msg.title, msg.body);
    reqwest::blocking::Client::new()
        .post(webhook)
        .json(&json!({
            "text": text,
        }))
        .send()
        .with_context(|| format!("failed to POST Slack webhook {webhook}"))?
        .error_for_status()
        .with_context(|| format!("Slack webhook returned error status for {webhook}"))?;
    Ok(())
}
