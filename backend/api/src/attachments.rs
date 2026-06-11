//! Shared attachment plumbing for the multipart write paths (TS-M2-A3 / A5).
//!
//! Both the public create route (`tickets.rs`) and the staff reply route
//! (`staff.rs`) dual-accept by `Content-Type` (ROADMAP §2): an
//! `application/json` body keeps the M1 contract, a `multipart/form-data` body
//! carries the same fields as individual parts plus an optional `attachment`
//! file part. This module centralises:
//!
//! * reading the seeded attachment config keys into an [`UploadPolicy`];
//! * draining a multipart request into named text fields + the optional
//!   `attachment` part (name/mime/bytes);
//! * validating that part against the policy (422 keyed on `attachment`).
//!
//! @implements ROADMAP M2 Decisions §2: dual-accept multipart, `attachment` key.
//! @implements FS-022.13: the upload gate reads the seeded config policy.

use axum::extract::Multipart;
use sqlx::postgres::PgPool;

use ost_core::upload::{validate_upload, UploadPolicy};
use ost_core::{ApiError, AttachmentSpec};

/// Config key: attachments master switch (`"true"`/`"false"`).
pub const CFG_ALLOW_ATTACHMENTS: &str = "allow_attachments";
/// Config key: comma-separated extension allow-list (`.*` = all).
pub const CFG_ALLOWED_FILETYPES: &str = "allowed_filetypes";
/// Config key: max upload size in bytes.
pub const CFG_MAX_FILE_SIZE: &str = "max_file_size";

/// The multipart field name carrying the file part (§2).
pub const ATTACHMENT_PART: &str = "attachment";

/// Load the attachment [`UploadPolicy`] from the seeded `config` rows.
///
/// Missing keys fall back to safe defaults (attachments off, empty allow-list,
/// 0 size) so an unconfigured DB default-denies rather than accepting anything.
///
/// @implements FS-022.13 / BS-022.14: policy from config, default-deny.
pub async fn load_upload_policy(pool: &PgPool) -> Result<UploadPolicy, ApiError> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT key, value FROM config
         WHERE key IN ($1, $2, $3)",
    )
    .bind(CFG_ALLOW_ATTACHMENTS)
    .bind(CFG_ALLOWED_FILETYPES)
    .bind(CFG_MAX_FILE_SIZE)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal("Attachment policy lookup failed"))?;

    let mut allow = false;
    let mut filetypes = String::new();
    let mut max_size: u64 = 0;
    for (key, value) in rows {
        match key.as_str() {
            CFG_ALLOW_ATTACHMENTS => allow = value.trim().eq_ignore_ascii_case("true"),
            CFG_ALLOWED_FILETYPES => filetypes = value,
            CFG_MAX_FILE_SIZE => max_size = value.trim().parse().unwrap_or(0),
            _ => {}
        }
    }
    Ok(UploadPolicy::new(allow, filetypes, max_size))
}

/// The text fields + optional file part drained from a multipart request.
#[derive(Debug, Default)]
pub struct MultipartForm {
    /// Named text parts (field name → value).
    pub fields: std::collections::HashMap<String, String>,
    /// The optional `attachment` file part, if one was present and non-empty.
    pub attachment: Option<AttachmentSpec>,
}

impl MultipartForm {
    /// A text field by name, or `""` when absent (mirrors the JSON `#[serde(default)]`).
    #[must_use]
    pub fn field(&self, name: &str) -> &str {
        self.fields.get(name).map(String::as_str).unwrap_or("")
    }
}

/// Drain a [`Multipart`] body into [`MultipartForm`]: every non-file part becomes
/// a text field, and the `attachment` part (if present, with a filename and
/// non-empty bytes) becomes the [`AttachmentSpec`].
///
/// An empty `attachment` part (no file chosen) is treated as no attachment.
///
/// @implements ROADMAP §2: multipart drain (text parts + `attachment` file part).
pub async fn drain_multipart(mut multipart: Multipart) -> Result<MultipartForm, ApiError> {
    let mut form = MultipartForm::default();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| ApiError::validation("Malformed multipart request"))?
    {
        let name = field.name().unwrap_or("").to_string();
        let filename = field.file_name().map(str::to_string);
        let content_type = field.content_type().map(str::to_string);

        if name == ATTACHMENT_PART {
            // The file part: read its bytes. A part with a filename but zero
            // bytes (or no filename at all) means "no file chosen".
            let file_name = filename.unwrap_or_default();
            let mime = content_type.unwrap_or_default();
            let bytes = field
                .bytes()
                .await
                .map_err(|_| ApiError::validation("Could not read the uploaded file"))?
                .to_vec();
            if file_name.is_empty() || bytes.is_empty() {
                continue;
            }
            form.attachment = Some(AttachmentSpec::new(file_name, mime, bytes));
        } else {
            let value = field
                .text()
                .await
                .map_err(|_| ApiError::validation("Malformed multipart field"))?;
            form.fields.insert(name, value);
        }
    }

    Ok(form)
}

/// Validate the optional attachment against the policy, returning a 422 keyed on
/// `attachment` on failure. A `None` attachment passes trivially.
///
/// @implements EC-011.5 / FS-021.16: a disallowed/oversized upload ⇒ 422.
pub fn validate_attachment(
    attachment: &Option<AttachmentSpec>,
    policy: &UploadPolicy,
) -> Result<(), ApiError> {
    if let Some(spec) = attachment {
        validate_upload(&spec.name, spec.bytes.len() as u64, &spec.mime, policy)
            .map_err(|e| e.into_api_error())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_defaults_to_empty() {
        let form = MultipartForm::default();
        assert_eq!(form.field("name"), "");
    }

    #[test]
    fn validate_none_attachment_passes() {
        let policy = UploadPolicy::new(true, ".pdf", 1000);
        assert!(validate_attachment(&None, &policy).is_ok());
    }

    #[test]
    fn validate_disallowed_attachment_keys_on_attachment_field() {
        let policy = UploadPolicy::new(true, ".pdf", 1_000_000);
        let spec = AttachmentSpec::new("evil.exe", "application/octet-stream", vec![1, 2, 3]);
        let err = validate_attachment(&Some(spec), &policy).unwrap_err();
        assert_eq!(err.status, http::StatusCode::UNPROCESSABLE_ENTITY);
        assert!(err.fields.contains_key("attachment"));
    }

    #[test]
    fn validate_oversize_attachment_rejected() {
        let policy = UploadPolicy::new(true, ".pdf", 2);
        let spec = AttachmentSpec::new("ok.pdf", "application/pdf", vec![1, 2, 3]);
        let err = validate_attachment(&Some(spec), &policy).unwrap_err();
        assert!(err.fields.contains_key("attachment"));
    }
}
