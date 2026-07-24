# TS-M3-C2 — add(route): FS-021.10 transfer route + dept change + SLA re-select

- **ID**: TS-M3-C2
- **Type**: Technical Story
- **Parent**: US-M3-C1
- **Labels**: Technical Story, M3
- **Scope**: small

## Context

This technical story supports US-M3-C1 "Agent claims, assigns, transfers a ticket".
Implements the backend route for transferring a ticket to a different department. Transfer
changes the ticket's department, reopens it if closed, re-selects the SLA based on the new
department (precedence per FS-021.13), logs an internal note, records a `transferred` lifecycle
event, and re-checks access (returning the agent to the queue if access is lost).

## Impact

- add(route): `POST /api/staff/tickets/{id}/transfer`
  - Body: `{ "dept_id": 2, "comments": "string >= 5 chars" }`
  - Requires `can_transfer_tickets` permission
  - Validates: department selected, exists, differs from current, comments >= 5 chars
  - On closed ticket: reopens it (BS-021.5)
  - Re-selects SLA: dept SLA > topic SLA > system default (FS-021.13)
  - Logs internal note "Ticket transfered from <old> to <new>" (note: legacy typo preserved)
  - Records `transferred` lifecycle event in ticket_event
  - Returns: `{ "success": true, "message": "Ticket transferred successfully to <dept>", "access_lost": bool }`
  - Error: 403 (no permission), 422 (validation error), 404 (ticket/dept not found)

- update(service): `ticket_service::transfer(ticket_id, new_dept_id, comments, actor_staff)`
  - Changes ticket.dept_id
  - Reopens if closed
  - Calls `select_sla_id(trump=null, dept_id, topic_id)` to re-select SLA
  - Creates internal note thread entry
  - Records lifecycle event

- add(service): `ticket_service::select_sla_id(trump, dept_id, topic_id)` — SLA precedence resolver
  - trump (explicit filter value) > department's SLA > help topic's SLA > system default SLA
  - Returns sla_id or None

- add(service): `ticket_service::check_staff_access(staff, ticket)` — re-check access after transfer
  - Returns true if staff has access to ticket (dept access OR assigned to staff/team)

## Regressions

- The existing ticket detail route must not break when department changes.
- Staff auth and permission checks must remain enforced.

## Acceptance Tests

### AC-1: FS-021.10 — POST /api/staff/tickets/{id}/transfer succeeds with valid dept and >= 5 char comment. [API-ONLY]
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"open","dept_id":1} -> {ticket_id} (Support dept)
- Setup: POST /api/auth/login {"username":"agent","password":"Agent123!"} -> {session_cookie}
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/transfer
- Headers: Cookie: {session_cookie}, Content-Type: application/json
- Body: {"dept_id":2,"comments":"Billing issue"}
- Expect: 200 `{"success":true,"message":"Ticket transferred successfully to Sales","access_lost":false}`
- Verify: SELECT dept_id FROM ticket WHERE ticket_id = {ticket_id} -> 2
- Status: [x]

### AC-2: FS-021.10 — Transfer fails if same department. [API-ONLY]
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"open","dept_id":1} -> {ticket_id}
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/transfer
- Headers: Cookie: {session_cookie}, Content-Type: application/json
- Body: {"dept_id":1,"comments":"Same department"}
- Expect: 422 `{"error":{"message":"Ticket already in the department"}}`
- Status: [x]

### AC-3: FS-021.10 — Transfer fails if department does not exist. [API-ONLY]
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"open","dept_id":1} -> {ticket_id}
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/transfer
- Headers: Cookie: {session_cookie}, Content-Type: application/json
- Body: {"dept_id":999,"comments":"Invalid department"}
- Expect: 422 `{"error":{"message":"Unknown or invalid department"}}`
- Status: [x]

### AC-4: BS-021.8 — Transfer fails if comment < 5 chars. [API-ONLY]
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"open","dept_id":1} -> {ticket_id}
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/transfer
- Headers: Cookie: {session_cookie}, Content-Type: application/json
- Body: {"dept_id":2,"comments":"abc"}
- Expect: 422 `{"error":{"message":"Transfer comments too short!"}}`
- Status: [x]

### AC-5: FS-021.10 — Transferring a closed ticket reopens it. [API-ONLY]
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"closed","dept_id":1} -> {ticket_id}
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/transfer
- Headers: Cookie: {session_cookie}, Content-Type: application/json
- Body: {"dept_id":2,"comments":"Moving to Sales"}
- Expect: 200 `{"success":true}`
- Verify: SELECT status FROM ticket WHERE ticket_id = {ticket_id} -> 'open'
- Status: [x]

