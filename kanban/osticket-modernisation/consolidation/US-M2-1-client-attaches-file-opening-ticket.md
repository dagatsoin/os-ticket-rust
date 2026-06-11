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
- Setup: `cargo run -p tools --bin seed -- --reset`.
- Navigate: http://localhost:3702/open
- Verify: alongside Name/Email/Subject/Message there is an **attachment file input** with helper text naming the allowed types and the 1 MB max.
- Status: [ ]

### AC-2: Submitting a valid form WITH a permitted file shows a confirmation page with a ticket number AND an attachment chip. [BROWSER]
- Navigate: http://localhost:3702/open
- Action: fill Name "Mia Wong", Email "mia@example.com", Subject "Invoice query", Message "See the attached invoice."; choose a < 1 MB `invoice.pdf`.
- Action: submit.
- Verify: the confirmation page renders a 6-digit ticket number AND an **attachment chip** labelled `invoice.pdf`. Record {number, email} for downstream flows.
- Status: [ ]

### AC-3: A disallowed file type shows an inline validation error and creates no ticket. [BROWSER]
- Navigate: http://localhost:3702/open
- Action: fill valid Name/Email/Subject/Message; choose a file with a disallowed extension (e.g. `evil.exe`).
- Action: attempt to submit.
- Verify: an **inline error on the attachment field** ("Invalid file type" / not-allowed message); no confirmation page; logging in as staff (agent / Agent123!) shows no new ticket in the Open queue.
- Status: [ ]

### AC-4: An oversized file (> 1 MB) shows an inline validation error and creates no ticket. [BROWSER]
- Navigate: http://localhost:3702/open
- Action: fill valid fields; choose a permitted-extension file larger than 1 MB.
- Action: attempt to submit.
- Verify: an **inline "too big" error** on the attachment field; no confirmation page; no new ticket in the staff Open queue.
- Status: [ ]

### AC-5: The attachment persists bound to the ticket's first message and dedups identical bytes. [API-ONLY]
- Setup: create a ticket via the public route with a multipart `.pdf`, twice with the SAME bytes.
- Request: read each ticket's detail as staff (login agent / Agent123!, then `GET /api/staff/tickets/{id}`).
- Expect: each ticket's `M` entry lists a `ticket_attachment` for the file with the right name/size; both reference the SAME `attachment_file` row (one `storage_key` / SHA-256 — D1 dedup); exactly one blob exists on disk under `var/blobs/`.
- Status: [ ]

## Checklist (children)

- [ ] TS-M2-A3 — Public create-ticket route accepts + validates + binds an attachment
- [ ] TS-M2-A4 — `/open` file input + inline validation error + confirmation chip

## Test Infrastructure

- Small fixture files: a < 1 MB `invoice.pdf` (valid), an `evil.exe` (bad type), a > 1 MB permitted-type file (oversized). Document their paths in the TS tickets.
- The staff Open queue (US-M1-3, done) is the in-browser persistence oracle for AC-3/AC-4.

## Dependencies

- TS-M2-A1 (blob store + schema), TS-M2-A2 (upload validation + config keys). M1 public create route.
