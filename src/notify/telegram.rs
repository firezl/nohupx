use anyhow::{bail, Context, Result};
use serde_json::json;

use crate::config::NotifyTargetConfig;
use crate::notify::NotifyMessage;

pub fn send(target: &NotifyTargetConfig, msg: &NotifyMessage) -> Result<()> {
    let NotifyTargetConfig::Telegram {
        bot_token_env,
        bot_token,
        chat_id,
        ..
    } = target
    else {
        bail!("not a telegram target");
    };

    let token = match bot_token_env {
        Some(var) => {
            std::env::var(var).with_context(|| format!("environment variable {var} is not set"))?
        }
        None => bot_token
            .clone()
            .context("telegram bot token is missing; set bot_token_env or bot_token")?,
    };
    let url = format!("https://api.telegram.org/bot{token}/sendMessage");
    let text = format!("{}\n\n{}", msg.title, msg.body);

    reqwest::blocking::Client::new()
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
