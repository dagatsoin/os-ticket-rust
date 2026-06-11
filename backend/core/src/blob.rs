//! SHA-256 content-addressed filesystem blob store (TS-M2-A1).
//!
//! @implements FS-022.12: Content-addressed file storage (DEVIATION D1) — bytes
//!   are stored on disk keyed by their content SHA-256 at
//!   `<BLOB_ROOT>/aa/bb/<sha256>` (the first two hex byte-pairs become fan-out
//!   directories). Putting identical bytes twice is idempotent and shares one
//!   on-disk file, giving real byte-level de-duplication. This obsoletes the
//!   legacy time-salted KL-022.4 quirk and replaces the chunked-DB store.
//!
//! Blob root (ROADMAP M2 Decisions §1): env `BLOB_ROOT`, default
//! `<workspace root>/var/blobs`, resolved to an absolute path at startup; all
//! binaries and tests honor it via [`BlobStore::from_env`].

use std::io;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

/// Environment variable naming the blob-store root directory (§1).
pub const BLOB_ROOT_ENV: &str = "BLOB_ROOT";

/// Default blob root relative to the workspace root when `BLOB_ROOT` is unset.
pub const DEFAULT_BLOB_SUBDIR: &str = "var/blobs";

/// A content-addressed filesystem blob store.
///
/// The store owns an absolute `root` directory. Each blob lives at
/// `root/aa/bb/<sha256>` where `aa`/`bb` are the first two hex byte-pairs of the
/// lowercase-hex SHA-256 digest of the content.
#[derive(Debug, Clone)]
pub struct BlobStore {
    root: PathBuf,
}

/// Errors from blob-store operations.
#[derive(Debug, thiserror::Error)]
pub enum BlobError {
    /// The supplied hash was not a 64-char lowercase hex SHA-256 digest.
    #[error("invalid blob hash: {0}")]
    InvalidHash(String),
    /// An underlying filesystem error.
    #[error("blob I/O error: {0}")]
    Io(#[from] io::Error),
}

impl BlobStore {
    /// Construct a store rooted at `root`. The directory is created lazily on
    /// the first `put`; the path is stored as-is (callers pass an absolute path,
    /// e.g. via [`BlobStore::from_env`]).
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Resolve the blob root from the environment (§1).
    ///
    /// Honors `BLOB_ROOT` when set; otherwise defaults to
    /// `<workspace_root>/var/blobs`. The result is canonicalised to an absolute
    /// path (best-effort: the parent is resolved even if the leaf does not yet
    /// exist), so every binary and test agrees on one location.
    pub fn from_env(workspace_root: impl AsRef<Path>) -> Self {
        let root = match std::env::var(BLOB_ROOT_ENV) {
            Ok(v) if !v.trim().is_empty() => PathBuf::from(v),
            _ => workspace_root.as_ref().join(DEFAULT_BLOB_SUBDIR),
        };
        Self::new(absolutize(&root))
    }

    /// The absolute root directory of this store.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The on-disk path a blob with `hash` would occupy: `root/aa/bb/<hash>`.
    ///
    /// Errors if `hash` is not a 64-char lowercase hex SHA-256 digest.
    pub fn path_for(&self, hash: &str) -> Result<PathBuf, BlobError> {
        if !is_sha256_hex(hash) {
            return Err(BlobError::InvalidHash(hash.to_string()));
        }
        Ok(self.root.join(&hash[0..2]).join(&hash[2..4]).join(hash))
    }

