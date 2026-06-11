# TS-M2-A5 — update(route): FS-021.3 staff reply attachment hook + attachments in thread payloads (BACKEND)

- **ID**: TS-M2-A5
- **Type**: Technical Story
- **Parent**: EPIC-M2-A
- **Labels**: Technical Story, M2
- **Scope**: small

## Context

Extends the M1 staff reply route (TS-M1-C3) so a reply can carry an attachment, and extends the
staff-detail + client-thread response payloads so every thread entry exposes its bound attachments.
**This ticket is BACKEND ONLY (ROADMAP M2 Decisions §10):** the reply multipart hook + the
`attachments` array in thread payloads. Chip *rendering* + click-to-download is TS-M2-B2; the reply
*composer* UI (canned dropdown + own-file input + carried chips) is TS-M2-D3.

**Multipart strategy (ROADMAP M2 Decisions §2):** the reply route **dual-accepts by Content-Type** —
`application/json` (the M1 contract, no attachment) OR `multipart/form-data` (`body`/`cannedId` as
form parts plus an optional `attachment` file part); 422 file errors key on `attachment`. Blobs are
stored under `BLOB_ROOT` (§1). Hashing via `sha2`/`hex` (§4).

## Impact

- update(route): `POST /api/staff/tickets/{id}/reply` dual-accepts by Content-Type; the multipart variant carries an optional `attachment` — validate (A2), store (A1), bind `ticket_attachment` (ref_type `R`, ref_id = the new `R` entry).
- update(api): ticket-detail + client-thread responses include each entry's attachments under the key **`attachments: [{id, name, size, mime}]`** — the SAME shape used by canned detail (D2) and consumed by the chips (B2) (ROADMAP M2 Decisions §7).
- **(no UI in this ticket)** — chip rendering/download is TS-M2-B2; the reply composer is TS-M2-D3.

## Regressions

- A reply with no attachment must post exactly as M1 (TS-M1-C3). The client portal thread (US-M1-4) must still render unchanged when no attachments exist.

## Acceptance Tests

### AC-1: FS-021.3 — a staff reply with a permitted file binds a ticket_attachment (ref_type R) to the new entry. [API-ONLY]
- Multipart reply with `note.png` → new `R` entry has a `ticket_attachment` for `note.png`.
- Status: [ ]

### AC-2: a disallowed/oversized reply attachment returns 422 and posts no reply. [API-ONLY]
- Multipart reply with `evil.exe` → 422 field error; no `R` entry created.
- Status: [ ]

### AC-3: ticket-detail + client-thread responses expose each entry's attachments under `attachments: [{id,name,size,mime}]`. [API-ONLY]
- GET staff detail + client thread → entries list their attachment metadata under the shared `attachments` key (§7), the same shape as canned detail (D2).
- Status: [ ]

### AC-4: a JSON (no-attachment) reply still posts and returns an empty `attachments` array (M1 contract preserved). [API-ONLY]
- POST a plain JSON reply → new `R` entry with `attachments: []`; the M1 reply behaviour is unchanged.
- Status: [ ]

## Dependencies

- TS-M2-A1, TS-M2-A2, TS-M1-C3 (reply route). (Chip rendering/download is B2; the reply composer is D3 — not dependencies of this backend ticket.)
