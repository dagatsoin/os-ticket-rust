//! Mailer **port** + a recording stub and a real SMTP transport.
//!
//! @implements FS-040: outbound mail dispatch — modelled as a port (trait) so
//!   the transport swaps without touching callers. The M1 implementation is a
//!   **stub** ([`StubMailer`]) that dispatches nothing but **records** every
//!   intended send (QA reads them via `GET /api/dev/mailbox`). TS-M2-E1 adds the
//!   real [`SmtpMailer`] (DEVIATION D3): a single env-driven `lettre` transport,
//!   selected at startup when `SMTP_HOST` is set, otherwise the stub is retained.
//! @implements FS-040.12: SMTP transport behind the mailer port.
//!
//! The recording store is in-memory and shared (an `Arc<Mutex<…>>`), held in the
//! long-lived application state so recorded sends survive for the process
//! lifetime — long enough for QA to read them back.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde::Serialize;

/// One intended outbound message captured by the stub mailer or delivered by the
/// SMTP transport.
///
/// `from` is **optional**: when `None`, the transport supplies its configured
/// default envelope/`From` (`SMTP_FROM`). `headers` carries extra RFC-5322 header
/// lines — used by the E2 wrappers for the anti-loop headers (BS-040.22); empty
/// for a plain send. Text-only body (KL-040.1): a single `text/plain` part.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OutboundMail {
    /// Recipient address.
    pub to: String,
    /// Subject line.
    pub subject: String,
    /// Message body (plain text / sanitised).
    pub body: String,
    /// Optional explicit `From` (else the transport default `SMTP_FROM`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// Extra header lines `(name, value)` — the anti-loop headers (E2/BS-040.22).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub headers: Vec<(String, String)>,
}

impl OutboundMail {
    /// A plain message with no extra headers and the transport default `From`.
    pub fn new(
        to: impl Into<String>,
        subject: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self {
            to: to.into(),
            subject: subject.into(),
            body: body.into(),
            from: None,
            headers: Vec::new(),
        }
    }

    /// Attach an extra header line (chainable). Used by the E2 wrappers.
    #[must_use]
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }
}

/// The mailer port. Implementations dispatch (SMTP) or record (stub) mail.
///
/// `async` (the real SMTP transport performs network I/O); `Send + Sync` so it
/// lives behind an `Arc` in shared Axum state.
#[async_trait]
pub trait Mailer: Send + Sync {
    /// Send (or record) one message.
    async fn send(&self, mail: OutboundMail) -> Result<(), MailError>;
}

/// Errors a mailer transport may surface. The stub never errors.
#[derive(Debug, thiserror::Error)]
pub enum MailError {
    #[error("mail transport error: {0}")]
    Transport(String),
}

/// The stub mailer: logs each send and **records** it in memory, dispatching
/// nothing. Cloneable — clones share the same recording store.
#[derive(Clone, Default)]
pub struct StubMailer {
    recorded: Arc<Mutex<Vec<OutboundMail>>>,
}

impl StubMailer {
    /// A fresh stub mailer with an empty recorded-sends store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot of everything recorded so far (oldest first). Backs the
    /// `GET /api/dev/mailbox` dev endpoint.
    pub fn recorded(&self) -> Vec<OutboundMail> {
        self.recorded.lock().expect("mailer mutex not poisoned").clone()
    }

    /// Number of recorded sends.
    pub fn len(&self) -> usize {
        self.recorded.lock().expect("mailer mutex not poisoned").len()
    }

    /// Whether nothing has been recorded yet.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[async_trait]
impl Mailer for StubMailer {
    /// @implements FS-040: record the intended send; dispatch nothing.
    async fn send(&self, mail: OutboundMail) -> Result<(), MailError> {
        tracing::info!(
            to = %mail.to,
            subject = %mail.subject,
            "stub mailer recorded an intended send (no real dispatch)"
        );
        self.recorded
            .lock()
            .expect("mailer mutex not poisoned")
            .push(mail);
        Ok(())
    }
}

/// TLS mode for the SMTP transport connection.
///
/// - `Implicit` — implicit TLS / SMTPS: the whole connection is wrapped in TLS
///   from the first byte (the standard for port **465**).
/// - `StartTls` — connect in plaintext then upgrade to TLS via the `STARTTLS`
///   command (the standard for port **587**).
/// - `None` — no TLS, plaintext throughout (the local **Mailpit** relay on 3704,
///   or bare port 25).
///
/// @implements FS-040.12 / §D3: SMTP transport TLS selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmtpTls {
    /// Implicit TLS / SMTPS (port 465).
    Implicit,
    /// STARTTLS upgrade (port 587).
    StartTls,
    /// Plaintext, no TLS (Mailpit / port 25).
    None,
}

