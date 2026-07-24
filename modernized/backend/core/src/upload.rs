//! Shared upload validation gate (TS-M2-A2).
//!
//! Every upload path (A3 create, A5 reply, D4 canned own-file) runs
//! [`validate_upload`] before a blob is stored. It mirrors the legacy
//! extension-based allow-list + max-size behaviour and its default-deny quirk.
//!
//! @implements FS-022.13: Upload validation (type + size) — extension allow-list
//!   check + max-size check; a failure yields a field error.
//! @implements BS-022.13: allowed file types are matched by extension only,
//!   against the configured comma-separated allow-list; `.*` permits all types.
//! @implements BS-022.14: an empty/unset allow-list rejects everything
//!   (default-deny); only the explicit wildcard `.*` allows all types.
//! @implements EC-022.16: empty/whitespace allow-list ⇒ every file rejected.
//!
//! Modernisation note: A2 keeps the legacy extension allow-list + default-deny
//! semantics, but deliberately drops the legacy 3–4-char extension-regex quirk
//! (KL-022.3 / EC-022.17): the real final extension is compared, so an extension
//! of any length matches the allow-list normally. MIME is accepted for future
//! use but, like the legacy code, is NOT enforced (extension-only, KL-022.3).

use crate::error::ApiError;

/// The 422 field key for attachment errors (ROADMAP M2 Decisions §2).
pub const ATTACHMENT_FIELD: &str = "attachment";

/// The wildcard allow-list token that permits all file types.
pub const ALLOW_ALL: &str = ".*";

/// Attachment policy, sourced from the seeded config keys (TS-M2-A2 config seed).
///
/// Built from the `allow_attachments`, `allowed_filetypes`, and `max_file_size`
/// config values; passed into [`validate_upload`] so the helper stays pure.
#[derive(Debug, Clone)]
pub struct UploadPolicy {
    /// Master switch — when false, no upload is permitted regardless of type/size.
    pub allow_attachments: bool,
    /// Comma-separated allow-list of extensions (each like `.pdf`), or `.*`.
    /// Stored as the raw config string; parsed per call.
    pub allowed_filetypes: String,
    /// Maximum accepted upload size in bytes (the actual byte count is checked).
    pub max_file_size: u64,
}

impl UploadPolicy {
    /// Construct a policy from raw config values.
    #[must_use]
    pub fn new(allow_attachments: bool, allowed_filetypes: impl Into<String>, max_file_size: u64) -> Self {
        Self {
            allow_attachments,
            allowed_filetypes: allowed_filetypes.into(),
            max_file_size,
        }
    }
}

/// Why an upload was rejected (one variant per gate, in check order).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UploadError {
    /// The master switch `allow_attachments` is off.
    #[error("Attachments are not permitted")]
    NotPermitted,
    /// The file's extension is not in the allow-list (or the list is empty).
    #[error("Invalid file type")]
    InvalidType,
    /// The file's byte size exceeds `max_file_size`.
    #[error("File is too big. Maximum of {max} bytes allowed")]
    TooBig {
        /// The configured maximum, in bytes.
        max: u64,
    },
}

impl UploadError {
    /// Render this rejection as the shared 422 envelope keyed on `attachment`.
    #[must_use]
    pub fn into_api_error(self) -> ApiError {
        ApiError::validation("Validation failed").with_field(ATTACHMENT_FIELD, self.to_string())
    }
}

/// Validate one upload against `policy`.
///
/// Checks, in order: the master switch, the extension allow-list (default-deny;
/// `.*` allows all), then the actual byte size against the cap. Returns `Ok(())`
/// when every gate passes.
///
/// * `name` — the original file name (the extension is taken from it).
/// * `size` — the **actual** byte count of the upload (never a client-claimed
///   size; the caller measures the received bytes).
/// * `_mime` — accepted for forward-compat but NOT enforced (extension-only,
///   per KL-022.3 / BS-022.13).
///
/// @implements FS-022.13 / BS-022.13 / BS-022.14: type + size upload gate.
pub fn validate_upload(name: &str, size: u64, _mime: &str, policy: &UploadPolicy) -> Result<(), UploadError> {
    // Gate 1: master switch (AC-4).
    if !policy.allow_attachments {
        return Err(UploadError::NotPermitted);
    }

    // Gate 2: extension allow-list (BS-022.13 / BS-022.14, default-deny).
    if !extension_allowed(name, &policy.allowed_filetypes) {
        return Err(UploadError::InvalidType);
    }

    // Gate 3: max size on the actual byte count (FS-022.13). A zero cap means
    // "no size limit" (legacy: a non-zero max-file-size is required to enforce).
    if policy.max_file_size != 0 && size > policy.max_file_size {
        return Err(UploadError::TooBig {
            max: policy.max_file_size,
        });
    }

    Ok(())
}

