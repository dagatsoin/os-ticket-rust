# TS-M2-D4 — update(route): BS-022.15 post a canned reply (carry attachments, mark unanswered)

- **ID**: TS-M2-D4
- **Type**: Technical Story
- **Parent**: US-M2-2
- **Labels**: Technical Story, M2
- **Scope**: small

## Context

Extends the reply route so a posted reply can reference a canned id (carrying its attachments) and/or
an own-file upload, and marks the ticket unanswered per BS-022.15.

**Documented divergence (pinned in EPIC-M2-D):** legacy BS-022.15 records the poster as
"SYSTEM (Canned Reply)"; M2 keeps the **posting agent** as author (no SYSTEM actor in the M1 model
yet). Substitution, attachment-carry, and the unanswered flag ARE preserved.

## Impact

- update(route): `POST /api/staff/tickets/{id}/reply` accepts an optional `cannedId` and an optional own `attachment`.
- update(core): if `cannedId` present, the posted body is the substituted canned body (re-rendered server-side for integrity) and the canned response's attachments are bound to the new `R` entry **by file id** (no re-upload — D1 dedup); an own file is validated/stored/bound too.
- update(core): a canned reply marks the ticket **unanswered** (BS-022.15).

## Regressions

- A plain reply (no cannedId, no file) must behave as M1 (TS-M1-C3); a reply with only an own file works (shares TS-M2-A5).

## Acceptance Tests

### AC-1: FS-022.14 — posting with a cannedId appends an R entry with the substituted body + carried attachments. [API-ONLY]
- POST reply with `cannedId` for "Acknowledge receipt" → new `R` entry, body substituted, a `ticket_attachment` for `policy.txt` (same file id as the canned attachment — no new blob).
- Status: [ ]

### AC-2: BS-022.15 — a posted canned reply marks the ticket unanswered. [API-ONLY]
- After the canned reply → ticket detail shows the unanswered/awaiting-customer flag set.
- Status: [ ]

### AC-3: own-file + canned in one reply both bind. [API-ONLY]
- POST with `cannedId` + an own `note.png` → the `R` entry carries both `policy.txt` (canned) and `note.png` (own).
- Status: [ ]

### AC-4: a plain reply (no canned, no file) still posts (M1 unchanged). [API-ONLY]
- POST a plain reply → new `R` entry, no attachments; status behaviour unchanged from M1 baseline.
- Status: [ ]

## Dependencies

- TS-M2-D2 (canned fetch/substitution), TS-M2-D1 (canned attachments), TS-M2-A5 (reply attachment hook), TS-M1-C3 (reply route).
