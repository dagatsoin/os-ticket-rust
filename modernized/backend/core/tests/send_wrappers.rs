//! Integration tests for the TS-M2-E2 send wrappers + packaged templates.
//!
//! The header assertions (AC-1/AC-2) drive the **real** `SmtpMailer` against a
//! running Mailpit and read the delivered headers via the Mailpit REST API; they
//! **skip-pass when `MAILPIT_URL` is unset** (ROADMAP M2 §5). The template
//! rendering case (AC-3) is pure and always runs.
//!
//! ```sh
//! docker compose up -d mailpit
//! MAILPIT_URL=http://localhost:3705 \
//!   cargo test -p ost_core --test send_wrappers
//! ```
//!
//! @implements BS-040.22: autoreply / notice anti-loop headers (AC-1/AC-2).
//! @implements FS-040.10 / FS-040.11: packaged templates render substituted (AC-3).

use ost_core::email::{send_autoreply, send_notice, NEW_TICKET_AUTORESPONSE, STAFF_REPLY_NOTIFICATION};
use ost_core::variable::{VarContext, VariableReplacer};
use ost_core::{OutboundMail, SmtpConfig, SmtpMailer, SmtpTls};

const SMTP_HOST: &str = "localhost";
const SMTP_PORT: u16 = 3704;
const FROM: &str = "support@example.com";

fn mailpit_url() -> Option<String> {
    std::env::var("MAILPIT_URL").ok().filter(|s| !s.trim().is_empty())
}

fn smtp_mailer() -> SmtpMailer {
    SmtpMailer::new(SmtpConfig {
        host: SMTP_HOST.to_string(),
        port: SMTP_PORT,
        from: FROM.to_string(),
        from_name: None,
        user: None,
        pass: None,
        tls: SmtpTls::None,
    })
    .expect("build SmtpMailer")
}

fn unique_to(tag: &str) -> String {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{tag}-{nonce}@example.com")
}

/// Poll for OUR message (by unique recipient) and return its `/headers` map.
async fn delivered_headers(base: &str, to: &str) -> serde_json::Value {
    for _ in 0..20 {
        let list: serde_json::Value = reqwest::get(format!("{base}/api/v1/messages"))
            .await
            .expect("list")
            .json()
            .await
            .expect("json");
        if let Some(found) = list["messages"].as_array().and_then(|arr| {
            arr.iter().find(|m| {
                m["To"]
                    .as_array()
                    .is_some_and(|tos| tos.iter().any(|t| t["Address"].as_str() == Some(to)))
            })
        }) {
            let id = found["ID"].as_str().unwrap_or_default();
            return reqwest::get(format!("{base}/api/v1/message/{id}/headers"))
                .await
                .expect("headers")
                .json()
                .await
                .expect("headers json");
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!("message to {to} was not delivered");
}

/// Assert a header is present with the exact value (Mailpit returns
/// `{ "Header-Name": ["value", ...] }`).
fn header_eq(headers: &serde_json::Value, name: &str, value: &str) -> bool {
    headers
        .get(name)
        .and_then(|v| v.as_array())
        .is_some_and(|arr| arr.iter().any(|x| x.as_str() == Some(value)))
}

/// AC-1: an auto-reply send carries the autoreply anti-loop headers.
#[tokio::test]
async fn autoreply_carries_anti_loop_headers() {
    let Some(base) = mailpit_url() else {
        eprintln!("MAILPIT_URL unset; skipping autoreply_carries_anti_loop_headers");
        return;
    };
    let to = unique_to("autoreply");
    send_autoreply(&smtp_mailer(), OutboundMail::new(&to, "Auto", "body"))
        .await
        .expect("send_autoreply");

    let h = delivered_headers(&base, &to).await;
    assert!(header_eq(&h, "Precedence", "auto_reply"), "Precedence: auto_reply");
    assert!(header_eq(&h, "X-Autoreply", "yes"), "X-Autoreply: yes");
    assert!(
        header_eq(&h, "X-Auto-Response-Suppress", "DR, RN, OOF, AutoReply"),
        "X-Auto-Response-Suppress"
    );
    assert!(
        header_eq(&h, "Auto-Submitted", "auto-replied"),
        "Auto-Submitted: auto-replied"
    );
}

/// AC-2: a notice send carries the notice anti-loop headers.
#[tokio::test]
async fn notice_carries_anti_loop_headers() {
    let Some(base) = mailpit_url() else {
        eprintln!("MAILPIT_URL unset; skipping notice_carries_anti_loop_headers");
        return;
    };
    let to = unique_to("notice");
    send_notice(&smtp_mailer(), OutboundMail::new(&to, "Notice", "body"))
        .await
        .expect("send_notice");

    let h = delivered_headers(&base, &to).await;
    assert!(
        header_eq(&h, "X-Auto-Response-Suppress", "OOF, AutoReply"),
        "X-Auto-Response-Suppress: OOF, AutoReply"
    );
    assert!(
        header_eq(&h, "Auto-Submitted", "auto-generated"),
        "Auto-Submitted: auto-generated"
    );
}

/// AC-3: the autoresponse + notification templates render with tokens
/// substituted — each subject + body contain the ticket number and no literal
/// `%{...}` remains. Pure (no Mailpit), always runs.
#[test]
fn templates_render_substituted() {
    let ctx = VarContext::ticket(
        "654321",
        "Mia",
        "Help",
        "mia@example.com",
        "open",
        "2026-06-11T10:00:00Z",
        "Support",
    );
    let r = VariableReplacer::new(ctx, "http://localhost:3702");
    for tpl in [NEW_TICKET_AUTORESPONSE, STAFF_REPLY_NOTIFICATION] {
        let (subject, body) = tpl.render(&r);
        assert!(subject.contains("654321"), "subject has ticket number");
        assert!(body.contains("654321"), "body has ticket number");
        assert!(!subject.contains("%{"), "no literal token in subject");
        assert!(!body.contains("%{"), "no literal token in body");
    }
}
