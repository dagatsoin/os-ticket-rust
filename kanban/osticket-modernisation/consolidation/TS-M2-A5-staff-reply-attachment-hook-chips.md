# TS-M2-A5 — update(route+ui): FS-021.3 staff reply attachment hook + thread chips

- **ID**: TS-M2-A5
- **Type**: Technical Story
- **Parent**: EPIC-M2-A
- **Labels**: Technical Story, M2
- **Scope**: small

## Context

Extends the M1 staff reply route (TS-M1-C3) + staff/client thread views so a reply can carry an
attachment and every thread entry renders its bound attachments as chips. Shared by US-M2-2 (staff
reply own-file) and US-M2-3 (download chips). Lives in Epic A (the upload/render half); the
download/auth half is Epic B.

## Impact

- update(route): `POST /api/staff/tickets/{id}/reply` accepts an optional multipart `attachment`; validate (A2), store (A1), bind `ticket_attachment` (ref_type `R`, ref_id = the new `R` entry).
- update(api): ticket-detail + client-thread responses include each entry's attachments (`id`, `name`, `size`, `mime`).
- update(ui): render the **AttachmentChip** (TS-M2-A4) on every thread entry in the staff detail view AND the client portal thread (download wiring is TS-M2-B2).

## Regressions

- A reply with no attachment must post exactly as M1 (TS-M1-C3). The client portal thread (US-M1-4) must still render unchanged when no attachments exist.

## Acceptance Tests

### AC-1: FS-021.3 — a staff reply with a permitted file binds a ticket_attachment (ref_type R) to the new entry. [API-ONLY]
- Multipart reply with `note.png` → new `R` entry has a `ticket_attachment` for `note.png`.
- Status: [ ]

### AC-2: a disallowed/oversized reply attachment returns 422 and posts no reply. [API-ONLY]
- Multipart reply with `evil.exe` → 422 field error; no `R` entry created.
- Status: [ ]

### AC-3: ticket-detail + client-thread responses expose each entry's attachments. [API-ONLY]
- GET staff detail + client thread → entries list their attachment metadata.
- Status: [ ]

### AC-4: thread entries render an AttachmentChip in both staff detail and the client portal. [BROWSER]
- Open a ticket with attachments as staff and as the client → chips appear on the right entries.
- Status: [ ]

## Dependencies

- TS-M2-A1, TS-M2-A2, TS-M2-A4 (chip component), TS-M1-C3 (reply route), TS-M1-C4/D2 (thread views).
