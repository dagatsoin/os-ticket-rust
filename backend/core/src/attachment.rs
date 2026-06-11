//! Attachment persistence + retrieval over the SHA-256 blob store (TS-M2-A3 /
//! A5 / B1).
//!
//! This is the shared core both write channels (public create A3, staff reply
//! A5) and the download routes (B1) reuse, so the attachment shape and the
//! upsert-by-hash dedup live in exactly one place.
//!
//! Layering:
//! * an upload is validated by [`crate::upload::validate_upload`] BEFORE any blob
//!   is stored;
//! * the bytes are stored once in the [`crate::blob::BlobStore`] (content
//!   addressed — identical bytes share one physical blob, DEVIATION D1);
//! * [`insert_attachment`] upserts the `attachment_file` metadata row by its
//!   content hash and binds it to a ticket + thread entry via `ticket_attachment`
//!   (`ref_type` mirrors the thread entry type — `M`/`R`/`N`).
//!
//! @implements FS-022.4: bind a stored file to a ticket + thread entry.
//! @implements FS-022.12 (D1): one `attachment_file` row per distinct content
//!   hash — a second upload of identical bytes upserts onto the same row.
//! @implements ROADMAP M2 Decisions §7: the `{id, name, size, mime}` view shape
//!   shared by staff detail, client thread, and canned detail responses.

use serde::Serialize;
use sqlx::postgres::PgPool;
use sqlx::{Postgres, Row, Transaction};

use crate::blob::sha256_hex;

/// Default MIME when an upload carries no/blank content type.
pub const DEFAULT_MIME: &str = "application/octet-stream";

/// A validated, in-memory upload ready to be stored + bound (A3/A5/D4).
///
/// The caller has already run [`crate::upload::validate_upload`] against the
/// actual byte length before constructing this.
#[derive(Debug, Clone)]
pub struct AttachmentSpec {
    /// Original file name (as supplied by the client).
    pub name: String,
    /// Declared MIME type; blank ⇒ [`DEFAULT_MIME`].
    pub mime: String,
    /// The raw file bytes (their SHA-256 becomes the content key).
    pub bytes: Vec<u8>,
}

impl AttachmentSpec {
    /// Construct a spec, normalising a blank MIME to [`DEFAULT_MIME`].
    pub fn new(name: impl Into<String>, mime: impl Into<String>, bytes: Vec<u8>) -> Self {
        let mime = mime.into();
        let mime = if mime.trim().is_empty() {
            DEFAULT_MIME.to_string()
        } else {
            mime
        };
        Self {
            name: name.into(),
            mime,
            bytes,
        }
    }

    /// The content key (lowercase-hex SHA-256 of the bytes).
    #[must_use]
    pub fn hash(&self) -> String {
        sha256_hex(&self.bytes)
    }

    /// The on-disk relative storage key `aa/bb/<sha256>` (mirrors the blob layout).
    #[must_use]
    pub fn storage_key(&self) -> String {
        let h = self.hash();
        format!("{}/{}/{}", &h[0..2], &h[2..4], h)
    }
}

/// One attachment as exposed in a thread-entry payload (ROADMAP §7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentView {
    pub id: i64,
    pub name: String,
    pub size: i64,
    pub mime: String,
}

/// The metadata needed to stream a stored blob back on download (B1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadFile {
    /// The parent ticket id (the auth gate compares this to the session's ticket).
    pub ticket_id: i64,
    /// `M`/`R`/`N` — the client route rejects `N` (a client never sees notes).
    pub ref_type: String,
    /// Original file name (for the `Content-Disposition`).
    pub name: String,
    /// Stored MIME (for the `Content-Type`).
    pub mime: String,
    /// Byte size (for the `Content-Length`).
    pub size: i64,
    /// Content hash (the blob-store key for [`crate::blob::BlobStore::open`]).
    pub hash: String,
}

