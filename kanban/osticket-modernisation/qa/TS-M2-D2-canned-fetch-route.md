# TS-M2-D2 — add(route): FS-022.14 canned-response fetch (substituted body + attachments)

- **ID**: TS-M2-D2
- **Type**: Technical Story
- **Parent**: US-M2-2
- **Labels**: Technical Story, M2
- **Scope**: small

## Context

The route the reply-box dropdown calls: list the offerable canned responses for a ticket, and fetch
one with its body **variable-substituted** (Epic C) for that ticket plus its attachment list.

## Impact

- add(route): `GET /api/staff/tickets/{id}/canned` — lists **enabled** responses scoped to the ticket's department (or dept 0/all) (BS-022.1, BS-022.2): `id`, `title`.
- add(route): `GET /api/staff/tickets/{id}/canned/{cannedId}` — returns the body run through the substitution engine for this ticket (FS-022.14) + the attachment list under the shared key **`attachments: [{id, name, size, mime}]`** (the SAME shape as thread-entry attachments §7).
- A disabled or out-of-scope canned id returns 404 (BS-022.2).

## Regressions

- New read routes; no write path touched.

## Acceptance Tests

> Setup (all): `cargo run -p tools --bin seed -- --reset`; backend on :3701; seed a ticket and capture
> its `{id}`; staff login to a cookie jar — `curl -c /tmp/qa-staff.jar -X POST .../api/staff/login`
> (`agent`/`Agent123!`). Note the canned ids from the list route.

### AC-1: BS-022.1/.2 — the list returns only enabled, dept-scoped responses. [API-ONLY]
- Request: `curl -s -b /tmp/qa-staff.jar http://localhost:3701/api/staff/tickets/{id}/canned`.
- Verify: the list includes "Acknowledge receipt" (enabled, dept 0); it does NOT include "Closed — disabled sample".
- Status: [x]

### AC-2: FS-022.14 — fetching a response returns its body with %{ticket.number} substituted. [API-ONLY]
- Request: `curl -s -b /tmp/qa-staff.jar http://localhost:3701/api/staff/tickets/{id}/canned/{cannedId}` (the "Acknowledge receipt" id).
- Verify: the returned `body` contains the ticket's own number and no literal `%{...}`.
- Status: [x]

### AC-3: FS-022.14 — the response includes its attachment list under the shared `attachments` key. [API-ONLY]
- Request: same detail call as AC-2.
- Verify: the payload's `attachments` (§7) lists `policy.txt` with `id/name/size/mime` — the same shape as thread-entry attachments (A5).
- Status: [x]

### AC-4: BS-022.2 — fetching a disabled canned id returns 404. [API-ONLY]
- Request: `curl -i -s -b /tmp/qa-staff.jar http://localhost:3701/api/staff/tickets/{id}/canned/{disabledCannedId}` (the "Closed — disabled sample" id from a direct DB lookup).
- Verify: HTTP 404 (out-of-scope/disabled canned id is not fetchable).
- Status: [x]

## Dependencies

- TS-M2-D1 (schema + seed), TS-M2-C1 (substitution engine), TS-M1-C1 (staff session).
