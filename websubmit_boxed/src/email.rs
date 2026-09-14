use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

use crate::config::Config;

/// Send a plain text email to every recipient over the configured SMTP server.
///
/// The transport is chosen from `smtp_port`, since speaking implicit TLS to a
/// STARTTLS port (or the reverse) fails in the handshake:
///   * 465 - implicit TLS, encrypted from the first byte
///   * 25  - plaintext relay, no TLS
///   * any other port (587 submission) - STARTTLS
///
/// Credentials are sent only when `smtp_user` is non-empty, so an
/// unauthenticated relay can be used.
pub(crate) fn send(
    _log: slog::Logger,
    config: &Config,
    reply_to: Option<String>,
    recipients: Vec<String>,
    subject: String,
    text: String,
) -> Result<(), lettre::transport::smtp::Error> {
    let from: Mailbox = match config.smtp_from.parse() {
        Ok(from) => from,
        Err(e) => {
            println!("invalid smtp_from address '{}': {}", config.smtp_from, e);
            return Ok(());
        }
    };

    let mut builder = match config.smtp_port {
        465 => SmtpTransport::relay(&config.smtp_server)?,
        25 => SmtpTransport::builder_dangerous(&config.smtp_server),
        _ => SmtpTransport::starttls_relay(&config.smtp_server)?,
    }
    .port(config.smtp_port);
    if !config.smtp_user.is_empty() {
        builder = builder.credentials(Credentials::new(
            config.smtp_user.clone(),
            config.smtp_password.clone(),
        ));
    }
    let mailer = builder.build();

    for recipient in recipients {
        let to: Mailbox = match recipient.parse() {
            Ok(to) => to,
            Err(e) => {
                println!("skipping invalid recipient '{}': {}", recipient, e);
                continue;
            }
        };
        let mut message = Message::builder().from(from.clone()).to(to);
        // Mail is always sent from the configured address, since an authenticated
        // server will not relay a forged sender; replies still reach the user.
        if let Some(reply_to) = reply_to.as_ref().and_then(|r| r.parse::<Mailbox>().ok()) {
            message = message.reply_to(reply_to);
        }
        let email = message
            .subject(subject.clone())
            .body(text.clone())
            .expect("failed to build email");
        mailer.send(&email)?;
    }

    Ok(())
}