/// Whether `name`'s extension is permitted by the comma-separated `allow_list`.
///
/// Default-deny: an empty/whitespace-only list permits nothing (BS-022.14).
/// `.*` (the only wildcard) permits everything. Otherwise the file's final
/// extension (lowercased, with its leading dot) must equal one of the listed
/// tokens (each normalised to a leading-dot, lowercase form).
fn extension_allowed(name: &str, allow_list: &str) -> bool {
    // Parse the allow-list into normalised `.ext` tokens (skip blanks).
    let mut tokens = allow_list
        .split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .peekable();

    // Default-deny: no tokens ⇒ reject (NOT "allow everything").
    if tokens.peek().is_none() {
        return false;
    }

    let file_ext = file_extension(name); // e.g. Some(".pdf"), lowercase, or None.

    for raw in tokens {
        // The explicit wildcard allows all types.
        if raw == ALLOW_ALL {
            return true;
        }
        if let Some(ref ext) = file_ext {
            if normalise_token(raw) == *ext {
                return true;
            }
        }
    }
    false
}

/// Normalise an allow-list token to `.ext` lowercase form (tolerate `pdf` or
/// `.PDF`). The wildcard is handled by the caller before this is reached.
fn normalise_token(raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    if lower.starts_with('.') {
        lower
    } else {
        format!(".{lower}")
    }
}

