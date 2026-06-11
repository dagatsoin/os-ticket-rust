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

> Setup (all): `cargo run -p tools --bin seed -- --reset`; backend on :3701; fixtures in `/tmp/qa-fixtures`
> (see US-M2-1 Test Infrastructure for the creation block). Staff reads use a cookie jar from
> `POST /api/staff/login` (`agent`/`Agent123!`).

### AC-1: FS-011.7 — create with a permitted file returns 201 and binds a ticket_attachment to the M entry. [API-ONLY]
- Request: `curl -i -X POST http://localhost:3701/api/tickets -F name=Mia -F email=mia@example.com -F subject=Inv -F message=see -F attachment=@/tmp/qa-fixtures/invoice.pdf`.
- Verify: HTTP 201 + a 6-digit ticket number; `curl -b /tmp/qa-staff.jar http://localhost:3701/api/staff/tickets/{id}` shows the `M` entry with `attachments` listing `invoice.pdf` (name/size/mime).
- Status: [ ]

### AC-2: EC-011.5 — create with a disallowed/oversized file returns 422 and creates no ticket. [API-ONLY]
- Request: `curl -i -X POST .../api/tickets -F name=Mia -F email=mia@example.com -F subject=Bad -F message=x -F attachment=@/tmp/qa-fixtures/evil.exe`; separately repeat with `@/tmp/qa-fixtures/big.pdf`.
- Verify: both return HTTP 422 with a field error keyed on `attachment` (§2); `docker exec backend-db-1 psql -U postgres -d osticket_dev -c "select count(*) from ticket;"` is unchanged and `find "${BLOB_ROOT:-var/blobs}" -type f | wc -l` is unchanged (no ticket, no blob).
- Status: [ ]

### AC-3: D1 — two creates with identical bytes share one attachment_file / one blob. [API-ONLY]
- Request: run the AC-1 `curl` twice with the SAME `/tmp/qa-fixtures/invoice.pdf` bytes.
- Verify: two ticket numbers; `psql -c "select count(*) from ticket_attachment;"` → 2; `select count(*) from attachment_file;` → 1 (same SHA-256); `find "${BLOB_ROOT:-var/blobs}" -type f | wc -l` → 1 blob.
- Status: [ ]

### AC-4: create with NO attachment still returns 201 (M1 unchanged). [API-ONLY]
- Request: `curl -i -X POST .../api/tickets -H 'Content-Type: application/json' -d '{"name":"Mia","email":"mia@example.com","subject":"Plain","message":"hi"}'`.
- Verify: HTTP 201; the new ticket's `M` entry has `attachments: []` (no `ticket_attachment`), M1 contract preserved.
- Status: [ ]

## Dependencies

- TS-M2-A1 (store + schema), TS-M2-A2 (validation), TS-M1-B1/B2 (core + create route).
