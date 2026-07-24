# TS-M3-G1 — add(route): FS-021.21 mass_process endpoint + per-action permission

- **ID**: TS-M3-G1
- **Type**: Technical Story
- **Parent**: US-M3-G1
- **Labels**: Technical Story, M3
- **Scope**: small

## Context

Implements the backend bulk action handler per FS-021.21. The endpoint receives an action
(`close`, `reopen`, `delete`) and a list of ticket IDs. It gates on `canManageTickets`, then
re-checks the specific action permission (close needs `can_close_tickets`; reopen needs
`can_close_tickets OR can_create_tickets`; delete needs `can_delete_tickets`).

Each ticket is processed individually with partial success reporting: if 3 of 5 fail their
preconditions (already closed, etc.), the response reports "2 of 5 tickets closed".

## Impact

- add(route): `POST /api/staff/tickets/bulk` with JSON body:
  ```json
  { "action": "close" | "reopen" | "delete", "ticket_ids": [1, 2, 3] }
  ```
- add(service): `ticket_service::bulk_action(staff_id, action, ticket_ids)` — iterates tickets,
  checks per-action permission, applies action, returns `{ succeeded: N, failed: M, message }`.
- update(ticket_service): reuses `close_ticket`, `reopen_ticket`, `delete_ticket` from EPIC-M3-C
  (or adds them if not yet implemented).

## Regressions

- Single-ticket close/reopen/delete routes must continue to work unchanged.

## Acceptance Tests

> Setup (all): `cargo run -p tools --bin seed -- --reset`; backend on :3701; staff login to cookie
> jar `/tmp/qa-staff.jar` (agent with `can_close_tickets=true`, `can_delete_tickets=true`).

### AC-1: FS-021.21 — bulk close succeeds for all open tickets. [API-ONLY]
- Setup: POST /api/dev/seed-tickets → [{status: "open"}, {status: "open"}, {status: "open"}] → {ids: [10, 11, 12]}
- Setup: POST /api/dev/login → {username: "agent", password: "Agent123!"} → cookie jar /tmp/qa-staff.jar
- Request: `curl -s -b /tmp/qa-staff.jar -X POST http://localhost:3701/api/staff/tickets/bulk -H "Content-Type: application/json" -d '{"action":"close","ticket_ids":[10,11,12]}'`
- Expect: HTTP 200, `{ "succeeded": 3, "failed": 0, "message": "3 tickets closed" }`
- Request: `curl -s -b /tmp/qa-staff.jar http://localhost:3701/api/staff/tickets/10`
- Expect: Response contains `"status": "closed"`
- Status: [x]

### AC-2: FS-021.21 — bulk reopen succeeds for closed tickets. [API-ONLY]
- Setup: POST /api/dev/seed-tickets → [{status: "closed"}, {status: "closed"}] → {ids: [10, 11]}
- Request: `curl -s -b /tmp/qa-staff.jar -X POST http://localhost:3701/api/staff/tickets/bulk -H "Content-Type: application/json" -d '{"action":"reopen","ticket_ids":[10,11]}'`
- Expect: HTTP 200, `{ "succeeded": 2, "failed": 0, "message": "2 tickets reopened" }`
- Status: [x]

### AC-3: FS-021.21 — bulk delete removes tickets. [API-ONLY]
- Setup: POST /api/dev/seed-tickets → [{status: "open"}, {status: "open"}] → {ids: [20, 21]}
- Request: `curl -s -b /tmp/qa-staff.jar -X POST http://localhost:3701/api/staff/tickets/bulk -H "Content-Type: application/json" -d '{"action":"delete","ticket_ids":[20,21]}'`
- Expect: HTTP 200, `{ "succeeded": 2, "failed": 0, "message": "2 tickets deleted" }`
- Request: `curl -s -b /tmp/qa-staff.jar http://localhost:3701/api/staff/tickets/20`
- Expect: HTTP 404
- Status: [x]

### AC-4: BS-020.11 — partial success when some tickets are already in target state. [API-ONLY]
- Setup: POST /api/dev/seed-tickets → [{status: "closed"}, {status: "open"}, {status: "open"}] → {ids: [30, 31, 32]}
- Request: `curl -s -b /tmp/qa-staff.jar -X POST http://localhost:3701/api/staff/tickets/bulk -H "Content-Type: application/json" -d '{"action":"close","ticket_ids":[30,31,32]}'`
- Expect: HTTP 200, `{ "succeeded": 2, "failed": 1, "message": "2 of 3 tickets closed" }`
- Status: [x]

### AC-5: EC-020.9 — empty ticket_ids returns error. [API-ONLY]
- Request: `curl -s -b /tmp/qa-staff.jar -X POST http://localhost:3701/api/staff/tickets/bulk -H "Content-Type: application/json" -d '{"action":"close","ticket_ids":[]}'`
- Expect: HTTP 422, body contains `"No tickets selected"`
- Status: [x]

### AC-6: EC-020.8 — staff without close permission is rejected for bulk close. [API-ONLY]
- Setup: POST /api/dev/seed-staff → {username: "agent2", password: "Agent123!", can_close_tickets: false}
- Setup: POST /api/dev/login → {username: "agent2"} → cookie jar /tmp/qa-agent2.jar
- Request: `curl -s -b /tmp/qa-agent2.jar -X POST http://localhost:3701/api/staff/tickets/bulk -H "Content-Type: application/json" -d '{"action":"close","ticket_ids":[10]}'`
- Expect: HTTP 403, body contains `"Permission denied"`
- Status: [x]

### AC-7: EC-020.13 — unknown action is rejected. [API-ONLY]
- Request: `curl -s -b /tmp/qa-staff.jar -X POST http://localhost:3701/api/staff/tickets/bulk -H "Content-Type: application/json" -d '{"action":"bogus","ticket_ids":[10]}'`
- Expect: HTTP 422, body contains `"Unknown action"`
- Status: [x]

### AC-8: BS-020.9 — canManageTickets gate (not admin, not can-close, not can-delete). [API-ONLY]
- Setup: POST /api/dev/seed-staff → {username: "agent3", password: "Agent123!", isadmin: false, can_close_tickets: false, can_delete_tickets: false}
- Setup: POST /api/dev/login → {username: "agent3"} → cookie jar /tmp/qa-agent3.jar
- Request: `curl -s -b /tmp/qa-agent3.jar -X POST http://localhost:3701/api/staff/tickets/bulk -H "Content-Type: application/json" -d '{"action":"close","ticket_ids":[10]}'`
- Expect: HTTP 403, body contains `"Permission denied"`
- Status: [x]

## Test Infrastructure

**Dev Endpoints Required (Setup only):**
- `POST /api/dev/seed-tickets` — create tickets with specified status; returns `{ids: [...]}`
- `POST /api/dev/seed-staff` — create staff with permission flags
- `POST /api/dev/login` — authenticate and return session cookie

## Dependencies

- **EPIC-M3-C**: `close_ticket`, `reopen_ticket`, `delete_ticket` service functions (or we add them here).
- **TS-M3-prep**: seed expansion for group permission flags.
- **TS-M1-A4b**: staff session/auth gates.