impl SmtpTls {
    /// Resolve the effective TLS mode from an optional explicit `SMTP_TLS` value
    /// and the SMTP port.
    ///
    /// An explicit `SMTP_TLS` (`implicit` | `starttls` | `none`, case-insensitive,
    /// surrounding whitespace ignored) **wins**. When it is absent (or an
    /// unrecognised value), the mode is inferred from the port: `465 → Implicit`,
    /// `587 → StartTls`, everything else (25, 3704 Mailpit, …) `→ None`. This keeps
    /// dev/Mailpit plaintext by default while making a `465` relay use TLS.
    ///
    /// Pure — no environment or network access; unit-tested.
    ///
    /// @implements FS-040.12 / §D3: SMTP transport TLS selection.
    #[must_use]
    pub fn resolve(tls_env: Option<&str>, port: u16) -> Self {
        match tls_env.map(|s| s.trim().to_ascii_lowercase()).as_deref() {
            Some("implicit") => SmtpTls::Implicit,
            Some("starttls") => SmtpTls::StartTls,
            Some("none") => SmtpTls::None,
            // Unset or unrecognised ⇒ infer from the port.
            _ => match port {
                465 => SmtpTls::Implicit,
                587 => SmtpTls::StartTls,
                _ => SmtpTls::None,
            },
        }
    }
}

/// Env-driven SMTP configuration for the real transport (DEVIATION D3, §D3).
///
/// `auth` (user+pass) is **optional** — Mailpit accepts unauthenticated SMTP, so
/// dev runs may set only host/port/from. `from_name` is an optional display name
/// for the `From` header. `tls` selects the connection security (implicit / STARTTLS
/// / none), resolved from `SMTP_TLS` with a port-based default.
#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub from: String,
    pub from_name: Option<String>,
    pub user: Option<String>,
    pub pass: Option<String>,
    pub tls: SmtpTls,
}

impl SmtpConfig {
    /// Read the SMTP config from the environment, returning `None` when
    /// `SMTP_HOST` is unset (→ the caller keeps the stub mailer, §D3).
    ///
    /// - `SMTP_HOST` (required to activate), `SMTP_PORT` (default 25),
    /// - `SMTP_FROM` (required when active; falls back to `support@localhost`),
    /// - optional `SMTP_FROM_NAME`, `SMTP_USER`, `SMTP_PASS` (auth optional),
    /// - optional `SMTP_TLS` (`implicit` | `starttls` | `none`); when unset the
    ///   TLS mode is inferred from the port (465 → implicit, 587 → starttls,
    ///   else none) via [`SmtpTls::resolve`].
    ///
    /// @implements FS-040.12 / §D3: env-driven single SMTP transport.
    pub fn from_env() -> Option<Self> {
        let host = std::env::var("SMTP_HOST").ok().filter(|s| !s.trim().is_empty())?;
        let port = std::env::var("SMTP_PORT")
            .ok()
            .and_then(|s| s.trim().parse::<u16>().ok())
            .unwrap_or(25);
        let from = std::env::var("SMTP_FROM")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "support@localhost".to_string());
        let from_name = std::env::var("SMTP_FROM_NAME").ok().filter(|s| !s.trim().is_empty());
        let user = std::env::var("SMTP_USER").ok().filter(|s| !s.trim().is_empty());
        let pass = std::env::var("SMTP_PASS").ok().filter(|s| !s.trim().is_empty());
        let tls_env = std::env::var("SMTP_TLS").ok();
        let tls = SmtpTls::resolve(tls_env.as_deref(), port);
        Some(Self {
            host,
            port,
            from,
            from_name,
            user,
            pass,
            tls,
        })
    }
}