/// The file's final extension including the leading dot, lowercased
/// (e.g. `archive.tar.gz` → `.gz`). `None` when the name has no dot, ends in a
/// dot, or is a dotfile with no extension.
fn file_extension(name: &str) -> Option<String> {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let idx = base.rfind('.')?;
    // A leading-dot dotfile (".env") or a trailing dot ("name.") has no ext.
    if idx == 0 || idx + 1 == base.len() {
        return None;
    }
    Some(base[idx..].to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(allow: bool, list: &str, max: u64) -> UploadPolicy {
        UploadPolicy::new(allow, list, max)
    }

    const DEFAULT_LIST: &str = ".pdf,.png,.jpg,.txt,.doc";
    const ONE_MB: u64 = 1_048_576;

    // --- AC-1: permitted extension passes; disallowed is rejected. --------
    #[test]
    fn permitted_extension_passes_disallowed_rejected() {
        let p = policy(true, DEFAULT_LIST, ONE_MB);
        assert!(validate_upload("invoice.pdf", 100, "application/pdf", &p).is_ok());
        assert_eq!(
            validate_upload("evil.exe", 100, "application/octet-stream", &p),
            Err(UploadError::InvalidType)
        );
    }

    #[test]
    fn extension_match_is_case_insensitive() {
        let p = policy(true, DEFAULT_LIST, ONE_MB);
        assert!(validate_upload("PHOTO.JPG", 100, "image/jpeg", &p).is_ok());
        assert!(validate_upload("Doc.PdF", 100, "application/pdf", &p).is_ok());
    }

    #[test]
    fn uses_final_extension_only() {
        let p = policy(true, DEFAULT_LIST, ONE_MB);
        // archive.pdf.exe → .exe → rejected (no allow-list smuggling).
        assert_eq!(
            validate_upload("archive.pdf.exe", 100, "x", &p),
            Err(UploadError::InvalidType)
        );
        // report.final.pdf → .pdf → allowed.
        assert!(validate_upload("report.final.pdf", 100, "application/pdf", &p).is_ok());
    }

    // --- AC-2: empty list rejects all; `.*` allows all. -------------------
    #[test]
    fn empty_allow_list_rejects_everything() {
        for list in ["", "   ", ",", " , "] {
            let p = policy(true, list, ONE_MB);
            assert_eq!(
                validate_upload("invoice.pdf", 100, "application/pdf", &p),
                Err(UploadError::InvalidType),
                "empty list `{list:?}` must default-deny"
            );
        }
    }

    #[test]
    fn wildcard_allows_all_types() {
        let p = policy(true, ".*", ONE_MB);
        assert!(validate_upload("evil.exe", 100, "x", &p).is_ok());
        assert!(validate_upload("anything.xyz", 100, "x", &p).is_ok());
        assert!(validate_upload("noextension", 100, "x", &p).is_ok());
    }

    // --- AC-3: size check on actual byte count. ---------------------------
    #[test]
    fn rejects_over_cap_accepts_at_cap() {
        let p = policy(true, DEFAULT_LIST, ONE_MB);
        // 1 MB + 1 byte → too big.
        assert_eq!(
            validate_upload("invoice.pdf", ONE_MB + 1, "application/pdf", &p),
            Err(UploadError::TooBig { max: ONE_MB })
        );
        // Exactly at cap → ok.
        assert!(validate_upload("invoice.pdf", ONE_MB, "application/pdf", &p).is_ok());
    }

    #[test]
    fn zero_cap_means_no_size_limit() {
        let p = policy(true, ".*", 0);
        assert!(validate_upload("big.bin", 10_000_000, "x", &p).is_ok());
    }

    // --- AC-4: master switch off → everything refused. --------------------
    #[test]
    fn disabled_master_switch_refuses_all() {
        let p = policy(false, ".*", ONE_MB);
        assert_eq!(
            validate_upload("invoice.pdf", 1, "application/pdf", &p),
            Err(UploadError::NotPermitted)
        );
        // Even a perfectly valid file is refused when attachments are off.
        assert_eq!(
            validate_upload("ok.pdf", 100, "application/pdf", &p),
            Err(UploadError::NotPermitted)
        );
    }

    // --- check order: master switch precedes type/size. -------------------
    #[test]
    fn master_switch_takes_precedence_over_type_and_size() {
        let p = policy(false, "", 0);
        // A disallowed type + oversize would otherwise error on type/size, but
        // the master switch short-circuits first.
        assert_eq!(
            validate_upload("evil.exe", u64::MAX, "x", &p),
            Err(UploadError::NotPermitted)
        );
    }

    // --- envelope mapping -------------------------------------------------
    #[test]
    fn maps_to_attachment_field_error() {
        let err = UploadError::InvalidType.into_api_error();
        let v = serde_json::to_value(err.to_envelope()).unwrap();
        assert_eq!(v["error"]["fields"]["attachment"], "Invalid file type");
        assert!(v["error"]["fields"]["attachment"].is_string());
    }

    #[test]
    fn too_big_message_includes_max() {
        let err = UploadError::TooBig { max: ONE_MB }.into_api_error();
        let v = serde_json::to_value(err.to_envelope()).unwrap();
        assert!(v["error"]["fields"]["attachment"]
            .as_str()
            .unwrap()
            .contains("1048576"));
    }

    // --- file_extension edge cases ---------------------------------------
    #[test]
    fn dotfiles_and_no_extension_have_no_ext() {
        // dotfile and no-dot name → None → rejected by a literal list.
        let p = policy(true, DEFAULT_LIST, ONE_MB);
        assert_eq!(
            validate_upload(".env", 10, "x", &p),
            Err(UploadError::InvalidType)
        );
        assert_eq!(
            validate_upload("README", 10, "x", &p),
            Err(UploadError::InvalidType)
        );
        assert_eq!(
            validate_upload("trailing.", 10, "x", &p),
            Err(UploadError::InvalidType)
        );
    }

    #[test]
    fn allow_list_tolerates_bare_and_uppercase_tokens() {
        // "pdf, .PNG" should still match invoice.pdf and photo.png.
        let p = policy(true, "pdf, .PNG", ONE_MB);
        assert!(validate_upload("invoice.pdf", 10, "x", &p).is_ok());
        assert!(validate_upload("photo.png", 10, "x", &p).is_ok());
        assert_eq!(
            validate_upload("note.txt", 10, "x", &p),
            Err(UploadError::InvalidType)
        );
    }
}
