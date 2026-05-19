use anyhow::{bail, Context, Result};
use serde_json::json;

use crate::config::NotifyTargetConfig;
use crate::notify::{http_client, resolve_required_secret, NotifyMessage};

pub fn send(target: &NotifyTargetConfig, msg: &NotifyMessage) -> Result<()> {
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
    let url = format!("https://api.telegram.org/bot{token}/sendMessage");
    let text = format!("{}\n\n{}", msg.title, msg.body);

    http_client(target)?
        .post(&url)
        .json(&json!({
            "chat_id": chat_id,
            "text": text,
        }))
        .send()
        .context("failed to POST Telegram Bot API")?
        .error_for_status()
        .context("Telegram Bot API returned error status")?;
    Ok(())
}
