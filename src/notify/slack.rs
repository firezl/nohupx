use anyhow::{bail, Context, Result};
use serde_json::json;

use crate::config::NotifyTargetConfig;
use crate::notify::{http_client, resolve_required_secret, NotifyMessage};

pub fn send(target: &NotifyTargetConfig, msg: &NotifyMessage) -> Result<()> {
    let NotifyTargetConfig::Slack {
        webhook,
        webhook_env,
        webhook_secret,
        ..
    } = target
    else {
        bail!("not a slack target");
    };
    let webhook = resolve_required_secret(
        webhook.as_deref(),
        webhook_env.as_deref(),
        webhook_secret.as_deref(),
        "Slack webhook URL",
    )?;

    let text = format!("{}\n\n{}", msg.title, msg.body);
    http_client(target)?
        .post(&webhook)
        .json(&json!({
            "text": text,
        }))
        .send()
        .with_context(|| format!("failed to POST Slack webhook {webhook}"))?
        .error_for_status()
        .with_context(|| format!("Slack webhook returned error status for {webhook}"))?;
    Ok(())
}