/// The real SMTP transport (TS-M2-E1) built on `lettre` (tokio1 + rustls).
///
/// Plain-text only (KL-040.1): one `text/plain` body part. The configured
/// `SMTP_FROM` is the default `From`; a message may override it via
/// [`OutboundMail::from`]. Extra [`OutboundMail::headers`] are emitted as raw
/// header lines — the E2 anti-loop headers ride through here.
///
/// @implements FS-040.12: SMTP delivery behind the mailer port (DEVIATION D3).
#[derive(Clone)]
pub struct SmtpMailer {
    transport: lettre::AsyncSmtpTransport<lettre::Tokio1Executor>,
    config: SmtpConfig,
}

impl SmtpMailer {
    /// Build the transport from [`SmtpConfig`], honouring [`SmtpConfig::tls`]:
    ///
    /// - [`SmtpTls::Implicit`] — `relay()` (implicit TLS / SMTPS, rustls via the
    ///   `tokio1-rustls-tls` feature), for a prod relay on port 465.
    /// - [`SmtpTls::StartTls`] — `starttls_relay()` (plaintext → STARTTLS upgrade),
    ///   for a submission relay on port 587.
    /// - [`SmtpTls::None`] — `builder_dangerous()` + `Tls::None` (plaintext), for
    ///   the local Mailpit relay (the M1 behaviour, unchanged).
    ///
    /// Credentials are attached only when both user and pass are present (Mailpit
    /// needs none; a prod relay typically sets `SMTP_USER`/`SMTP_PASS`).
    ///
    /// @implements FS-040.12 / §D3: SMTP transport built per TLS mode.
    pub fn new(config: SmtpConfig) -> Result<Self, MailError> {
        use lettre::transport::smtp::authentication::Credentials;
        use lettre::transport::smtp::client::Tls;
        use lettre::{AsyncSmtpTransport, Tokio1Executor};

        let mut builder = match config.tls {
            SmtpTls::Implicit => AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host)
                .map_err(|e| MailError::Transport(format!("smtp implicit-TLS relay init: {e}")))?
                .port(config.port),
            SmtpTls::StartTls => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.host)
                .map_err(|e| MailError::Transport(format!("smtp STARTTLS relay init: {e}")))?
                .port(config.port),
            SmtpTls::None => AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.host)
                .port(config.port)
                .tls(Tls::None),
        };

        if let (Some(user), Some(pass)) = (config.user.clone(), config.pass.clone()) {
            builder = builder.credentials(Credentials::new(user, pass));
        }

        Ok(Self {
            transport: builder.build(),
            config,
        })
    }
}