    /// Store `bytes`, returning the lowercase-hex SHA-256 of the content.
    ///
    /// Idempotent (D1): if a blob with the same hash already exists on disk the
    /// existing file is reused and no rewrite occurs, so identical content
    /// always maps to one physical file. The write is atomic — bytes go to a
    /// temp file in the leaf directory and are renamed into place — so a
    /// concurrent reader never observes a partial blob.
    pub async fn put(&self, bytes: &[u8]) -> Result<String, BlobError> {
        let hash = sha256_hex(bytes);
        let path = self.path_for(&hash)?;

        // Dedup: already present → nothing to write.
        if tokio::fs::try_exists(&path).await? {
            return Ok(hash);
        }

        let dir = path
            .parent()
            .expect("blob path always has a fan-out parent dir");
        tokio::fs::create_dir_all(dir).await?;

        // Atomic publish: write to a unique temp file, then rename into place.
        // A rename onto an existing target is harmless (lost race → identical
        // bytes), so we ignore an AlreadyExists at the final existence check.
        let tmp = dir.join(format!(".tmp-{hash}-{}", unique_suffix()));
        {
            let mut f = File::create(&tmp).await?;
            f.write_all(bytes).await?;
            f.sync_all().await?;
        }
        match tokio::fs::rename(&tmp, &path).await {
            Ok(()) => {}
            Err(e) => {
                // Clean the temp file; if the target now exists (race), succeed.
                let _ = tokio::fs::remove_file(&tmp).await;
                if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
                    return Err(BlobError::Io(e));
                }
            }
        }
        Ok(hash)
    }

    /// Open a stored blob for reading by its `hash`.
    ///
    /// Returns the bytes' `File` handle; the caller streams it (e.g. via
    /// `tokio_util::io::ReaderStream` on the download path). Errors with a
    /// `NotFound` I/O error when no such blob exists.
    pub async fn open(&self, hash: &str) -> Result<File, BlobError> {
        let path = self.path_for(hash)?;
        Ok(File::open(&path).await?)
    }

    /// Whether a blob with `hash` is present on disk.
    pub async fn exists(&self, hash: &str) -> Result<bool, BlobError> {
        let path = self.path_for(hash)?;
        Ok(tokio::fs::try_exists(&path).await?)
    }

    /// Read a stored blob fully into memory. Convenience for small blobs/tests.
    pub async fn read_all(&self, hash: &str) -> Result<Vec<u8>, BlobError> {
        let path = self.path_for(hash)?;
        Ok(tokio::fs::read(&path).await?)
    }

    /// Remove a blob by `hash`. A missing blob is treated as success (the
    /// post-condition "no such blob" already holds), so reclamation is
    /// idempotent. Empty fan-out directories are best-effort pruned.
    pub async fn remove(&self, hash: &str) -> Result<(), BlobError> {
        let path = self.path_for(hash)?;
        match tokio::fs::remove_file(&path).await {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(BlobError::Io(e)),
        }
        // Best-effort prune of now-empty `aa/bb` then `aa` dirs.
        if let Some(bb) = path.parent() {
            let _ = tokio::fs::remove_dir(bb).await;
            if let Some(aa) = bb.parent() {
                let _ = tokio::fs::remove_dir(aa).await;
            }
        }
        Ok(())
    }
}

/// Compute the lowercase-hex SHA-256 of `bytes`.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Whether `s` is a 64-char lowercase-hex string (a SHA-256 digest).
#[must_use]
pub fn is_sha256_hex(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Best-effort absolutize: if the path is relative, join it onto the current
/// working dir. We do not canonicalise (the leaf may not exist yet); §1 only
/// requires an absolute path resolved at startup.
fn absolutize(p: &Path) -> PathBuf {
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(p))
            .unwrap_or_else(|_| p.to_path_buf())
    }
}

