use anyhow::{bail, Context, Result};
use serde_json::json;

use crate::config::NotifyTargetConfig;
use crate::notify::template::RenderedMessage;
use crate::notify::{http_client, merged_text, resolve_required_secret};

pub fn send(target: &NotifyTargetConfig, msg: &RenderedMessage) -> Result<()> {
    let NotifyTargetConfig::Feishu {
        webhook,
        webhook_env,
        webhook_secret,
        ..
    } = target
    else {
        bail!("not a feishu target");
    };
    let webhook = resolve_required_secret(
        webhook.as_deref(),
        webhook_env.as_deref(),
        webhook_secret.as_deref(),
        "Feishu webhook URL",
    )?;

    let text = merged_text(msg);
    http_client(target)?
        .post(&webhook)
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
