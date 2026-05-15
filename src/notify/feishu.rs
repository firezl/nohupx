use anyhow::{bail, Context, Result};
use serde_json::json;

use crate::config::NotifyTargetConfig;
use crate::notify::NotifyMessage;

pub fn send(target: &NotifyTargetConfig, msg: &NotifyMessage) -> Result<()> {
    let NotifyTargetConfig::Feishu { webhook, .. } = target else {
        bail!("not a feishu target");
    };

    let text = format!("{}\n\n{}", msg.title, msg.body);
    reqwest::blocking::Client::new()
        .post(webhook)
        .json(&json!({
            "msg_type": "text",
            "content": {
                "text": text,
            },
        }))
        .send()
        .with_context(|| format!("failed to POST Feishu webhook {webhook}"))?
        .error_for_status()
        .with_context(|| format!("Feishu webhook returned error status for {webhook}"))?;
    Ok(())
}
