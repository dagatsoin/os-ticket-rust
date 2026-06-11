# US-M2-1 — Attachments – Client attaches a file when opening a ticket

- **ID**: US-M2-1
- **Type**: User Story
- **Parent**: EPIC-M2-A
- **Labels**: User Story, M2
- **Scope**: small

## Spec References

- FS-011 (FS-011.7 attachment on public submission; EC-011.5 oversized/disallowed upload)
- FS-022 (FS-022.13 upload validation; FS-022.4 binding; BS-022.13/.14 extension allow-list)

## Context

Epic: EPIC-M2-A — Attachment storage & upload.
Extends the M1 public open-ticket form so a visitor can attach a file to the request they submit.
This is the first step of the M2 demo path (root AC-1) and the producer of the file that the client
later downloads (root AC-5).

## Description

As a visitor opening a support ticket, I can attach a file (e.g. an invoice PDF) to my request, see
it confirmed on the success page, and trust that an oversized or wrong-type file is rejected with a
clear message before my ticket is created.

## Impact

- Frontend (file input on `/open`; inline validation error; attachment chip on the confirmation page)
- Backend (public create route accepts a multipart upload; validates; stores blob; binds to the ticket's `M` thread entry)
- Database (`attachment_file`, `ticket_attachment`)
- Filesystem (`var/blobs/ab/cd/<sha256>`)
- Browser (desktop)

## Business Rules

- BS-022.13: allowed file types are matched by **extension** against the configured allow-list (seed `.pdf,.png,.jpg,.txt,.doc`).
- BS-022.14: an empty allow-list denies everything; `.*` allows all (M2 seeds a real list, so a disallowed extension is rejected).
- FS-022.13: a file larger than `max_file_size` (seed **1 MB**) is rejected with a per-file error.
- Attachments are gated by `allow_attachments` (seed enabled); when disabled the file input is hidden.

## Regressions

- M1 ticket-create without an attachment must still succeed (the file input is optional). Verify the M1 round-trip (US-M1-2) still passes with no file selected.

## Acceptance Criteria

### AC-1: The /open form shows a file input when attachments are enabled. [BROWSER]
- Setup: `cargo run -p tools --bin seed -- --reset`; fixtures present (see Test Infrastructure).
- Navigate: http://localhost:3702/open
- Verify: alongside Name/Email/Subject/Message there is an **attachment file input** with helper text naming the allowed types and the 1 MB max.
- Status: [x]

### AC-2: Submitting a valid form WITH a permitted file shows a confirmation page with a ticket number AND an attachment chip. [BROWSER]
- Navigate: http://localhost:3702/open
- Action: fill Name "Mia Wong", Email "mia@example.com", Subject "Invoice query", Message "See the attached invoice."; in the attachment file input choose `/tmp/qa-fixtures/invoice.pdf` (< 1 MB).
- Action: submit.
- Verify: the confirmation page renders a 6-digit ticket number AND an **attachment chip** labelled `invoice.pdf`. Record {number, email} for downstream flows.
- Status: [x]

### AC-3: A disallowed file type shows an inline validation error and creates no ticket. [BROWSER]
- Navigate: http://localhost:3702/open
- Action: fill valid Name/Email/Subject/Message; choose `/tmp/qa-fixtures/evil.exe` (disallowed extension).
- Action: attempt to submit.
- Verify: an **inline error on the attachment field** ("Invalid file type" / not-allowed message); no confirmation page; logging in as staff (agent / Agent123!) shows no new ticket in the Open queue.
- Status: [x]

### AC-4: An oversized file (> 1 MB) shows an inline validation error and creates no ticket. [BROWSER]
- Navigate: http://localhost:3702/open
- Action: fill valid fields; choose `/tmp/qa-fixtures/big.pdf` (permitted extension, > 1 MB).
- Action: attempt to submit.
- Verify: an **inline "too big" error** on the attachment field; no confirmation page; no new ticket in the staff Open queue.
- Status: [x]

### AC-5: The attachment persists bound to the ticket's first message and dedups identical bytes. [API-ONLY]
- Setup: `cargo run -p tools --bin seed -- --reset`; fixtures present (see Test Infrastructure).
- Request: create a ticket via the public multipart route with the SAME `.pdf` bytes, twice (capture both ticket numbers):
  `curl -s -X POST http://localhost:3701/api/tickets -F name=Mia -F email=mia@example.com -F subject=Dup -F message=one -F attachment=@/tmp/qa-fixtures/invoice.pdf` then repeat the command.
- Request: read each ticket's detail as staff — `curl -c /tmp/qa-staff.jar -X POST http://localhost:3701/api/staff/login` with `agent`/`Agent123!` (keep the `ost_staff_sess` cookie jar), then `curl -b /tmp/qa-staff.jar http://localhost:3701/api/staff/tickets/{id}` for each ticket.
- Verify (DB): `docker exec backend-db-1 psql -U postgres -d osticket_dev -c "select count(*) from attachment_file; select count(*) from ticket_attachment;"` → 1 `attachment_file` row, 2 `ticket_attachment` rows.
- Verify (disk): `find "${BLOB_ROOT:-var/blobs}" -type f | wc -l` → exactly 1 blob; each ticket's `M` entry in the detail JSON lists the file under `attachments` with the right name/size, both pointing at the same `attachment_file` id (one `storage_key` / SHA-256 — D1 dedup).
- Status: [x] — D1 dedup verified: 2 uploads of identical invoice.pdf -> +1 attachment_file (id=52, hash cfa3...), +1 disk blob (cf/a3/...), +2 ticket_attachment rows (tickets 191532 & 507152, both file_id=52, ref_type M). Both staff detail JSONs list invoice.pdf size 69 under the M entry. Absolute counts read 2/2 not 1/1 only because the seed reset preserves the canned policy.txt blob (attachment_file id=4); the upload DELTA matches the AC exactly.

## Checklist (children)

- [ ] TS-M2-A3 — Public create-ticket route accepts + validates + binds an attachment
- [ ] TS-M2-A4 — `/open` file input + inline validation error + confirmation chip

## Test Infrastructure

- **Fixture files** (create once before the sweep, shared across all M2 upload tickets):
  ```sh
  mkdir -p /tmp/qa-fixtures
  printf '%%PDF-1.4\n1 0 obj<</Type/Catalog>>endobj\ntrailer<</Root 1 0 R>>\n%%%%EOF\n' > /tmp/qa-fixtures/invoice.pdf   # valid, <1 MB
  printf 'MZ\220\000\003' > /tmp/qa-fixtures/evil.exe                                                                      # disallowed type
  { printf '%%PDF-1.4\n'; head -c 1048577 /dev/zero | tr '\0' A; } > /tmp/qa-fixtures/big.pdf                              # permitted type, >1 MB
  ```
  Paths: `/tmp/qa-fixtures/invoice.pdf` (valid), `/tmp/qa-fixtures/evil.exe` (bad type), `/tmp/qa-fixtures/big.pdf` (oversized).
- The staff Open queue (US-M1-3, done) is the in-browser persistence oracle for AC-3/AC-4.
- `BLOB_ROOT` (ROADMAP M2 §1) defaults to `<workspace root>/var/blobs`; AC-5's disk assertion honors it.

## Dependencies

- TS-M2-A1 (blob store + schema), TS-M2-A2 (upload validation + config keys). M1 public create route.
