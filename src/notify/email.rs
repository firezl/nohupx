use anyhow::{bail, Context, Result};
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

use crate::config::NotifyTargetConfig;
use crate::notify::NotifyMessage;

pub fn send(target: &NotifyTargetConfig, msg: &NotifyMessage) -> Result<()> {
    let NotifyTargetConfig::Email {
        smtp_host,
        smtp_port,
        username,
        password_env,
        password,
        from,
        to,
        ..
    } = target
    else {
        bail!("not an email target");
    };

    let password = match password_env {
        Some(var) => {
            std::env::var(var).with_context(|| format!("environment variable {var} is not set"))?
        }
        None => password
            .clone()
            .context("email password is missing; set password_env or password")?,
    };

    let mut builder = Message::builder()
        .from(
            from.parse::<Mailbox>()
                .context("invalid email from address")?,
        )
        .subject(&msg.title);
    for recipient in to {
        builder = builder.to(recipient
            .parse::<Mailbox>()
            .with_context(|| format!("invalid recipient address {recipient}"))?);
    }
    let email = builder
        .body(msg.body.clone())
        .context("failed to build email message")?;

    let creds = Credentials::new(username.clone(), password);
    let mailer = SmtpTransport::starttls_relay(smtp_host)
        .with_context(|| format!("failed to configure SMTP relay {smtp_host}"))?
        .port(smtp_port.unwrap_or(587))
        .credentials(creds)
        .build();

    mailer.send(&email).context("failed to send email")?;
    Ok(())
}