#[async_trait]
impl Mailer for SmtpMailer {
    /// @implements FS-040.12 / KL-040.1: deliver one plain-text message via SMTP.
    async fn send(&self, mail: OutboundMail) -> Result<(), MailError> {
        use lettre::message::header::{HeaderName, HeaderValue};
        use lettre::message::Mailbox;
        use lettre::AsyncTransport;

        let from_addr = mail.from.as_deref().unwrap_or(&self.config.from);
        let from: Mailbox = match &self.config.from_name {
            Some(name) if mail.from.is_none() => format!("{name} <{from_addr}>")
                .parse()
                .map_err(|e| MailError::Transport(format!("bad From: {e}")))?,
            _ => from_addr
                .parse()
                .map_err(|e| MailError::Transport(format!("bad From: {e}")))?,
        };
        let to: Mailbox = mail
            .to
            .parse()
            .map_err(|e| MailError::Transport(format!("bad To: {e}")))?;

        let mut message = lettre::Message::builder()
            .from(from)
            .to(to)
            .subject(mail.subject.clone())
            .body(mail.body.clone())
            .map_err(|e| MailError::Transport(format!("build message: {e}")))?;

        // Emit the extra header lines (anti-loop headers, BS-040.22) as raw
        // RFC-5322 headers via lettre's untyped header mechanism.
        for (name, value) in &mail.headers {
            let hname = HeaderName::new_from_ascii(name.clone())
                .map_err(|e| MailError::Transport(format!("bad header name {name}: {e}")))?;
            message
                .headers_mut()
                .insert_raw(HeaderValue::new(hname, value.clone()));
        }

        self.transport
            .send(message)
            .await
            .map_err(|e| MailError::Transport(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> OutboundMail {
        OutboundMail::new(
            "user@example.com",
            "Ticket #123456 updated",
            "A staff member replied.",
        )
    }

    /// AC-5 anchor: the stub mailer records an intended send without dispatching
    /// real mail. Named so `cargo test -p ost_core stub_mailer` selects it.
    #[tokio::test]
    async fn stub_mailer_records_intended_send_dispatching_nothing() {
        let mailer = StubMailer::new();
        let before = mailer.len();
        mailer.send(sample()).await.unwrap();
        assert_eq!(mailer.len(), before + 1, "recorded-sends store grows by one");
    }

    #[tokio::test]
    async fn stub_mailer_records_without_dispatching() {
        let mailer = StubMailer::new();
        assert!(mailer.is_empty());

        mailer.send(sample()).await.unwrap();

        assert_eq!(mailer.len(), 1);
        let recorded = mailer.recorded();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0], sample());
    }

    #[tokio::test]
    async fn stub_mailer_clones_share_one_store() {
        let mailer = StubMailer::new();
        let clone = mailer.clone();
        clone.send(sample()).await.unwrap();
        assert_eq!(mailer.len(), 1);
    }

    #[tokio::test]
    async fn stub_mailer_serialises_recorded_send_to_json() {
        let mailer = StubMailer::new();
        mailer.send(sample()).await.unwrap();
        let json = serde_json::to_value(mailer.recorded()).unwrap();
        assert_eq!(json[0]["to"], "user@example.com");
        assert_eq!(json[0]["subject"], "Ticket #123456 updated");
    }

    /// §D3: `SMTP_HOST` unset ⇒ no SMTP config (caller keeps the stub).
    #[test]
    fn smtp_config_absent_without_host() {
        // Guard: ensure SMTP_HOST is not set in this process for the assertion.
        if std::env::var("SMTP_HOST").is_ok() {
            return; // skip when the env is configured (e.g. live E2E shell)
        }
        assert!(SmtpConfig::from_env().is_none());
    }

    /// §D3 TLS: with `SMTP_TLS` unset, the mode is inferred from the port —
    /// 465 → implicit, 587 → starttls, everything else (25, 3704 Mailpit) → none.
    #[test]
    fn smtp_tls_port_default_inference() {
        assert_eq!(SmtpTls::resolve(None, 465), SmtpTls::Implicit);
        assert_eq!(SmtpTls::resolve(None, 587), SmtpTls::StartTls);
        assert_eq!(SmtpTls::resolve(None, 3704), SmtpTls::None); // Mailpit
        assert_eq!(SmtpTls::resolve(None, 25), SmtpTls::None);
        assert_eq!(SmtpTls::resolve(None, 2525), SmtpTls::None);
    }

    /// §D3 TLS: an explicit `SMTP_TLS` overrides the port-based default, and is
    /// case-insensitive / whitespace-trimmed.
    #[test]
    fn smtp_tls_explicit_overrides_port_default() {
        // Explicit `none` on 465 (which would otherwise infer implicit).
        assert_eq!(SmtpTls::resolve(Some("none"), 465), SmtpTls::None);
        // Explicit `implicit` on 3704 (which would otherwise infer none).
        assert_eq!(SmtpTls::resolve(Some("implicit"), 3704), SmtpTls::Implicit);
        // Explicit `starttls` on 465 (override the implicit default).
        assert_eq!(SmtpTls::resolve(Some("starttls"), 465), SmtpTls::StartTls);
        // Case-insensitive + surrounding whitespace tolerated.
        assert_eq!(SmtpTls::resolve(Some(" Implicit "), 25), SmtpTls::Implicit);
        assert_eq!(SmtpTls::resolve(Some("STARTTLS"), 25), SmtpTls::StartTls);
    }

    /// §D3 TLS: an empty / unrecognised `SMTP_TLS` falls back to the port default
    /// rather than erroring.
    #[test]
    fn smtp_tls_unrecognised_falls_back_to_port() {
        assert_eq!(SmtpTls::resolve(Some(""), 465), SmtpTls::Implicit);
        assert_eq!(SmtpTls::resolve(Some("garbage"), 587), SmtpTls::StartTls);
        assert_eq!(SmtpTls::resolve(Some("ssl"), 3704), SmtpTls::None);
    }

    #[test]
    fn outbound_mail_with_header_appends() {
        let m = OutboundMail::new("a@b.c", "s", "b")
            .with_header("Precedence", "auto_reply")
            .with_header("Auto-Submitted", "auto-replied");
        assert_eq!(m.headers.len(), 2);
        assert_eq!(m.headers[0], ("Precedence".into(), "auto_reply".into()));
    }
}
