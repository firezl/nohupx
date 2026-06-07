use anyhow::{bail, Context, Result};
use serde_json::json;

use crate::config::{NotifyConfig, NotifyTargetConfig};
use crate::notify::attachment::{read_log_attachment, AttachmentOutcome};
use crate::notify::template::{max_attachment_bytes, RenderedMessage};
use crate::notify::{http_client, merged_text, resolve_required_secret};

pub fn send(
    config: &NotifyConfig,
    target: &NotifyTargetConfig,
    msg: &RenderedMessage,
) -> Result<()> {
    let NotifyTargetConfig::Telegram {
        bot_token_env,
        bot_token_secret,
        bot_token,
        chat_id,
        ..
    } = target
    else {
        bail!("not a telegram target");
    };

    let token = resolve_required_secret(
        bot_token.as_deref(),
        bot_token_env.as_deref(),
        bot_token_secret.as_deref(),
        "Telegram bot token",
    )?;
    let client = http_client(target)?;
    let text = merged_text(msg);
    let send_message_url = format!("https://api.telegram.org/bot{token}/sendMessage");

    client
        .post(&send_message_url)
        .json(&json!({
            "chat_id": chat_id,
            "text": text,
        }))
        .send()
        .context("failed to POST Telegram Bot API sendMessage")?
        .error_for_status()
        .context("Telegram Bot API sendMessage returned error status")?;

    if msg.attach_log {
        match read_log_attachment(&msg.log_path, max_attachment_bytes(config))? {
            AttachmentOutcome::Ready(attachment) => {
                let send_document_url = format!("https://api.telegram.org/bot{token}/sendDocument");
                let part = reqwest::blocking::multipart::Part::bytes(attachment.bytes)
                    .file_name(attachment.filename)
                    .mime_str("text/plain")
                    .context("failed to build Telegram document part")?;
                client
                    .post(&send_document_url)
                    .multipart(
                        reqwest::blocking::multipart::Form::new()
                            .text("chat_id", chat_id.clone())
                            .part("document", part),
                    )
                    .send()
                    .context("failed to POST Telegram Bot API sendDocument")?
                    .error_for_status()
                    .context("Telegram Bot API sendDocument returned error status")?;
            }
            AttachmentOutcome::Skipped(reason) => {
                eprintln!("Warning: {reason}");
            }
        }
    }

    Ok(())
}
