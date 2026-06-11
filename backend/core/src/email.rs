//! Message-class send wrappers + packaged-default email templates (TS-M2-E2).
//!
//! Two thin wrappers over the [`crate::mailer::Mailer`] port apply the
//! **anti-loop header** option sets (BS-040.22) so an autoresponse / notification
//! cannot ping-pong with a remote autoresponder:
//!
//! * [`send_autoreply`] — the **autoreply** option set (new-ticket autoresponse).
//! * [`send_notice`] — the **notice** option set (staff-reply notification).
//!
//! Plus the two **packaged-default templates** M2 needs (FS-040.10 / BS-040.13):
//! the new-ticket autoresponse and the staff-reply notification, each a
//! `subject` + `body` carrying `%{ticket.*}` / `%{url}` tokens, rendered through
//! the Epic-C [`crate::variable::VariableReplacer`]. DEVIATION D3: these embedded
//! defaults stand in for the template-set admin UI (deferred to M4/M5).
//!
//! @implements FS-040.13: specialized send wrappers (autoreply / notice).
//! @implements BS-040.22: message-class anti-loop headers.
//! @implements FS-040.10 / BS-040.13: packaged-default subject + body fallback.
//! @implements FS-011.12: new-ticket autoresponse template.
//! @implements FS-021.3: staff-reply notification template.

use crate::mailer::{MailError, Mailer, OutboundMail};
use crate::variable::VariableReplacer;

/// The **autoreply** anti-loop header set (BS-040.22): `Precedence: auto_reply`,
/// `X-Autoreply: yes`, `X-Auto-Response-Suppress: DR, RN, OOF, AutoReply`,
/// `Auto-Submitted: auto-replied`.
pub const AUTOREPLY_HEADERS: &[(&str, &str)] = &[
    ("Precedence", "auto_reply"),
    ("X-Autoreply", "yes"),
    ("X-Auto-Response-Suppress", "DR, RN, OOF, AutoReply"),
    ("Auto-Submitted", "auto-replied"),
];

/// The **notice** anti-loop header set (BS-040.22):
/// `X-Auto-Response-Suppress: OOF, AutoReply`, `Auto-Submitted: auto-generated`.
pub const NOTICE_HEADERS: &[(&str, &str)] = &[
    ("X-Auto-Response-Suppress", "OOF, AutoReply"),
    ("Auto-Submitted", "auto-generated"),
];

/// Apply a header option set to an [`OutboundMail`].
fn with_headers(mut mail: OutboundMail, headers: &[(&str, &str)]) -> OutboundMail {
    for (name, value) in headers {
        mail = mail.with_header(*name, *value);
    }
    mail
}

/// Send `mail` as an **auto-reply** — applies the autoreply anti-loop headers
/// (BS-040.22) and dispatches through the active mailer transport.
///
/// @implements FS-040.13 / BS-040.22: autoreply send wrapper.
pub async fn send_autoreply(mailer: &dyn Mailer, mail: OutboundMail) -> Result<(), MailError> {
    mailer.send(with_headers(mail, AUTOREPLY_HEADERS)).await
}

/// Send `mail` as a **notice** (alert/notification) — applies the notice
/// anti-loop headers (BS-040.22) and dispatches through the active transport.
///
/// @implements FS-040.13 / BS-040.22: notice send wrapper.
pub async fn send_notice(mailer: &dyn Mailer, mail: OutboundMail) -> Result<(), MailError> {
    mailer.send(with_headers(mail, NOTICE_HEADERS)).await
}

/// A packaged-default template: a `subject` + `body` pair carrying `%{...}`
/// tokens, resolved through [`VariableReplacer`] (FS-040.10 / BS-040.13).
#[derive(Debug, Clone, Copy)]
pub struct EmailTemplate {
    /// The subject line with `%{...}` tokens.
    pub subject: &'static str,
    /// The body with `%{...}` tokens.
    pub body: &'static str,
}

impl EmailTemplate {
    /// Render this template's subject + body for a ticket context, returning the
    /// substituted `(subject, body)`. Unknown tokens are preserved verbatim by
    /// the engine (BS-040.17) so an authoring mistake stays visible.
    #[must_use]
    pub fn render(&self, replacer: &VariableReplacer) -> (String, String) {
        let rendered = replacer.render_all(&[self.subject, self.body]);
        (rendered[0].clone(), rendered[1].clone())
    }
}

