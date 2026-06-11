# TS-M2-A3 — update(route): FS-011.7 public create accepts + binds an attachment

- **ID**: TS-M2-A3
- **Type**: Technical Story
- **Parent**: US-M2-1
- **Labels**: Technical Story, M2
- **Scope**: small

## Context

Extends the M1 public create route (TS-M1-B2) to accept a multipart upload, validate it (A2), store
the blob (A1), and bind it to the new ticket's first `M` thread entry.

**Multipart strategy (ROADMAP M2 Decisions §2):** enable the axum `multipart` feature. The route
**DUAL-ACCEPTS by Content-Type** — `application/json` keeps the M1 contract unchanged (no attachment),
`multipart/form-data` carries `name`/`email`/`subject`/`message` as individual form parts plus an
optional `attachment` file part. The 422 field key for file errors is `attachment`. Blobs are stored
under `BLOB_ROOT` (ROADMAP M2 Decisions §1).

## Impact

- update(route): `POST /api/tickets` **dual-accepts by Content-Type** — `application/json` (M1 JSON fields, no attachment) OR `multipart/form-data` (the same fields as individual form parts plus an optional `attachment` file part).
- update(core): on create, validate the upload (A2), `store.put` the bytes (A1), insert `attachment_file` (if new) + `ticket_attachment` (ref_type `M`, ref_id = the new `M` entry).
- A failed upload validation returns 422 with a field error on `attachment`; the ticket is NOT created.

## Regressions

- A create request with NO attachment must behave exactly as M1 (still 201). Re-run TS-M1-B2 ACs.

## Acceptance Tests

### AC-1: FS-011.7 — create with a permitted file returns 201 and binds a ticket_attachment to the M entry. [API-ONLY]
- Request: multipart POST with valid fields + `invoice.pdf`.
- Expect: 201 + ticket number; staff detail shows the `M` entry with a `ticket_attachment` for `invoice.pdf`.
- Status: [ ]

### AC-2: EC-011.5 — create with a disallowed/oversized file returns 422 and creates no ticket. [API-ONLY]
- Request: multipart POST with `evil.exe` (and separately a > 1 MB file).
- Expect: 422 with a field error on `attachment`; no ticket, no blob.
- Status: [ ]

### AC-3: D1 — two creates with identical bytes share one attachment_file / one blob. [API-ONLY]
- Request: two multipart creates with the SAME `.pdf` bytes.
- Expect: two tickets, two `ticket_attachment` rows, ONE `attachment_file` (same SHA-256), one blob on disk.
- Status: [ ]

### AC-4: create with NO attachment still returns 201 (M1 unchanged). [API-ONLY]
- Request: JSON (or multipart with no file part) valid create.
- Expect: 201, no `ticket_attachment`.
- Status: [ ]

## Dependencies

- TS-M2-A1 (store + schema), TS-M2-A2 (validation), TS-M1-B1/B2 (core + create route).
