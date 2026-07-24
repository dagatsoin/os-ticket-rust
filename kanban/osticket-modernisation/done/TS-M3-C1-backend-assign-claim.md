# TS-M3-C1 — add(route): BS-021.8 assign/claim routes + validation + internal notes

- **ID**: TS-M3-C1
- **Type**: Technical Story
- **Parent**: US-M3-C1
- **Labels**: Technical Story, M3
- **Scope**: small

## Context

This technical story supports US-M3-C1 "Agent claims, assigns, transfers a ticket".
Implements the backend routes for assigning a ticket to a staff/team and claiming (self-assign).
Assignment logs an internal note, triggers assignment alerts (M4), and on a closed ticket
triggers a reopen. Release (unassign) requires department-manager status and is deferred to M4.

## Impact

- add(route): `POST /api/staff/tickets/{id}/assign` — assigns to staff (`s{id}`) or team (`t{id}`)
  - Body: `{ "assignee": "s123" | "t456", "comments": "string >= 5 chars" }`
  - Requires `can_assign_tickets` permission
  - Validates: assignee selected, not already assigned to same, comments >= 5 chars
  - On closed ticket: reopens it (FS-021.7)
  - Logs internal note "Ticket assigned to <name> by <actor>" or default text
  - Returns: `{ "success": true, "message": "..." }`, 200 on success
  - Error: 403 (no permission), 422 (validation error), 404 (ticket not found)

- add(route): `POST /api/staff/tickets/{id}/claim` — self-assign
  - No body required (comments default to "Ticket claimed by <name>")
  - Requires `can_assign_tickets` permission
  - Validates: ticket is open, not already assigned (EC-021.7)
  - Logs internal note "Ticket claimed by <name>"
  - Returns: `{ "success": true, "message": "Ticket is now assigned to you!" }`, 200
  - Error: 403, 422 (already assigned / not open), 404

- add(service): `ticket_service::assign(ticket_id, staff_id, assignee_type, assignee_id, comments, actor_staff)`
  - Sets `staff_id` for staff assignment, `team_id` for team assignment
  - Reopens closed ticket (calls reopen logic from TS-M3-C3)
  - Creates internal note thread entry
  - Records `assigned` lifecycle event (optional, deferred to M4 with alerts)

- add(service): `ticket_service::claim(ticket_id, staff_id)`
  - Validates: open + unassigned
  - Delegates to assign with self as assignee and default comment

## Regressions

- The existing ticket detail route must not break when assignment fields are populated.
- Staff auth and permission checks must remain enforced.

## Acceptance Tests

### AC-1: BS-021.8 — POST /api/staff/tickets/{id}/claim succeeds for open unassigned ticket. [API-ONLY]
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"open","assigned":false} -> {ticket_id}
- Setup: POST /api/auth/login {"username":"agent","password":"Agent123!"} -> {session_cookie}
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/claim
- Headers: Cookie: {session_cookie}
- Expect: 200 `{"success":true,"message":"Ticket is now assigned to you!"}`
- Verify: SELECT staff_id FROM ticket WHERE ticket_id = {ticket_id} -> equals agent's staff_id
- Status: [x]

### AC-2: FS-021.8 — Claim fails if ticket is already assigned. [API-ONLY]
- Depends: AC-1 (ticket now assigned to agent)
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/claim
- Headers: Cookie: {session_cookie}
- Expect: 422 `{"error":{"message":"Ticket already assigned"}}`
- Status: [x]

### AC-3: FS-021.8 — Claim fails if ticket is closed. [API-ONLY]
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"closed"} -> {ticket_id}
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/claim
- Headers: Cookie: {session_cookie}
- Expect: 422 `{"error":{"message":"Only open tickets can be claimed"}}`
- Status: [x]