/// New-ticket **autoresponse** packaged default (FS-011.12). Sent to the
/// requester after a ticket is opened. References the new ticket number + a
/// `%{url}` link back to the helpdesk.
///
/// @implements FS-011.12: new-ticket autoresponse default content.
pub const NEW_TICKET_AUTORESPONSE: EmailTemplate = EmailTemplate {
    subject: "[#%{ticket.number}] %{ticket.subject}",
    body: "Dear %{ticket.name},\n\n\
        Thank you for contacting us. A support ticket has been opened for your \
        request and our team will respond shortly.\n\n\
        Ticket number: %{ticket.number}\n\
        Subject: %{ticket.subject}\n\
        Status: %{ticket.status}\n\n\
        You can view your ticket online at:\n%{url}\n\n\
        Please keep the ticket number above for your records.\n\n\
        Regards,\n\
        Support",
};

/// Staff-reply **notification** packaged default (FS-021.3). Sent to the
/// requester after a staff agent replies. References the ticket number + a
/// `%{url}` link to view the reply.
///
/// @implements FS-021.3: staff-reply notification default content.
pub const STAFF_REPLY_NOTIFICATION: EmailTemplate = EmailTemplate {
    subject: "[#%{ticket.number}] %{ticket.subject}",
    body: "Dear %{ticket.name},\n\n\
        A staff member has posted a reply to your support ticket.\n\n\
        Ticket number: %{ticket.number}\n\
        Subject: %{ticket.subject}\n\n\
        To read the reply and respond, view your ticket online at:\n%{url}\n\n\
        Regards,\n\
        Support",
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mailer::StubMailer;
    use crate::variable::{VarContext, VariableReplacer};

    fn replacer() -> VariableReplacer {
        let ctx = VarContext::ticket(
            "123456",
            "Mia Example",
            "Printer broken",
            "mia@example.com",
            "open",
            "2026-06-11T10:00:00Z",
            "Support",
        );
        VariableReplacer::new(ctx, "http://localhost:3702")
    }

    /// AC-1 anchor (header values asserted end-to-end in the api smtp test):
    /// send_autoreply applies the four autoreply headers.
    #[tokio::test]
    async fn send_autoreply_applies_autoreply_headers() {
        let stub = StubMailer::new();
        send_autoreply(&stub, OutboundMail::new("a@b.c", "s", "b"))
            .await
            .unwrap();
        let m = &stub.recorded()[0];
        for (name, value) in AUTOREPLY_HEADERS {
            assert!(
                m.headers
                    .iter()
                    .any(|(n, v)| n == name && v == value),
                "autoreply header {name}: {value} present"
            );
        }
    }

    /// AC-2 anchor: send_notice applies the two notice headers (and none of the
    /// autoreply-only headers like X-Autoreply).
    #[tokio::test]
    async fn send_notice_applies_notice_headers() {
        let stub = StubMailer::new();
        send_notice(&stub, OutboundMail::new("a@b.c", "s", "b"))
            .await
            .unwrap();
        let m = &stub.recorded()[0];
        for (name, value) in NOTICE_HEADERS {
            assert!(
                m.headers.iter().any(|(n, v)| n == name && v == value),
                "notice header {name}: {value} present"
            );
        }
        assert!(
            !m.headers.iter().any(|(n, _)| n == "X-Autoreply"),
            "notice send carries no X-Autoreply header"
        );
    }

    /// AC-3 anchor: both packaged templates render with tokens substituted — the
    /// ticket number appears and no literal `%{...}` remains.
    #[test]
    fn templates_render_with_tokens_substituted() {
        let r = replacer();
        for tpl in [NEW_TICKET_AUTORESPONSE, STAFF_REPLY_NOTIFICATION] {
            let (subject, body) = tpl.render(&r);
            assert!(subject.contains("123456"), "subject references the ticket number");
            assert!(body.contains("123456"), "body references the ticket number");
            assert!(body.contains("http://localhost:3702"), "body resolves %{{url}}");
            assert!(!subject.contains("%{"), "no literal token in subject");
            assert!(!body.contains("%{"), "no literal token in body");
        }
    }
}
