use anyhow::{bail, Context, Result};
use serde_json::json;

use crate::config::{NotifyConfig, NotifyTargetConfig};
use crate::notify::attachment::{log_base64, read_log_attachment, AttachmentOutcome};
use crate::notify::template::{max_attachment_bytes, RenderedMessage};
use crate::notify::{http_client, resolve_required_secret};

pub fn send(
    config: &NotifyConfig,
    target: &NotifyTargetConfig,
    msg: &RenderedMessage,
) -> Result<()> {
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

    let mut payload = json!({
        "title": msg.title,
        "body": msg.body,
        "success": msg.success,
        "exit_code": msg.exit_code,
        "command": msg.command,
        "host": msg.host,
        "duration_seconds": msg.duration_seconds,
        "log_path": msg.log_path.display().to_string(),
    });

    if msg.attach_log {
        match read_log_attachment(&msg.log_path, max_attachment_bytes(config))? {
            AttachmentOutcome::Ready(attachment) => {
                payload["log_base64"] = json!(log_base64(&attachment));
                payload["log_filename"] = json!(attachment.filename);
            }
            AttachmentOutcome::Skipped(reason) => {
                eprintln!("Warning: {reason}");
            }
        }
    }

    http_client(target)?
        .post(&url)
        .json(&payload)
        .send()
        .with_context(|| format!("failed to POST webhook {url}"))?
        .error_for_status()
        .with_context(|| format!("webhook returned error status for {url}"))?;
    Ok(())
}
