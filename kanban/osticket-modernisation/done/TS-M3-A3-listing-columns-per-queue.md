# TS-M3-A3 — update(route): BS-020.5 listing columns per queue (rightmost column logic)

- **ID**: TS-M3-A3
- **Type**: Technical Story
- **Parent**: US-M3-A1
- **Labels**: Technical Story, M3
- **Scope**: small

## Context

Per BS-020.5 and FS-020.3, the ticket listing table has a variable rightmost column depending
on the queue:
- **Assigned To**: shown when assigned tickets are visible (system or staff override enables it)
- **Closed By**: shown on the Closed queue
- **Department**: shown otherwise (when assigned display is off)

This ticket updates the queue response to include the appropriate field and a metadata flag
indicating which column variant the frontend should render.

## Impact

- update(route): `GET /api/staff/tickets` response includes:
  - **Response shape**: `{ items: QueueItem[], rightmost_column: string, total: number }`
  - `rightmost_column: "assigned_to" | "closed_by" | "department"` (metadata for frontend)
  - Each ticket row includes the relevant field (`assigned_to_name`, `closed_by_name`, or
    `department_name`) populated.
- update(service): fetch assigned staff/team name, closing staff name, or department name as
  needed per queue.
- add(db): join to staff/team/department tables as needed.
- **Note**: The `closed_by_name` field requires the `ticket.closed_by_staff_id` column added
  by TS-M3-schema.

## Regressions

- M1 queue response may already include department; ensure backward compatibility or coordinate
  frontend update.
- The "Assigned To" column shows staff name or team name (whichever is assigned); if both, show
  the staff name.

## Test Infrastructure

- **Backend**: http://localhost:3701 (must be running)
- **Session cookie**: Obtain via `POST /api/staff/login` with `{"username":"agent","password":"Agent123!"}`
- **Dev endpoint needed**: `POST /api/dev/seed-tickets` with staff_id, team_id, closed_by_staff_id options

## Acceptance Tests

### AC-1: BS-020.5 — Open queue with show_assigned_tickets=1 returns rightmost_column=assigned_to. [API-ONLY]
- Setup: `psql -U postgres -d osticket_dev -c "UPDATE config SET value='1' WHERE key='show_assigned_tickets'"`
- Setup: `curl -X POST http://localhost:3701/api/dev/seed-tickets -H 'Content-Type: application/json' -d '{"tickets":[{"status":"open","staff_id":1}]}'`
- Request: `curl -s -b 'session={{cookie}}' http://localhost:3701/api/staff/tickets?status=open`
- Expect: 200, response contains `"rightmost_column":"assigned_to"` and each item has `assigned_to_name` field
- Status: [x]

### AC-2: BS-020.5 — Open queue with show_assigned_tickets=0 returns rightmost_column=department. [API-ONLY]
- Setup: `psql -U postgres -d osticket_dev -c "UPDATE config SET value='0' WHERE key='show_assigned_tickets'"`
- Request: `curl -s -b 'session={{cookie}}' http://localhost:3701/api/staff/tickets?status=open`
- Expect: 200, response contains `"rightmost_column":"department"` and each item has `department_name` field
- Status: [x]

### AC-3: BS-020.5 — Closed queue returns rightmost_column=closed_by. [API-ONLY]
- Setup: `curl -X POST http://localhost:3701/api/dev/seed-tickets -H 'Content-Type: application/json' -d '{"tickets":[{"status":"closed","closed_by_staff_id":1}]}'`
- Request: `curl -s -b 'session={{cookie}}' http://localhost:3701/api/staff/tickets?status=closed`
- Expect: 200, response contains `"rightmost_column":"closed_by"` and each item has `closed_by_name` field (e.g., "agent")
- Status: [x]

### AC-4: BS-020.5 — Answered queue returns rightmost_column=assigned_to. [API-ONLY]
- Setup: `curl -X POST http://localhost:3701/api/dev/seed-tickets -H 'Content-Type: application/json' -d '{"tickets":[{"status":"open","isanswered":true}]}'`
- Request: `curl -s -b 'session={{cookie}}' http://localhost:3701/api/staff/tickets?status=answered`
- Expect: 200, response contains `"rightmost_column":"assigned_to"`
- Status: [x]

### AC-5: BS-020.5 — Overdue queue returns rightmost_column=assigned_to. [API-ONLY]
- Setup: `curl -X POST http://localhost:3701/api/dev/seed-tickets -H 'Content-Type: application/json' -d '{"tickets":[{"status":"open","isoverdue":true}]}'`
- Request: `curl -s -b 'session={{cookie}}' http://localhost:3701/api/staff/tickets?status=overdue`
- Expect: 200, response contains `"rightmost_column":"assigned_to"`
- Status: [x]

### AC-6: BS-020.5 — Assigned (My Tickets) queue returns rightmost_column=department. [API-ONLY]
- Setup: `curl -X POST http://localhost:3701/api/dev/seed-tickets -H 'Content-Type: application/json' -d '{"tickets":[{"status":"open","staff_id":1}]}'`
- Request: `curl -s -b 'session={{cookie}}' http://localhost:3701/api/staff/tickets?status=assigned`
- Expect: 200, response contains `"rightmost_column":"department"` (Assigned To hidden when it is always "me")
- Status: [x]

### AC-7: Assigned-to shows staff name when assigned to staff. [API-ONLY]
- Setup: `psql -c "UPDATE config SET value='1' WHERE key='show_assigned_tickets'"` and seed a ticket assigned to agent (staff_id=1)
- Request: `curl -s -b 'session={{cookie}}' http://localhost:3701/api/staff/tickets?status=open`
- Expect: 200, item has `"assigned_to_name":"agent"` (or staff's full name)
- Status: [x]

### AC-8: Assigned-to shows team name when assigned to team only. [API-ONLY]
- Setup: get team_id for "Tier 2", then `curl -X POST http://localhost:3701/api/dev/seed-tickets -H 'Content-Type: application/json' -d '{"tickets":[{"status":"open","team_id":1,"staff_id":null}]}'`
- Request: `curl -s -b 'session={{cookie}}' http://localhost:3701/api/staff/tickets?status=open`
- Expect: 200, item has `"assigned_to_name":"Tier 2"` (team name when no individual staff assigned)
- Status: [x]

## Dependencies

- **M1 done**: queue route exists.
- **TS-M3-schema**: migration adds `ticket.closed_by_staff_id` column (required for Closed By display).
- **TS-M3-A1**: status param handling.
- **TS-M3-prep**: seed expansion (team, config keys).