/// Upsert the `attachment_file` metadata (by content hash) and bind it to a
/// ticket + thread entry, inside an existing transaction.
///
/// Dedup (D1): the upsert is `ON CONFLICT (hash) DO UPDATE` so two uploads of
/// identical bytes converge on ONE `attachment_file` row; the `RETURNING id`
/// yields that shared row's id either way. A fresh `ticket_attachment` row is
/// inserted per binding (so two tickets sharing one file get two bindings, one
/// file).
///
/// The bytes must already be in the blob store (the caller does the `put`); this
/// only writes the relational rows. `ref_type` is the thread entry type the file
/// hangs off (`M` on create, `R` on a reply).
///
/// @implements FS-022.4 / FS-022.12 (D1): bind + upsert-by-hash dedup.
pub async fn insert_attachment(
    tx: &mut Transaction<'_, Postgres>,
    ticket_id: i64,
    ref_id: i64,
    ref_type: &str,
    spec: &AttachmentSpec,
) -> Result<AttachmentView, sqlx::Error> {
    let hash = spec.hash();
    let storage_key = spec.storage_key();
    let size = spec.bytes.len() as i64;

    // Upsert by content hash → one row per distinct content (D1). The no-op
    // UPDATE (set name to itself) lets RETURNING fire on a conflict too.
    let file_id: i64 = sqlx::query_scalar(
        r#"INSERT INTO attachment_file (mime, size, hash, name, storage_key)
           VALUES ($1, $2, $3, $4, $5)
           ON CONFLICT (hash) DO UPDATE SET name = attachment_file.name
           RETURNING id"#,
    )
    .bind(&spec.mime)
    .bind(size)
    .bind(&hash)
    .bind(&spec.name)
    .bind(&storage_key)
    .fetch_one(&mut **tx)
    .await?;

    sqlx::query(
        "INSERT INTO ticket_attachment (ticket_id, file_id, ref_id, ref_type)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(ticket_id)
    .bind(file_id)
    .bind(ref_id)
    .bind(ref_type)
    .execute(&mut **tx)
    .await?;

    Ok(AttachmentView {
        id: file_id,
        name: spec.name.clone(),
        size,
        mime: spec.mime.clone(),
    })
}

/// Load every attachment bound to a ticket, grouped by the thread entry
/// (`ref_id`) it hangs off. The map value is the §7 view list for that entry.
///
/// Returned ids are the **`ticket_attachment` binding ids** (not the shared
/// `attachment_file` id), so a download route can resolve a specific binding
/// back to its parent ticket for the auth gate (B1).
///
/// @implements ROADMAP §7: per-entry `{id, name, size, mime}` lists.
pub async fn load_attachments_by_ref(
    pool: &PgPool,
    ticket_id: i64,
) -> Result<std::collections::HashMap<i64, Vec<AttachmentView>>, sqlx::Error> {
    let rows = sqlx::query(
        r#"SELECT ta.id AS binding_id, ta.ref_id,
                  af.name, af.size, af.mime
           FROM ticket_attachment ta
           JOIN attachment_file af ON af.id = ta.file_id
           WHERE ta.ticket_id = $1
           ORDER BY ta.id ASC"#,
    )
    .bind(ticket_id)
    .fetch_all(pool)
    .await?;

    let mut map: std::collections::HashMap<i64, Vec<AttachmentView>> =
        std::collections::HashMap::new();
    for row in rows {
        let ref_id: i64 = row.get("ref_id");
        let view = AttachmentView {
            id: row.get("binding_id"),
            name: row.get("name"),
            size: row.get("size"),
            mime: row.get("mime"),
        };
        map.entry(ref_id).or_default().push(view);
    }
    Ok(map)
}

/// Resolve a `ticket_attachment` binding id to the metadata needed to stream it
/// back on download (B1). `None` when no such binding exists (the route turns
/// that into a 404 with no existence leak).
///
/// @implements FS-022.10 / FS-022.11: download resolution (binding → file).
pub async fn load_download_file(
    pool: &PgPool,
    binding_id: i64,
) -> Result<Option<DownloadFile>, sqlx::Error> {
    let row = sqlx::query(
        r#"SELECT ta.ticket_id, ta.ref_type,
                  af.name, af.mime, af.size, af.hash
           FROM ticket_attachment ta
           JOIN attachment_file af ON af.id = ta.file_id
           WHERE ta.id = $1"#,
    )
    .bind(binding_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| DownloadFile {
        ticket_id: r.get("ticket_id"),
        ref_type: r.get("ref_type"),
        name: r.get("name"),
        mime: r.get("mime"),
        size: r.get("size"),
        hash: r.get("hash"),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_mime_defaults_to_octet_stream() {
        let spec = AttachmentSpec::new("x.bin", "   ", vec![1, 2, 3]);
        assert_eq!(spec.mime, DEFAULT_MIME);
        let spec2 = AttachmentSpec::new("x.pdf", "application/pdf", vec![1]);
        assert_eq!(spec2.mime, "application/pdf");
    }

    #[test]
    fn storage_key_mirrors_blob_layout() {
        let spec = AttachmentSpec::new("abc.txt", "text/plain", b"abc".to_vec());
        // SHA-256("abc") is a known vector.
        let h = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert_eq!(spec.hash(), h);
        assert_eq!(spec.storage_key(), format!("ba/78/{h}"));
    }

    #[test]
    fn view_serialises_camel_case_shape() {
        let v = AttachmentView {
            id: 7,
            name: "invoice.pdf".into(),
            size: 1234,
            mime: "application/pdf".into(),
        };
        let json = serde_json::to_value(&v).unwrap();
        assert_eq!(json["id"], 7);
        assert_eq!(json["name"], "invoice.pdf");
        assert_eq!(json["size"], 1234);
        assert_eq!(json["mime"], "application/pdf");
    }
}
