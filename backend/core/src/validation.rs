//! Input validation helpers (pure functions, no I/O).
//!
//! @implements FS-003.7: Declarative field-set validation (the required-field
//!   gate, modelled here as standalone helpers).
//! @implements FS-003.8: Per-type validation rules (email/password floors).
//! @implements FS-003.9: Standalone format validators (`is_email`).
//! @implements FS-011.8: Web-ticket field validation (required + length bounds).
//! @implements BS-009: Email format rule (local@domain, TLD >= 2 alphanumerics).
//! @implements BS-012: Password minimum length (>= 5 characters).
//!
//! These deliberately mirror the legacy `Validator`/`Format` behaviour only for
//! the parts the M1 slice exercises. They are deterministic and unit-tested.

/// Minimum password length at the input boundary (BS-012 / FS-003.8).
///
/// This is the field-validation floor used at input boundaries; staff account
/// password policy/hashing is owned separately (see [`crate::hashing`]).
pub const PASSWORD_MIN_LEN: usize = 5;

/// Validate an email address per BS-009.
///
/// A valid email is `local-part@domain` where:
/// * the local part is non-empty,
/// * the domain is one-or-more dot-separated labels of alphanumeric/hyphen
///   characters (a label may not start/end with a hyphen, nor be empty),
/// * the final label (the TLD) is at least two **alphanumeric** characters.
///
/// Shape/length only — never whether the address is deliverable.
///
/// @implements FS-003.9 / BS-009: standalone email format validator.
#[must_use]
pub fn is_email(email: &str) -> bool {
    // Exactly one '@', splitting local-part from domain.
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    if local.is_empty() || domain.contains('@') {
        return false;
    }
    // Reject any whitespace anywhere.
    if email.chars().any(char::is_whitespace) {
        return false;
    }

    let labels: Vec<&str> = domain.split('.').collect();
    // Need at least two labels (a domain + a TLD).
    if labels.len() < 2 {
        return false;
    }
    for label in &labels {
        if !is_domain_label(label) {
            return false;
        }
    }
    // TLD: last label, >= 2 chars, all alphanumeric (no hyphen).
    let tld = labels[labels.len() - 1];
    tld.len() >= 2 && tld.chars().all(|c| c.is_ascii_alphanumeric())
}

/// A domain label: non-empty, only alphanumeric or hyphen, not hyphen-bounded.
fn is_domain_label(label: &str) -> bool {
    if label.is_empty() {
        return false;
    }
    if label.starts_with('-') || label.ends_with('-') {
        return false;
    }
    label
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Validate a required free-text field with an optional maximum length.
///
/// Trims leading/trailing whitespace first (FS-011.8: `email`/`phone`/`subject`/
/// `name` are trimmed before validation), then:
/// * rejects empty/whitespace-only values (required-field gate),
/// * rejects values whose trimmed length exceeds `max_len`.
///
/// Returns the trimmed value on success.
///
/// @implements FS-011.8 / FS-003.7: required-field + length validation.
pub fn validate_required(value: &str, max_len: usize) -> Result<&str, FieldError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(FieldError::Missing);
    }
    if trimmed.chars().count() > max_len {
        return Err(FieldError::TooLong { max: max_len });
    }
    Ok(trimmed)
}

/// Validate a password against the BS-012 minimum-length floor.
///
/// @implements FS-003.8 / BS-012: password minimum length (>= 5).
pub fn validate_password(password: &str) -> Result<(), FieldError> {
    // Password length is measured by characters; not trimmed (a password may
    // legitimately contain leading/trailing spaces).
    if password.chars().count() < PASSWORD_MIN_LEN {
        return Err(FieldError::TooShort {
            min: PASSWORD_MIN_LEN,
        });
    }
    Ok(())
}

/// Why a single field failed validation.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FieldError {
    /// A required field was missing or empty after trimming.
    #[error("is required")]
    Missing,
    /// The field exceeded its maximum length.
    #[error("must be at most {max} characters")]
    TooLong { max: usize },
    /// The field was shorter than its minimum length.
    #[error("must be at least {min} characters")]
    TooShort { min: usize },
    /// The field was not a valid email address.
    #[error("must be a valid email address")]
    InvalidEmail,
}

