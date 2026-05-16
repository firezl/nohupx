use anyhow::{bail, Context, Result};

use crate::config::NotifyTargetConfig;
use crate::notify::NotifyMessage;

pub fn send(target: &NotifyTargetConfig, msg: &NotifyMessage) -> Result<()> {
    let NotifyTargetConfig::Ntfy { url, .. } = target else {
        bail!("not an ntfy target");
    };

    reqwest::blocking::Client::new()
        .post(url)
        .header("Title", &msg.title)
        .body(msg.body.clone())
        .send()
        .with_context(|| format!("failed to POST ntfy notification {url}"))?
        .error_for_status()
        .with_context(|| format!("ntfy returned error status for {url}"))?;
    Ok(())
}
