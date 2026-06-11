# TS-M2-B2 — add(ui): FS-022.10 clickable attachment chips → download (client portal + staff detail)

- **ID**: TS-M2-B2
- **Type**: Technical Story
- **Parent**: US-M2-3
- **Labels**: Technical Story, M2
- **Scope**: small

## Context

Owns the **chip rendering + click-to-download** in BOTH the client portal thread and the staff detail
thread (ROADMAP M2 Decisions §10). Consumes the `attachments: [{id,name,size,mime}]` arrays that A5
adds to the thread payloads (§7) and the B1 download routes. The reply *composer* chips (carried
canned attachments, read-only) are TS-M2-D3; this ticket renders thread-entry chips in both
read views and makes every chip clickable.

**Download UX (ROADMAP M2 Decisions §9):** a chip click uses **fetch-with-credentials → blob →
object-URL download** (NOT a plain anchor) so a 403/404 surfaces a **visible inline error**
(US-M2-3 AC-3) rather than navigating away. The client uses the session-bound route
`GET /api/client/ticket/attachments/{attachmentId}` (no ticketId param, §8); staff uses
`GET /api/staff/tickets/{ticketId}/attachments/{attachmentId}`.

## Impact

- add(ui): a reusable **AttachmentChip** rendered on every thread entry in BOTH the staff detail view AND the client portal thread, sourced from each entry's `attachments` array (§7). **Note:** does not re-create the chip shell; injects `onClick` into A4's AttachmentChip (ROADMAP M2 Decisions §14).
- update(ui): clicking a chip **fetches with credentials**, turns the response into a blob + object-URL to trigger the browser download (§9); a 403/404 renders a visible inline error near the chip instead of navigating.
- update(ui): the client portal uses the **session-bound client route** (§8); the staff detail uses the staff route.

## Regressions

- Threads without attachments render unchanged (US-M1-4 / staff detail).

## Acceptance Tests

> Setup (all): `cargo run -p tools --bin seed -- --reset`; fixtures present; seed/build a demo ticket
> with an `M` `invoice.pdf` and an agent `R` `policy.txt` reply (run the US-M2-1 + US-M2-2 flows, or the
> root demo ticket). Backend on :3701, SPA on :3702.

### AC-1: clicking a chip in the client portal downloads the file. [BROWSER]
- Navigate: http://localhost:3702/tickets → log in with the demo ticket number + its email.
- Action: in the read-only thread, click the `policy.txt` chip.
- Verify: the browser downloads `policy.txt` (object-URL download, §9); bytes match the seeded canned attachment.
- Status: [ ]

### AC-2: clicking a chip in the staff detail view downloads the file. [BROWSER]
- Navigate: log in agent / Agent123! at http://localhost:3702/staff/login; open the demo ticket.
- Action: click an attachment chip in the thread.
- Verify: the file downloads for the authorized staff session.
- Status: [ ]

### AC-3: a client logged into a different ticket cannot reach this ticket's download (visible inline error). [BROWSER]
- Setup: open a SECOND ticket (different email); in a FRESH client session log into the portal for that second ticket only.
- Action: drive a download of the FIRST ticket's attachment id via the session-bound client route in this second session.
- Verify: the fetch returns 404; a **visible inline error** appears near the chip and no file is downloaded (§9, mirrors EC-022.9).
- Status: [ ]

## Test Infrastructure

- Reuses the demo ticket (invoice.pdf on M, policy.txt on R) + a second ticket for the cross-session negative.

## Dependencies

- **TS-M2-A5** (thread payloads expose the `attachments` array, §7) **+ TS-M2-B1** (download routes, §8). The AttachmentChip component itself is introduced by TS-M2-A4 (confirmation page) and reused here.
