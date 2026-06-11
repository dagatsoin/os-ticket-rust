//! Mailer **port** + a recording stub implementation (TS-M1-A4b).
//!
//! @implements FS-040: outbound mail dispatch — modelled here as a port (trait)
//!   so later milestones (M2/M5) swap in a real SMTP transport without touching
//!   callers. The M1 implementation is a **stub**: it dispatches nothing but
//!   **records** every intended send so QA can verify reply notifications via
//!   `GET /api/dev/mailbox` (the dev mailbox route, env-gated).
//!
//! The recording store is in-memory and shared (an `Arc<Mutex<…>>`), held in the
//! long-lived application state so recorded sends survive for the lifetime of
//! the process — long enough for QA to read them back.

use std::sync::{Arc, Mutex};

use serde::Serialize;

/// One intended outbound message captured by the stub mailer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OutboundMail {
    /// Recipient address.
    pub to: String,
    /// Subject line.
    pub subject: String,
    /// Message body (plain text / sanitised).
    pub body: String,
}

/// The mailer port. Implementations dispatch (or, for the stub, record) mail.
///
/// `Send + Sync` so it can live behind an `Arc` in shared Axum state.
pub trait Mailer: Send + Sync {
    /// Send (or record) one message. The stub never performs real I/O, so this
    /// is infallible there; the trait returns `Result` so a real SMTP transport
    /// can surface transport errors.
    fn send(&self, mail: OutboundMail) -> Result<(), MailError>;
}

/// Errors a (future, real) mailer transport may surface. The stub never errors.
#[derive(Debug, thiserror::Error)]
pub enum MailError {
    #[error("mail transport error: {0}")]
    Transport(String),
}

/// The M1 stub mailer: logs each send and **records** it in memory, dispatching
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

impl Mailer for StubMailer {
    /// @implements FS-040: record the intended send; dispatch nothing.
    fn send(&self, mail: OutboundMail) -> Result<(), MailError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> OutboundMail {
        OutboundMail {
            to: "user@example.com".into(),
            subject: "Ticket #123456 updated".into(),
            body: "A staff member replied.".into(),
        }
    }

    /// AC-5 anchor: the stub mailer records an intended send without dispatching
    /// real mail. Named so `cargo test -p ost_core stub_mailer` selects it.
    #[test]
    fn stub_mailer_records_intended_send_dispatching_nothing() {
        let mailer = StubMailer::new();
        let before = mailer.len();
        mailer.send(sample()).unwrap();
        assert_eq!(mailer.len(), before + 1, "recorded-sends store grows by one");
        // No real SMTP transport exists in the stub; nothing is dispatched.
    }

    #[test]
    fn stub_mailer_records_without_dispatching() {
        let mailer = StubMailer::new();
        assert!(mailer.is_empty());

        mailer.send(sample()).unwrap();

        // The recorded-sends store grew by exactly one; no transport was hit.
        assert_eq!(mailer.len(), 1);
        let recorded = mailer.recorded();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0], sample());
    }

    #[test]
    fn stub_mailer_clones_share_one_store() {
        let mailer = StubMailer::new();
        let clone = mailer.clone();
        clone.send(sample()).unwrap();
        // The original sees the clone's recorded send (shared Arc).
        assert_eq!(mailer.len(), 1);
    }

    #[test]
    fn stub_mailer_serialises_recorded_send_to_json() {
        let mailer = StubMailer::new();
        mailer.send(sample()).unwrap();
        let json = serde_json::to_value(mailer.recorded()).unwrap();
        assert_eq!(json[0]["to"], "user@example.com");
        assert_eq!(json[0]["subject"], "Ticket #123456 updated");
    }
}
