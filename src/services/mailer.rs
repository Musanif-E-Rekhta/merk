//! Outbound email transport.
//!
//! Defines the [`Mailer`] trait and ships three implementations selected by
//! `MAIL_TRANSPORT`:
//! - [`LogMailer`] (`log`, default) — writes the message to the tracing log.
//! - [`NoopMailer`] (`noop`) — silent; appropriate for tests.
//! - [`SmtpMailer`] (`smtp`) — delivers via [`lettre`] using the SMTP fields
//!   on [`AppConfig`].
//!
//! Body templates are inlined for now — both reset and verification mails
//! ship as plain-text only, since the URLs are the meat of the message.
//! Switch to `tera`/`askama` if marketing ever wants HTML.

use async_trait::async_trait;
use lettre::message::{Mailbox, Message};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::{AsyncSmtpTransport, AsyncTransport, Tokio1Executor};
use std::sync::Arc;
use tracing::info;

use crate::config::AppConfig;

#[derive(Debug, thiserror::Error)]
pub enum MailerError {
    #[error("mailer transport failed: {0}")]
    Transport(String),
    #[error("mailer config invalid: {0}")]
    Config(String),
}

#[async_trait]
pub trait Mailer: Send + Sync {
    async fn send_password_reset(
        &self,
        to_email: &str,
        reset_url: &str,
    ) -> Result<(), MailerError>;

    async fn send_email_verification(
        &self,
        to_email: &str,
        verify_url: &str,
    ) -> Result<(), MailerError>;
}

/// Logs the reset URL via `tracing` instead of delivering email.
pub struct LogMailer;

#[async_trait]
impl Mailer for LogMailer {
    async fn send_password_reset(
        &self,
        to_email: &str,
        reset_url: &str,
    ) -> Result<(), MailerError> {
        info!(target: "mailer", to = %to_email, %reset_url, "password reset email (log transport)");
        Ok(())
    }

    async fn send_email_verification(
        &self,
        to_email: &str,
        verify_url: &str,
    ) -> Result<(), MailerError> {
        info!(target: "mailer", to = %to_email, %verify_url, "email verification (log transport)");
        Ok(())
    }
}

/// Discards every send. Useful in tests that only care about token state.
pub struct NoopMailer;

#[async_trait]
impl Mailer for NoopMailer {
    async fn send_password_reset(
        &self,
        _to_email: &str,
        _reset_url: &str,
    ) -> Result<(), MailerError> {
        Ok(())
    }

    async fn send_email_verification(
        &self,
        _to_email: &str,
        _verify_url: &str,
    ) -> Result<(), MailerError> {
        Ok(())
    }
}

/// Delivers via lettre's async SMTP transport. Uses STARTTLS on port 587 by
/// default, implicit TLS on 465. Empty `user` disables auth (anonymous relay).
pub struct SmtpMailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
}

impl SmtpMailer {
    pub fn try_from_config(config: &AppConfig) -> Result<Self, MailerError> {
        if config.mail_smtp_host.is_empty() {
            return Err(MailerError::Config(
                "MAIL_TRANSPORT=smtp but MAIL_SMTP_HOST is empty".into(),
            ));
        }
        let from: Mailbox = config
            .mail_smtp_from
            .parse()
            .map_err(|e: lettre::address::AddressError| {
                MailerError::Config(format!("MAIL_SMTP_FROM invalid: {e}"))
            })?;

        let mut builder = if config.mail_smtp_port == 465 {
            // Implicit TLS — wrap the connection from the start.
            let tls = TlsParameters::new(config.mail_smtp_host.clone())
                .map_err(|e| MailerError::Config(format!("TLS setup failed: {e}")))?;
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.mail_smtp_host)
                .tls(Tls::Wrapper(tls))
                .port(465)
        } else {
            // STARTTLS upgrade on the submission port.
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.mail_smtp_host)
                .map_err(|e| MailerError::Config(format!("STARTTLS setup failed: {e}")))?
                .port(config.mail_smtp_port)
        };
        if !config.mail_smtp_user.is_empty() {
            builder = builder.credentials(Credentials::new(
                config.mail_smtp_user.clone(),
                config.mail_smtp_pass.clone(),
            ));
        }

        Ok(SmtpMailer {
            transport: builder.build(),
            from,
        })
    }

    async fn send(&self, to: &str, subject: &str, body: String) -> Result<(), MailerError> {
        let to: Mailbox = to
            .parse()
            .map_err(|e: lettre::address::AddressError| {
                MailerError::Transport(format!("recipient invalid: {e}"))
            })?;
        let message = Message::builder()
            .from(self.from.clone())
            .to(to)
            .subject(subject)
            .body(body)
            .map_err(|e| MailerError::Transport(format!("build message: {e}")))?;
        self.transport
            .send(message)
            .await
            .map_err(|e| MailerError::Transport(format!("send: {e}")))?;
        Ok(())
    }
}

#[async_trait]
impl Mailer for SmtpMailer {
    async fn send_password_reset(
        &self,
        to_email: &str,
        reset_url: &str,
    ) -> Result<(), MailerError> {
        let body = format!(
            "Hi,\n\n\
             We received a request to reset your Musanif password. \
             Open the link below in your browser to choose a new one. \
             The link expires in 30 minutes.\n\n\
             {reset_url}\n\n\
             If you didn't ask for a reset, you can ignore this message — \
             nothing has changed.\n",
        );
        self.send(to_email, "Reset your Musanif password", body).await
    }

    async fn send_email_verification(
        &self,
        to_email: &str,
        verify_url: &str,
    ) -> Result<(), MailerError> {
        let body = format!(
            "Welcome to Musanif!\n\n\
             To finish setting up your account, please confirm your email \
             by opening the link below.\n\n\
             {verify_url}\n\n\
             The link is good for 24 hours.\n",
        );
        self.send(to_email, "Confirm your Musanif email", body).await
    }
}

/// Build the mailer described by `config.mail_transport`.
///
/// Unrecognised transports — and SMTP configs that fail to construct — fall
/// back to `LogMailer` with a warning so a typo or a misconfigured relay
/// doesn't take down the auth flow at startup.
pub fn build(config: &AppConfig) -> Arc<dyn Mailer> {
    match config.mail_transport.as_str() {
        "noop" => Arc::new(NoopMailer),
        "log" => Arc::new(LogMailer),
        "smtp" => match SmtpMailer::try_from_config(config) {
            Ok(m) => Arc::new(m),
            Err(e) => {
                tracing::warn!(error = %e, "MAIL_TRANSPORT=smtp but mailer init failed; falling back to log");
                Arc::new(LogMailer)
            }
        },
        other => {
            tracing::warn!(
                transport = %other,
                "unknown MAIL_TRANSPORT; falling back to log mailer"
            );
            Arc::new(LogMailer)
        }
    }
}