/// A process-unique suffix for temp-file names (no external uuid dep).
fn unique_suffix() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{t}-{}-{n}", std::process::id())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A throwaway BlobStore rooted under the OS temp dir, unique per test.
    fn temp_store() -> (BlobStore, PathBuf) {
        let root = std::env::temp_dir().join(format!("ost-blob-test-{}", unique_suffix()));
        (BlobStore::new(&root), root)
    }

    fn cleanup(root: &Path) {
        let _ = std::fs::remove_dir_all(root);
    }

    /// AC-1: putting identical bytes twice yields one blob + one hash (dedup).
    #[tokio::test]
    async fn dedup() {
        let (store, root) = temp_store();
        let bytes = b"the same exact bytes";

        let h1 = store.put(bytes).await.unwrap();
        let h2 = store.put(bytes).await.unwrap();
        assert_eq!(h1, h2, "identical bytes must yield the same SHA-256");

        // Exactly one file on disk under <root>/aa/bb/<sha256>.
        let path = store.path_for(&h1).unwrap();
        assert!(path.exists(), "blob must exist at {path:?}");
        assert_eq!(
            path,
            root.join(&h1[0..2]).join(&h1[2..4]).join(&h1),
            "layout must be <root>/aa/bb/<sha256>"
        );

        // Count regular files anywhere under root → exactly one.
        let count = count_files(&root);
        assert_eq!(count, 1, "dedup must leave exactly one physical blob");

        cleanup(&root);
    }

    /// AC-2: a stored blob round-trips byte-for-byte via open(hash).
    #[tokio::test]
    async fn roundtrip() {
        let (store, root) = temp_store();
        let bytes: Vec<u8> = (0u8..=255).cycle().take(5000).collect();

        let hash = store.put(&bytes).await.unwrap();

        // open(hash) yields exactly the written bytes.
        use tokio::io::AsyncReadExt;
        let mut f = store.open(&hash).await.unwrap();
        let mut got = Vec::new();
        f.read_to_end(&mut got).await.unwrap();
        assert_eq!(got, bytes, "open(hash) must round-trip the exact bytes");

        // read_all helper agrees.
        assert_eq!(store.read_all(&hash).await.unwrap(), bytes);

        // The on-disk path equals <root>/aa/bb/<sha256>.
        let expected = root.join(&hash[0..2]).join(&hash[2..4]).join(&hash);
        assert_eq!(store.path_for(&hash).unwrap(), expected);

        cleanup(&root);
    }

    #[tokio::test]
    async fn hash_matches_known_sha256() {
        // SHA-256("abc") is a fixed, well-known vector.
        let (store, root) = temp_store();
        let h = store.put(b"abc").await.unwrap();
        assert_eq!(
            h,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        cleanup(&root);
    }

    #[tokio::test]
    async fn open_missing_is_not_found() {
        let (store, root) = temp_store();
        let missing = "0".repeat(64);
        let err = store.open(&missing).await.unwrap_err();
        match err {
            BlobError::Io(e) => assert_eq!(e.kind(), io::ErrorKind::NotFound),
            other => panic!("expected NotFound I/O error, got {other:?}"),
        }
        cleanup(&root);
    }

    #[tokio::test]
    async fn bad_hash_is_rejected() {
        let (store, _root) = temp_store();
        assert!(matches!(
            store.path_for("not-hex"),
            Err(BlobError::InvalidHash(_))
        ));
        assert!(matches!(
            store.path_for(&"A".repeat(64)), // uppercase rejected
            Err(BlobError::InvalidHash(_))
        ));
    }

    #[tokio::test]
    async fn remove_is_idempotent_and_reclaims() {
        let (store, root) = temp_store();
        let h = store.put(b"reclaim me").await.unwrap();
        assert!(store.exists(&h).await.unwrap());
        store.remove(&h).await.unwrap();
        assert!(!store.exists(&h).await.unwrap());
        // Second remove is a no-op (idempotent).
        store.remove(&h).await.unwrap();
        cleanup(&root);
    }

    #[test]
    fn from_env_default_is_workspace_var_blobs() {
        // With BLOB_ROOT unset, the default is <workspace>/var/blobs (absolute).
        // (We avoid mutating the process env in tests; build the default directly.)
        let ws = std::env::temp_dir().join("ws-root");
        let store = BlobStore::new(absolutize(&ws.join(DEFAULT_BLOB_SUBDIR)));
        assert!(store.root().is_absolute());
        assert!(store.root().ends_with("var/blobs"));
    }

    fn count_files(dir: &Path) -> usize {
        let mut n = 0;
        if let Ok(entries) = std::fs::read_dir(dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    n += count_files(&p);
                } else if p.is_file() {
                    n += 1;
                }
            }
        }
        n
    }
}
