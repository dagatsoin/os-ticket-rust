//! Password hashing and verification — **argon2id only**.
//!
//! @implements BS-002: password hashing + verify, argon2id only. No
//!   phpass/MD5/bcrypt legacy fallback (ROADMAP Decision 3 — M1 is greenfield
//!   with no legacy accounts to migrate).
//!
//! `hash_password` produces a PHC-format string with the `$argon2id$` prefix;
//! `verify_password` checks a plaintext against such a string. There is no other
//! algorithm path: legacy hash formats are simply unverifiable here.

use argon2::Argon2;
use password_hash::{
    rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
};

/// Errors from the hashing layer.
#[derive(Debug, thiserror::Error)]
pub enum HashError {
    /// Hashing the plaintext failed (e.g. backend/parameter error).
    #[error("failed to hash password: {0}")]
    Hash(String),
    /// The stored hash string could not be parsed as a PHC hash.
    #[error("stored password hash is malformed: {0}")]
    MalformedHash(String),
}

/// The argon2id PHC prefix every emitted hash carries.
pub const ARGON2ID_PREFIX: &str = "$argon2id$";

/// Hash a plaintext password with argon2id, returning a PHC-format string.
///
/// A fresh random salt is generated per call, so identical plaintexts produce
/// distinct hashes. The default [`Argon2`] configuration is argon2id.
///
/// @implements BS-002: argon2id hashing (no legacy algorithm path).
pub fn hash_password(plaintext: &str) -> Result<String, HashError> {
    let salt = SaltString::generate(&mut OsRng);
    // Argon2::default() == argon2id, current recommended parameters.
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(plaintext.as_bytes(), &salt)
        .map_err(|e| HashError::Hash(e.to_string()))?;
    Ok(hash.to_string())
}

/// Verify a plaintext password against a stored argon2id PHC hash.
///
/// Returns `Ok(true)` on a match, `Ok(false)` on a mismatch, and `Err` only
/// when the stored hash cannot be parsed. Only the argon2id verifier is wired
/// up — a non-argon2 hash string is treated as malformed.
///
/// @implements BS-002: argon2id verify (correct plaintext only).
pub fn verify_password(plaintext: &str, stored_hash: &str) -> Result<bool, HashError> {
    let parsed =
        PasswordHash::new(stored_hash).map_err(|e| HashError::MalformedHash(e.to_string()))?;
    // Only argon2id is offered as a verifier; any other algorithm id won't match.
    match Argon2::default().verify_password(plaintext.as_bytes(), &parsed) {
        Ok(()) => Ok(true),
        Err(password_hash::Error::Password) => Ok(false),
        Err(e) => Err(HashError::MalformedHash(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_are_argon2id_prefixed() {
        let hash = hash_password("Agent123!").unwrap();
        assert!(
            hash.starts_with(ARGON2ID_PREFIX),
            "expected `$argon2id$` prefix, got: {hash}"
        );
    }

    #[test]
    fn verify_accepts_correct_and_rejects_wrong() {
        let hash = hash_password("Agent123!").unwrap();
        assert!(verify_password("Agent123!", &hash).unwrap());
        assert!(!verify_password("wrong", &hash).unwrap());
        assert!(!verify_password("agent123!", &hash).unwrap()); // case sensitive
    }

    #[test]
    fn same_plaintext_yields_distinct_hashes_random_salt() {
        let a = hash_password("Agent123!").unwrap();
        let b = hash_password("Agent123!").unwrap();
        assert_ne!(a, b, "random salt must make hashes differ");
        // ...yet both verify.
        assert!(verify_password("Agent123!", &a).unwrap());
        assert!(verify_password("Agent123!", &b).unwrap());
    }

    #[test]
    fn malformed_stored_hash_is_an_error_not_a_false_positive() {
        assert!(verify_password("x", "not-a-phc-hash").is_err());
    }

    #[test]
    fn legacy_md5_style_hash_does_not_verify() {
        // A bare MD5 hex digest is not a valid PHC string → malformed, never a
        // silent match. There is no legacy algorithm path.
        let md5_like = "5f4dcc3b5aa765d61d8327deb882cf99";
        assert!(verify_password("password", md5_like).is_err());
    }
}