### AC-4: BS-021.8 — POST /api/staff/tickets/{id}/assign succeeds with valid staff assignee and >= 5 char comment. [API-ONLY]
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"open","assigned":false} -> {ticket_id}
- Setup: POST /api/dev/seed-staff {"name":"Bob Agent"} -> {bob_staff_id}
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/assign
- Headers: Cookie: {session_cookie}, Content-Type: application/json
- Body: {"assignee":"s{bob_staff_id}","comments":"Routing to specialist"}
- Expect: 200 `{"success":true,"message":"Ticket assigned successfully"}`
- Verify: SELECT staff_id FROM ticket WHERE ticket_id = {ticket_id} -> {bob_staff_id}
- Status: [x]

### AC-5: BS-021.8 — POST /api/staff/tickets/{id}/assign succeeds with team assignee. [API-ONLY]
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"open","assigned":false} -> {ticket_id}
- Setup: (TS-M3-prep provides team_id=1 "Tier 2")
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/assign
- Headers: Cookie: {session_cookie}, Content-Type: application/json
- Body: {"assignee":"t1","comments":"Escalating to Tier 2"}
- Expect: 200 `{"success":true}`
- Verify: SELECT team_id FROM ticket WHERE ticket_id = {ticket_id} -> 1
- Status: [x]

### AC-6: BS-021.8 — Assign fails if comment < 5 chars. [API-ONLY]
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"open","assigned":false} -> {ticket_id}
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/assign
- Headers: Cookie: {session_cookie}, Content-Type: application/json
- Body: {"assignee":"s1","comments":"Hi"}
- Expect: 422 `{"error":{"message":"Comment too short","fields":{"comments":"min 5 chars"}}}`
- Status: [x]

### AC-7: FS-021.7 — Assign fails if same assignee already set. [API-ONLY]
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"open","assigned":"s1"} -> {ticket_id}
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/assign
- Headers: Cookie: {session_cookie}, Content-Type: application/json
- Body: {"assignee":"s1","comments":"Duplicate assign"}
- Expect: 422 `{"error":{"message":"Ticket already assigned to the staff."}}`
- Status: [x]

### AC-8: FS-021.7 — Assigning a closed ticket reopens it. [API-ONLY]
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"closed"} -> {ticket_id}
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/assign
- Headers: Cookie: {session_cookie}, Content-Type: application/json
- Body: {"assignee":"s1","comments":"Taking over this ticket"}
- Expect: 200 `{"success":true}`
- Verify: SELECT status FROM ticket WHERE ticket_id = {ticket_id} -> 'open'
- Status: [x]

### AC-9: BS-021.1 — Assign/claim returns 403 without can_assign_tickets. [API-ONLY]
- Setup: POST /api/dev/set-group-perm {"group_id":1,"can_assign_tickets":false}
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"open","assigned":false} -> {ticket_id}
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/claim
- Headers: Cookie: {session_cookie}
- Expect: 403 `{"error":{"message":"Not allowed to assign tickets"}}`
- Cleanup: POST /api/dev/set-group-perm {"group_id":1,"can_assign_tickets":true}
- Status: [x]

### AC-10: FS-021.7 — Claim logs internal note "Ticket claimed by <name>". [API-ONLY]
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"open","assigned":false} -> {ticket_id}
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/claim
- Headers: Cookie: {session_cookie}
- Expect: 200
- Verify: SELECT body FROM ticket_thread WHERE ticket_id = {ticket_id} AND type = 'N' ORDER BY id DESC LIMIT 1 -> contains "Ticket claimed by" and "agent"
- Status: [x]

## Test Infrastructure

### Dev Endpoints Required for Test Setup

| Endpoint | Purpose | Request Shape |
|----------|---------|---------------|
| `POST /api/dev/seed-tickets` | Create test tickets | `{"count":N,"status":"...","assigned":bool\|"s{id}"}` |
| `POST /api/dev/seed-staff` | Create additional staff | `{"name":"..."}` |
| `POST /api/dev/set-group-perm` | Toggle group permissions | `{"group_id":N,"can_assign_tickets":bool}` |
| `POST /api/auth/login` | Get session cookie | `{"username":"...","password":"..."}` |

## Dependencies

- **M1 done**: ticket, staff, thread infrastructure.
- **TS-M3-prep**: team seed, permission flags.
- **TS-M3-C3**: reopen logic (called when assigning a closed ticket).
