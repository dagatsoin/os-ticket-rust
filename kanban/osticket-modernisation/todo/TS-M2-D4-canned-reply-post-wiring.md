# TS-M2-D4 — update(route): post a canned reply (carry attachments, mark answered like any reply)

- **ID**: TS-M2-D4
- **Type**: Technical Story
- **Parent**: US-M2-2
- **Labels**: Technical Story, M2
- **Scope**: small

## Context

Extends the reply route so a posted reply can reference a canned id (carrying its attachments) and/or
an own-file upload.

**isanswered semantics (ROADMAP M2 Decisions §3 — pinned):** legacy-faithful. **ANY staff reply —
plain OR canned-assisted — marks the ticket `isanswered=true`**, exactly like an M1 reply. BS-022.15's
"mark unanswered" applies to the filter-driven SYSTEM auto-reply path, which is **DEFERRED to M5**
(documented deviation); a canned-assisted reply posted by an agent is NOT that path.

**Documented divergence (pinned in EPIC-M2-D):** legacy BS-022.15 records the poster as
"SYSTEM (Canned Reply)"; M2 keeps the **posting agent** as author (no SYSTEM actor in the M1 model
yet). Substitution and attachment-carry ARE preserved.

**Multipart strategy (§2):** the route dual-accepts by Content-Type; the multipart variant carries
`body`/`cannedId` as form parts plus an optional `attachment` file part.

## Impact

- update(route): `POST /api/staff/tickets/{id}/reply` accepts an optional `cannedId` and an optional own `attachment` (multipart, §2).
- update(core): if `cannedId` present, the posted body is the substituted canned body (re-rendered server-side for integrity) and the canned response's attachments are bound to the new `R` entry **by file id** (no re-upload — D1 dedup); an own file is validated/stored/bound too. Bound attachments surface under the shared `attachments` key (§7).
- update(core): a canned-assisted reply marks the ticket **answered (`isanswered=true`)**, like any staff reply (§3) — it does NOT mark it unanswered.

## Regressions

- A plain reply (no cannedId, no file) must behave as M1 (TS-M1-C3); a reply with only an own file works (shares TS-M2-A5).

## Acceptance Tests

> Setup (all): `cargo run -p tools --bin seed -- --reset`; backend on :3701; fixtures in `/tmp/qa-fixtures`.
> Seed a ticket `{id}`; staff login to `/tmp/qa-staff.jar`; obtain the "Acknowledge receipt" `{cannedId}`
> from `GET /api/staff/tickets/{id}/canned`.

### AC-1: FS-022.14 — posting with a cannedId appends an R entry with the substituted body + carried attachments. [API-ONLY]
- Request: `curl -i -b /tmp/qa-staff.jar -X POST http://localhost:3701/api/staff/tickets/{id}/reply -F cannedId={cannedId} -F body=''`.
- Verify: HTTP 200/201; the new `R` entry's body is the substituted canned text (ticket number present, no literal `%{...}`) and `attachments` lists `policy.txt`; `psql -c "select count(*) from attachment_file;"` is unchanged from before the reply (the canned blob is re-bound by file id, no new blob — D1).
- Status: [ ]

### AC-2: §3 — a posted canned reply marks the ticket answered (isanswered=true), like any staff reply. [API-ONLY]
- Setup: before replying, `curl -b /tmp/qa-staff.jar .../api/staff/tickets/{id}` shows `isanswered=false`.
- Request: post the AC-1 canned reply.
- Verify: ticket detail now shows `isanswered=true` (Answered), the same as a plain M1 reply (BS-022.15's "mark unanswered" SYSTEM path is DEFERRED to M5); the `R` entry's author is the posting agent (not "SYSTEM").
- Status: [ ]

### AC-3: own-file + canned in one reply both bind. [API-ONLY]
- Request: `curl -i -b /tmp/qa-staff.jar -X POST .../api/staff/tickets/{id}/reply -F cannedId={cannedId} -F body='' -F attachment=@/tmp/qa-fixtures/note.png`.
- Verify: the `R` entry's `attachments` carries BOTH `policy.txt` (canned) and `note.png` (own).
- Status: [ ]

### AC-4: a plain reply (no canned, no file) still posts (M1 unchanged). [API-ONLY]
- Request: `curl -i -b /tmp/qa-staff.jar -X POST .../api/staff/tickets/{id}/reply -H 'Content-Type: application/json' -d '{"body":"plain"}'`.
- Verify: HTTP 200/201; new `R` entry with `attachments: []`; status behaviour unchanged from the M1 baseline.
- Status: [ ]

## Dependencies

- TS-M2-D2 (canned fetch/substitution), TS-M2-D1 (canned attachments), TS-M2-A5 (reply attachment hook), TS-M1-C3 (reply route).
