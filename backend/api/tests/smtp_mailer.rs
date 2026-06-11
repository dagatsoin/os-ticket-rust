//! Integration tests for the TS-M2-E1 SMTP mailer (DEVIATION D3).
//!
//! These exercise the **real** `SmtpMailer` against a running Mailpit relay and
//! assert delivery via the Mailpit REST API. They **skip-pass when `MAILPIT_URL`
//! is unset** (ROADMAP M2 §5 — same pattern as `TEST_DATABASE_URL`).
//!
//! ```sh
//! docker compose up -d mailpit
//! MAILPIT_URL=http://localhost:3705 \
//!   cargo test -p api --test smtp_mailer
//! ```
//!
//! @implements FS-040.12 / §D3: SMTP transport delivers via the mailer port.
//! @implements KL-040.1: text-only message (a `text/plain` part, no HTML).

use ost_core::{Mailer, OutboundMail, SmtpConfig, SmtpMailer};

/// The Mailpit REST base, or `None` ⇒ skip-pass (§5). Derives the SMTP port from
/// the web port by the fixed `3705 → 3704` (web → smtp) offset of the dev relay.
fn mailpit_url() -> Option<String> {
    std::env::var("MAILPIT_URL").ok().filter(|s| !s.trim().is_empty())
}

/// SMTP host/port the running Mailpit listens on (the dev compose mapping).
const SMTP_HOST: &str = "localhost";
const SMTP_PORT: u16 = 3704;
const FROM: &str = "support@example.com";

fn smtp_mailer() -> SmtpMailer {
    SmtpMailer::new(SmtpConfig {
        host: SMTP_HOST.to_string(),
        port: SMTP_PORT,
        from: FROM.to_string(),
        from_name: None,
        user: None,
        pass: None,
    })
    .expect("build SmtpMailer")
}

async fn empty_mailpit(base: &str) {
    let client = reqwest::Client::new();
    let _ = client
        .delete(format!("{base}/api/v1/messages"))
        .send()
        .await
        .expect("delete messages");
}

async fn messages(base: &str) -> serde_json::Value {
    reqwest::get(format!("{base}/api/v1/messages"))
        .await
        .expect("list messages")
        .json()
        .await
        .expect("messages json")
}

async fn message(base: &str, id: &str) -> serde_json::Value {
    reqwest::get(format!("{base}/api/v1/message/{id}"))
        .await
        .expect("get message")
        .json()
        .await
        .expect("message json")
}

/// AC-1: with the SMTP transport active, a send is delivered to Mailpit.
#[tokio::test]
async fn smtp_send_is_delivered_to_mailpit() {
    let Some(base) = mailpit_url() else {
        eprintln!("MAILPIT_URL unset; skipping smtp_send_is_delivered_to_mailpit");
        return;
    };
    // A unique recipient so a concurrently-running sibling test (shared Mailpit)
    // cannot inflate this assertion — we count only OUR delivered message.
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let to = format!("deliver-{nonce}@example.com");
    empty_mailpit(&base).await;

    smtp_mailer()
        .send(OutboundMail::new(
            &to,
            "Delivered probe",
            "hello from the SMTP mailer",
        ))
        .await
        .expect("send");

    // Poll Mailpit for the message addressed to our unique recipient.
    let mut found = 0;
    for _ in 0..20 {
        let msgs = messages(&base).await;
        found = msgs["messages"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter(|m| {
                        m["To"]
                            .as_array()
                            .is_some_and(|tos| tos.iter().any(|t| t["Address"].as_str() == Some(&to)))
                    })
                    .count()
            })
            .unwrap_or(0);
        if found >= 1 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    assert_eq!(found, 1, "exactly one message delivered to {to}");
}

/// AC-3: the delivered message is plain-text with the configured `From`.
#[tokio::test]
async fn smtp_message_is_plain_text_with_configured_from() {
    let Some(base) = mailpit_url() else {
        eprintln!("MAILPIT_URL unset; skipping smtp_message_is_plain_text_with_configured_from");
        return;
    };
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let to = format!("plain-{nonce}@example.com");
    empty_mailpit(&base).await;

    smtp_mailer()
        .send(OutboundMail::new(&to, "Plain text probe", "plain body"))
        .await
        .expect("send");

    // Locate OUR message by its unique recipient (shared Mailpit).
    let mut id = String::new();
    for _ in 0..20 {
        let msgs = messages(&base).await;
        if let Some(found) = msgs["messages"].as_array().and_then(|arr| {
            arr.iter().find(|m| {
                m["To"]
                    .as_array()
                    .is_some_and(|tos| tos.iter().any(|t| t["Address"].as_str() == Some(&to)))
            })
        }) {
            id = found["ID"].as_str().unwrap_or_default().to_string();
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    assert!(!id.is_empty(), "our message was delivered");
    let m = message(&base, &id).await;

    // From equals SMTP_FROM.
    assert_eq!(m["From"]["Address"].as_str(), Some(FROM));
    // text/plain present, no HTML part (KL-040.1).
    assert!(
        m["Text"].as_str().is_some_and(|t| t.contains("plain body")),
        "text/plain part present with the body"
    );
    assert!(
        m["HTML"].as_str().unwrap_or("").is_empty(),
        "no HTML part (text-only, KL-040.1)"
    );
}