/// Validate an email field: required, within `max_len`, and well-formed.
///
/// @implements FS-003.8 / FS-011.8: email field validation.
pub fn validate_email_field(value: &str, max_len: usize) -> Result<&str, FieldError> {
    let trimmed = validate_required(value, max_len)?;
    if is_email(trimmed) {
        Ok(trimmed)
    } else {
        Err(FieldError::InvalidEmail)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- is_email (FS-003.9 / BS-009) -------------------------------------

    #[test]
    fn accepts_valid_emails() {
        for ok in [
            "user@example.com",
            "first.last@example.co.uk",
            "agent+tag@sub.domain.io",
            "a@b.cd",
            "user_name@host-name.com",
            "weird.but.valid#$%@example.com",
        ] {
            assert!(is_email(ok), "expected `{ok}` to be valid");
        }
    }

    #[test]
    fn rejects_emails_without_at() {
        assert!(!is_email("plainaddress"));
    }

    #[test]
    fn rejects_emails_without_domain() {
        assert!(!is_email("user@"));
        assert!(!is_email("@example.com"));
    }

    #[test]
    fn rejects_emails_with_spaces() {
        assert!(!is_email("user name@example.com"));
        assert!(!is_email("user@exam ple.com"));
        assert!(!is_email(" user@example.com"));
    }

    #[test]
    fn rejects_single_label_domain_and_short_tld() {
        assert!(!is_email("user@localhost")); // no dot / no TLD
        assert!(!is_email("user@example.c")); // 1-char TLD < 2
    }

    #[test]
    fn rejects_double_at_and_hyphen_bounded_labels() {
        assert!(!is_email("user@@example.com"));
        assert!(!is_email("user@-example.com"));
        assert!(!is_email("user@example-.com"));
        assert!(!is_email("user@exa..mple.com")); // empty label
    }

    // --- validate_required (FS-011.8) -------------------------------------

    #[test]
    fn required_rejects_empty_and_whitespace_only() {
        assert_eq!(validate_required("", 100), Err(FieldError::Missing));
        assert_eq!(validate_required("   ", 100), Err(FieldError::Missing));
        assert_eq!(validate_required("\t\n", 100), Err(FieldError::Missing));
    }

    #[test]
    fn required_trims_and_accepts_nonempty() {
        assert_eq!(validate_required("  hello  ", 100), Ok("hello"));
    }

    #[test]
    fn required_enforces_max_length_on_trimmed_value() {
        // 5 visible chars within bound after trimming surrounding whitespace.
        assert_eq!(validate_required("  abcde  ", 5), Ok("abcde"));
        // 6 chars over a bound of 5.
        assert_eq!(
            validate_required("abcdef", 5),
            Err(FieldError::TooLong { max: 5 })
        );
    }

    #[test]
    fn required_counts_unicode_scalar_values_not_bytes() {
        // "é" is multi-byte but a single character; 3 chars within a bound of 3.
        assert_eq!(validate_required("éàç", 3), Ok("éàç"));
        assert_eq!(
            validate_required("éàçü", 3),
            Err(FieldError::TooLong { max: 3 })
        );
    }

    // --- validate_password (FS-003.8 / BS-012) ----------------------------

    #[test]
    fn password_floor_is_five_chars() {
        assert_eq!(
            validate_password("1234"),
            Err(FieldError::TooShort { min: 5 })
        );
        assert!(validate_password("12345").is_ok());
        assert!(validate_password("Agent123!").is_ok());
    }

    // --- validate_email_field ---------------------------------------------

    #[test]
    fn email_field_requires_present_valid_and_bounded() {
        assert_eq!(
            validate_email_field("  ", 100),
            Err(FieldError::Missing)
        );
        assert_eq!(
            validate_email_field("not-an-email", 100),
            Err(FieldError::InvalidEmail)
        );
        assert_eq!(
            validate_email_field("  a@b.cd  ", 100),
            Ok("a@b.cd")
        );
    }
}
