use anyhow::{bail, Context, Result};

use crate::config::{NotifyConfig, NotifyTargetConfig};
use crate::notify::attachment::{read_log_attachment, AttachmentOutcome};
use crate::notify::template::{max_attachment_bytes, RenderedMessage};
use crate::notify::{http_client, resolve_required_secret};

pub fn send(
    config: &NotifyConfig,
    target: &NotifyTargetConfig,
    msg: &RenderedMessage,
) -> Result<()> {
    let NotifyTargetConfig::Ntfy {
        url,
        url_env,
        url_secret,
        ..
    } = target
    else {
        bail!("not an ntfy target");
    };
    let url = resolve_required_secret(
        url.as_deref(),
        url_env.as_deref(),
        url_secret.as_deref(),
        "ntfy URL",
    )?;

    let client = http_client(target)?;
    let response = if msg.attach_log {
        match read_log_attachment(&msg.log_path, max_attachment_bytes(config))? {
            AttachmentOutcome::Ready(attachment) => {
                let part = reqwest::blocking::multipart::Part::bytes(attachment.bytes)
                    .file_name(attachment.filename)
                    .mime_str("text/plain")
                    .context("failed to build ntfy attachment part")?;
                client
                    .post(&url)
                    .header("Title", &msg.title)
                    .multipart(
                        reqwest::blocking::multipart::Form::new()
                            .text("message", msg.body.clone())
                            .part("file", part),
                    )
                    .send()
            }
            AttachmentOutcome::Skipped(reason) => {
                eprintln!("Warning: {reason}");
                client
                    .post(&url)
                    .header("Title", &msg.title)
                    .body(msg.body.clone())
                    .send()
            }
        }
    } else {
        client
            .post(&url)
            .header("Title", &msg.title)
            .body(msg.body.clone())
            .send()
    };

    response
        .with_context(|| format!("failed to POST ntfy notification {url}"))?
        .error_for_status()
        .with_context(|| format!("ntfy returned error status for {url}"))?;
    Ok(())
}
