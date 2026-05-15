use anyhow::{bail, Context, Result};
use serde_json::json;

use crate::config::NotifyTargetConfig;
use crate::notify::NotifyMessage;

pub fn send(target: &NotifyTargetConfig, msg: &NotifyMessage) -> Result<()> {
    let NotifyTargetConfig::Wecom { webhook, .. } = target else {
        bail!("not a wecom target");
    };

    let text = format!("{}\n\n{}", msg.title, msg.body);
    reqwest::blocking::Client::new()
        .post(webhook)
        .json(&json!({
            "msgtype": "text",
            "text": {
                "content": text,
            },
        }))
        .send()
        .with_context(|| format!("failed to POST WeCom webhook {webhook}"))?
        .error_for_status()
        .with_context(|| format!("WeCom webhook returned error status for {webhook}"))?;
    Ok(())
}
