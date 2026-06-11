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

- add(ui): a reusable **AttachmentChip** rendered on every thread entry in BOTH the staff detail view AND the client portal thread, sourced from each entry's `attachments` array (§7).
- update(ui): clicking a chip **fetches with credentials**, turns the response into a blob + object-URL to trigger the browser download (§9); a 403/404 renders a visible inline error near the chip instead of navigating.
- update(ui): the client portal uses the **session-bound client route** (§8); the staff detail uses the staff route.

## Regressions

- Threads without attachments render unchanged (US-M1-4 / staff detail).

## Acceptance Tests

### AC-1: clicking a chip in the client portal downloads the file. [BROWSER]
- Client portal thread → click `policy.txt` chip → browser downloads `policy.txt`, bytes match.
- Status: [ ]

### AC-2: clicking a chip in the staff detail view downloads the file. [BROWSER]
- Staff detail thread → click a chip → file downloads for the staff session.
- Status: [ ]

### AC-3: a client logged into a different ticket cannot reach this ticket's download (visible inline error). [BROWSER]
- In a second-ticket client session, attempt the first ticket's download → the fetch returns 404; a **visible inline error** appears near the chip and no file is downloaded (§9, mirrors EC-022.9).
- Status: [ ]

## Test Infrastructure

- Reuses the demo ticket (invoice.pdf on M, policy.txt on R) + a second ticket for the cross-session negative.

## Dependencies

- **TS-M2-A5** (thread payloads expose the `attachments` array, §7) **+ TS-M2-B1** (download routes, §8). The AttachmentChip component itself is introduced by TS-M2-A4 (confirmation page) and reused here.
