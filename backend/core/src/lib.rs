//! `core` — shared domain primitives for the osTicket modernisation backend.
//!
//! Exposes the shared JSON error envelope ([`error`]) plus the TS-M1-A4a
//! pure-function tier: input [`validation`], HTML [`sanitize`]-ation, and
//! password [`hashing`] (argon2id). HTTP adapters stay thin by reusing these.
//!
//! TS-M1-A4b adds the stateful/middleware tier: DB-backed [`session`]s (both
//! realms), [`csrf`] double-submit verification, the generic [`permission`]
//! gate, and the stub [`mailer`] port.

pub mod attachment;
pub mod blob;
pub mod canned;
pub mod csrf;
pub mod error;
pub mod hashing;
pub mod mailer;
pub mod permission;
pub mod sanitize;
pub mod session;
pub mod ticket;
pub mod upload;
pub mod validation;
pub mod variable;

pub use attachment::{
    bind_existing_file, insert_attachment, load_attachments_by_ref, load_download_file,
    AttachmentSpec, AttachmentView, DownloadFile, DEFAULT_MIME,
};
pub use blob::{is_sha256_hex, sha256_hex, BlobError, BlobStore, BLOB_ROOT_ENV};
pub use canned::{
    list_offerable, load_offerable_response, load_ticket_vars, CannedListItem, CannedResponse,
    TicketVars,
};
pub use csrf::{
    new_csrf_token, verify_double_submit, CLIENT_CSRF_COOKIE, CSRF_HEADER, STAFF_CSRF_COOKIE,
};
pub use error::{ApiError, ErrorBody, ErrorEnvelope};
pub use hashing::{hash_password, verify_password, HashError};
pub use mailer::{MailError, Mailer, OutboundMail, StubMailer};
pub use permission::{
    GroupPermissions, PermissionDenied, PERM_CAN_CREATE_TICKETS, PERM_CAN_POST_REPLY,
};
pub use sanitize::{safe_html, sanitize};
pub use session::{Realm, Session, SessionData, SessionError, SessionStore, SESSION_TTL_SECS};
pub use ticket::{
    append_thread_entry, append_thread_entry_with_attachment, create_ticket,
    create_ticket_with_attachment, create_ticket_with_numbers, load_thread, post_staff_reply,
    random_ticket_number, NewThreadEntry, NewTicket, NewTicketInput, ThreadEntry, ThreadType,
    Ticket, TicketError, TICKET_NUMBER_MAX, TICKET_NUMBER_MIN,
};
pub use upload::{
    validate_upload, UploadError, UploadPolicy, ALLOW_ALL, ATTACHMENT_FIELD,
};
pub use validation::{
    is_email, validate_email_field, validate_password, validate_required, FieldError,
    PASSWORD_MIN_LEN,
};
pub use variable::{VarContext, VariableReplacer, URL_TOKEN};