### AC-6: FS-021.10 — Transfer logs internal note with transfer message. [API-ONLY]
- Depends: AC-1 (transfer completed)
- Verify: SELECT body FROM ticket_thread WHERE ticket_id = {ticket_id} AND type = 'N' ORDER BY id DESC LIMIT 1 -> contains "Ticket transfered from" and "Support" and "Sales"
- Status: [x]

### AC-7: FS-021.10 — Transfer records a lifecycle event. [API-ONLY]
- Depends: AC-1 (transfer completed)
- Verify: SELECT state FROM ticket_event WHERE ticket_id = {ticket_id} ORDER BY id DESC LIMIT 1 -> 'transferred'
- Status: [x]

### AC-8: FS-021.13 — Transfer re-selects SLA based on new department. [API-ONLY]
- Setup: POST /api/dev/seed-sla {"name":"Standard","hours":24} -> {standard_sla_id}
- Setup: POST /api/dev/seed-sla {"name":"Urgent","hours":4} -> {urgent_sla_id}
- Setup: POST /api/dev/set-dept-sla {"dept_id":1,"sla_id":{standard_sla_id}} (Support)
- Setup: POST /api/dev/set-dept-sla {"dept_id":2,"sla_id":{urgent_sla_id}} (Sales)
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"open","dept_id":1} -> {ticket_id}
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/transfer
- Headers: Cookie: {session_cookie}, Content-Type: application/json
- Body: {"dept_id":2,"comments":"Transfer for SLA test"}
- Expect: 200
- Verify: SELECT sla_id FROM ticket WHERE ticket_id = {ticket_id} -> {urgent_sla_id}
- Status: [x]

### AC-9: BS-021.1 — Transfer returns 403 without can_transfer_tickets. [API-ONLY]
- Setup: POST /api/dev/set-group-perm {"group_id":1,"can_transfer_tickets":false}
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"open","dept_id":1} -> {ticket_id}
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/transfer
- Headers: Cookie: {session_cookie}, Content-Type: application/json
- Body: {"dept_id":2,"comments":"No permission"}
- Expect: 403 `{"error":{"message":"Not allowed to transfer tickets"}}`
- Cleanup: POST /api/dev/set-group-perm {"group_id":1,"can_transfer_tickets":true}
- Status: [x]

### AC-10: FS-021.10 — Transfer returns access_lost=true when agent loses access to new dept. [API-ONLY]
- Setup: POST /api/dev/seed-dept {"name":"External"} -> {external_dept_id}
- Setup: (agent has no access to External dept)
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"open","dept_id":1} -> {ticket_id}
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/transfer
- Headers: Cookie: {session_cookie}, Content-Type: application/json
- Body: {"dept_id":{external_dept_id},"comments":"Transfer to external"}
- Expect: 200 `{"success":true,"access_lost":true,"message":"Ticket transferred successfully to External"}`
- Status: [x]

## Test Infrastructure

### Dev Endpoints Required for Test Setup

| Endpoint | Purpose | Request Shape |
|----------|---------|---------------|
| `POST /api/dev/seed-tickets` | Create test tickets | `{"count":N,"status":"...","dept_id":N}` |
| `POST /api/dev/seed-dept` | Create department | `{"name":"..."}` |
| `POST /api/dev/seed-sla` | Create SLA plan | `{"name":"...","hours":N}` |
| `POST /api/dev/set-dept-sla` | Set department SLA | `{"dept_id":N,"sla_id":N}` |
| `POST /api/dev/set-group-perm` | Toggle group permissions | `{"group_id":N,"can_transfer_tickets":bool}` |
| `POST /api/auth/login` | Get session cookie | `{"username":"...","password":"..."}` |

## Dependencies

- **M1 done**: ticket, staff, thread infrastructure.
- **TS-M3-prep**: two departments (Support, Sales), group-dept access.
- **EPIC-M3-D**: SLA plans seeded; SLA selection precedence logic.
- **TS-M3-C3**: reopen logic (called when transferring a closed ticket).

**Note**: AC-8 (SLA re-selection after transfer) requires EPIC-M3-D SLA infrastructure.
Initial implementation stubs `select_sla_id` to return existing SLA unchanged.
