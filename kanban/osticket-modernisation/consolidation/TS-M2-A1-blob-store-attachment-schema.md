# TS-M2-A1 — add(store): FS-022.12 SHA-256 filesystem blob store + attachment schema

- **ID**: TS-M2-A1
- **Type**: Technical Story
- **Parent**: EPIC-M2-A
- **Labels**: Technical Story, M2
- **Scope**: small

## Context

Foundation of all M2 attachments. Builds the blob store + the two reference tables once; every later
attachment ticket (A3 create hook, A5 reply hook, B1 download, D1 canned attachments) is a thin
consumer of this.

**DEVIATION D1 (pinned):** filesystem blob store keyed by **content SHA-256** at
`<BLOB_ROOT>/<aa>/<bb>/<full-sha256>` (first two hex pairs as fan-out dirs), giving **real byte-level
dedup**. No chunked-DB table; the legacy FS-022.12 chunk store is NOT reproduced (KL-022.4
obsoleted — identical bytes share one blob).

**Blob root (ROADMAP M2 Decisions §1):** env `BLOB_ROOT`, default `<workspace root>/var/blobs`,
resolved to an absolute path at startup; all binaries and tests honor it.

**Crates (ROADMAP M2 Decisions §4):** SHA-256 hashing via `sha2`; hex encoding of the digest via `hex`.

## Impact

- add(store): a `BlobStore` in `ost_core` — `put(bytes) -> sha256` (idempotent: existing blob reused), `open(sha256) -> reader`, path `<BLOB_ROOT>/aa/bb/<sha256>` (BLOB_ROOT resolved absolute at startup, default `<workspace root>/var/blobs`).
- add(migration): `attachment_file` (`id`, `mime`, `size`, `hash` = sha256 unique, `name`, `storage_key`, `created`).
- add(migration): `ticket_attachment` (`id`, `ticket_id`, `file_id` → attachment_file, `ref_id` = thread entry id, `ref_type` enum `M`/`R`/`N`).
- update(.sqlx): regenerate the offline query cache per ROADMAP Decisions §5.

## Regressions

- New tables/store only; touches no existing M1 row. Verify M1 migrations still apply cleanly on top.

## Acceptance Tests

### AC-1: FS-022.12/D1 — putting identical bytes twice yields one blob + one attachment_file hash. [API-ONLY]
- Test: `store.put(b)` twice with identical bytes → same SHA-256, one file on disk, one `attachment_file` row reused.
- Status: [ ]

### AC-2: FS-022.12 — a stored blob round-trips byte-for-byte via open(hash). [API-ONLY]
- Test: `open(hash)` returns exactly the bytes written; the on-disk path matches `<BLOB_ROOT>/aa/bb/<sha256>`.
- Status: [ ]

### AC-3: schema — a ticket_attachment binds a file to a ticket + thread entry with ref_type M/R/N. [API-ONLY]
- Test: insert an `attachment_file` then a `ticket_attachment` (ref_type `M`); FK to thread entry + attachment_file enforced; ref_type constrained to M/R/N.
- Status: [ ]

## Dependencies

- M1 (ost_core, migrations harness, ticket/thread schema).
