use anyhow::{bail, Context, Result};

use crate::config::NotifyTargetConfig;
use crate::notify::{http_client, resolve_required_secret, NotifyMessage};

pub fn send(target: &NotifyTargetConfig, msg: &NotifyMessage) -> Result<()> {
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

    http_client(target)?
        .post(&url)
        .header("Title", &msg.title)
        .body(msg.body.clone())
        .send()
        .with_context(|| format!("failed to POST ntfy notification {url}"))?
        .error_for_status()
        .with_context(|| format!("ntfy returned error status for {url}"))?;
    Ok(())
}
