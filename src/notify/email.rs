use anyhow::{bail, Context, Result};
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::SmtpTransportBuilder;
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

    let port = smtp_port.unwrap_or(587);
    let creds = Credentials::new(username.clone(), password);
    let mailer = smtp_transport(smtp_host, port)?.credentials(creds).build();

    mailer.send(&email).context("failed to send email")?;
    Ok(())
}

fn smtp_transport(smtp_host: &str, port: u16) -> Result<SmtpTransportBuilder> {
    let builder = if use_implicit_tls(port) {
        SmtpTransport::relay(smtp_host)
            .with_context(|| format!("failed to configure SMTPS relay {smtp_host}:{port}"))?
    } else {
        SmtpTransport::starttls_relay(smtp_host).with_context(|| {
            format!("failed to configure STARTTLS SMTP relay {smtp_host}:{port}")
        })?
    };
    Ok(builder.port(port))
}

fn use_implicit_tls(port: u16) -> bool {
    port == 465
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_465_uses_implicit_tls() {
        assert!(use_implicit_tls(465));
    }

    #[test]
    fn port_587_uses_starttls() {
        assert!(!use_implicit_tls(587));
    }
}
