use anyhow::{bail, Context, Result};

use crate::config::{NotifyConfig, NotifyTargetConfig};
use crate::notify::attachment::{read_log_attachment, AttachmentOutcome};
use crate::notify::template::{max_attachment_bytes, RenderedMessage};
use crate::notify::{http_client, merged_text, resolve_required_secret};

pub fn send(
    config: &NotifyConfig,
    target: &NotifyTargetConfig,
    msg: &RenderedMessage,
) -> Result<()> {
    let NotifyTargetConfig::Discord {
        webhook,
        webhook_env,
        webhook_secret,
        ..
    } = target
    else {
        bail!("not a discord target");
    };
    let webhook = resolve_required_secret(
        webhook.as_deref(),
        webhook_env.as_deref(),
        webhook_secret.as_deref(),
        "Discord webhook URL",
    )?;

    let content = merged_text(msg);
    let client = http_client(target)?;
    let response = if msg.attach_log {
        match read_log_attachment(&msg.log_path, max_attachment_bytes(config))? {
            AttachmentOutcome::Ready(attachment) => {
                let part = reqwest::blocking::multipart::Part::bytes(attachment.bytes)
                    .file_name(attachment.filename)
                    .mime_str("text/plain")
                    .context("failed to build Discord attachment part")?;
                client
                    .post(&webhook)
                    .multipart(
                        reqwest::blocking::multipart::Form::new()
                            .text("content", content)
                            .part("file", part),
                    )
                    .send()
            }
            AttachmentOutcome::Skipped(reason) => {
                eprintln!("Warning: {reason}");
                client
                    .post(&webhook)
                    .json(&serde_json::json!({ "content": content }))
                    .send()
            }
        }
    } else {
        client
            .post(&webhook)
            .json(&serde_json::json!({ "content": content }))
            .send()
    };

    response
        .with_context(|| format!("failed to POST Discord webhook {webhook}"))?
        .error_for_status()
        .with_context(|| format!("Discord webhook returned error status for {webhook}"))?;
    Ok(())
}
