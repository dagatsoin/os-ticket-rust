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
pub mod email;
pub mod error;
pub mod hashing;
pub mod log;
pub mod mailer;
pub mod permission;
pub mod sanitize;
pub mod session;
pub mod ticket;
pub mod upload;
pub mod validation;
pub mod variable;
pub mod visibility;
pub mod workflow;

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
pub use email::{
    is_loop_suppressed_recipient, send_autoreply, send_notice, EmailTemplate, AUTOREPLY_HEADERS,
    NEW_TICKET_AUTORESPONSE, NOTICE_HEADERS, STAFF_REPLY_NOTIFICATION,
};
pub use error::{ApiError, ErrorBody, ErrorEnvelope};
pub use hashing::{hash_password, verify_password, HashError};
pub use log::{
    is_known_log_level, log, note_failed_login, parse_graceperiod_months, parse_log_level,
    purge_logs, LogType, LOG_LEVEL_LABELS,
};
pub use mailer::{MailError, Mailer, OutboundMail, SmtpConfig, SmtpMailer, StubMailer};
pub use permission::{
    GroupPermissions, PermissionDenied, PERM_CAN_ASSIGN_TICKETS, PERM_CAN_BAN_EMAILS,
    PERM_CAN_CLOSE_TICKETS, PERM_CAN_CREATE_TICKETS, PERM_CAN_DELETE_TICKETS,
    PERM_CAN_EDIT_TICKETS, PERM_CAN_MANAGE_FAQ, PERM_CAN_MANAGE_PREMADE, PERM_CAN_MANAGE_TICKETS,
    PERM_CAN_POST_REPLY, PERM_CAN_TRANSFER_TICKETS, PERM_CAN_VIEW_STAFF_STATS,
};
pub use sanitize::{safe_html, sanitize};
pub use session::{
    QueueSort, Realm, Session, SessionData, SessionError, SessionStore, SortPreferences,
    SESSION_TTL_SECS,
};
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
pub use visibility::{load_staff_visibility, StaffVisibility};
pub use workflow::{
    acquire_lock, apply_sla_on_create, assign_ticket, bulk_action, check_lock, claim_ticket,
    cleanup_expired_locks, clear_ticket_overdue, close_ticket, delete_ticket, get_lock_info,
    get_lock_time_minutes, mark_ticket_overdue, post_note, recompute_due_date_on_reopen,
    release_lock, reopen_ticket, select_sla_for_ticket, transfer_ticket, update_ticket,
    AssignResult, AssigneeType, BulkAction, BulkError, BulkResult, ClaimResult, CloseResult,
    DeleteResult, LockError, LockInfo, LockResult, NoteResult, NoteState, ReopenResult,
    TransferResult, UpdateError, UpdateResult, UpdateTicketInput, WorkflowError,
};
