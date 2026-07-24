# TS-M3-A2 — add(service): BS-020.2 visibility scoping (dept + assignment predicate)

- **ID**: TS-M3-A2
- **Type**: Technical Story
- **Parent**: US-M3-A1
- **Labels**: Technical Story, M3
- **Scope**: small

## Context

Implements the core visibility predicate that restricts which tickets a staff member can see
in any queue or search. This is the security boundary ensuring agents only see tickets in their
departments or assigned to them.

Per BS-020.2 and FS-020.4, the visibility predicate is the OR of:
1. Tickets directly assigned to the staff member AND open (`staff_id = me AND status = 'open'`)
2. IF NOT access-limited: tickets in any department the staff member has access to (`dept_id IN (my_depts)`)
3. IF the staff member belongs to teams: tickets assigned to those teams AND open (`team_id IN (my_teams) AND status = 'open'`)

The same predicate is used for both listings and quick stats.

## Impact

- add(service): `visibility::build_staff_visibility_predicate(staff_id, dept_ids, team_ids, is_access_limited)` → SQL WHERE fragment or filter closure.
- add(db): query helpers to fetch a staff member's accessible departments and team memberships.
- update(route): `GET /api/staff/tickets` and `GET /api/staff/tickets/stats` apply the visibility predicate.

## Regressions

- M1's queue route returned all open tickets (no scoping). After this ticket, the agent sees
  only tickets matching visibility rules. Seed must ensure agent has access to Support dept.
- Visibility is always enforced; there is no "see all" mode for non-admin staff.

## Test Infrastructure

- **Backend**: http://localhost:3701 (must be running)
- **Session cookie**: Obtain via `POST /api/staff/login` with staff credentials
- **Dev endpoint needed**: `POST /api/dev/seed-tickets` with dept_id, staff_id, team_id options
- **Dev endpoint needed**: `POST /api/dev/seed-staff` to create test staff accounts with custom visibility

## Acceptance Tests

### AC-1: BS-020.2 — Agent sees tickets in their accessible departments. [API-ONLY]
- Setup: `cargo run -p tools --bin seed -- --reset` (agent's group has access to Support)
- Setup: `curl -X POST http://localhost:3701/api/dev/seed-tickets -H 'Content-Type: application/json' -d '{"tickets":[{"status":"open","dept_id":1}]}'` (Support dept)
- Request: `curl -s -b 'session={{agent_cookie}}' http://localhost:3701/api/staff/tickets?status=open`
- Expect: 200, items array contains the Support ticket
- Status: [x]

### AC-2: BS-020.2 — Agent does NOT see tickets in departments they lack access to. [API-ONLY]
- Setup: remove Sales from agent's group access: `psql -c "DELETE FROM group_dept_access WHERE dept_id=2 AND group_id=(SELECT group_id FROM staff WHERE username='agent')"`
- Setup: `curl -X POST http://localhost:3701/api/dev/seed-tickets -H 'Content-Type: application/json' -d '{"tickets":[{"status":"open","dept_id":2}]}'` (Sales dept, id=2)
- Request: `curl -s -b 'session={{agent_cookie}}' http://localhost:3701/api/staff/tickets?status=open`
- Expect: 200, items array does NOT contain the Sales ticket
- Status: [x]

### AC-3: BS-020.2 — Agent sees tickets assigned to them regardless of department. [API-ONLY]
- Setup: from AC-2 (agent lacks Sales access)
- Setup: `curl -X POST http://localhost:3701/api/dev/seed-tickets -H 'Content-Type: application/json' -d '{"tickets":[{"status":"open","dept_id":2,"staff_id":1}]}'` (Sales, assigned to agent)
- Request: `curl -s -b 'session={{agent_cookie}}' http://localhost:3701/api/staff/tickets?status=open`
- Expect: 200, items array contains the Sales ticket (visible via direct assignment)
- Status: [x]

### AC-4: BS-020.2 — Agent sees tickets assigned to their team regardless of department. [API-ONLY]
- Setup: from AC-2 (agent lacks Sales access, agent is member of team Tier 2)
- Setup: get team_id: `psql -c "SELECT team_id FROM team WHERE name='Tier 2'"` -> e.g., 1
- Setup: `curl -X POST http://localhost:3701/api/dev/seed-tickets -H 'Content-Type: application/json' -d '{"tickets":[{"status":"open","dept_id":2,"team_id":1}]}'`
- Request: `curl -s -b 'session={{agent_cookie}}' http://localhost:3701/api/staff/tickets?status=open`
- Expect: 200, items array contains the Sales ticket (visible via team assignment)
- Status: [x]

### AC-5: BS-020.2 — Access-limited agent sees ONLY assigned tickets, not whole department. [API-ONLY]
- Setup: `curl -X POST http://localhost:3701/api/dev/seed-staff -H 'Content-Type: application/json' -d '{"username":"limited","password":"Limited123!","show_assigned_only":true,"dept_ids":[1]}'`
- Setup: `curl -X POST http://localhost:3701/api/dev/seed-tickets -H 'Content-Type: application/json' -d '{"tickets":[{"status":"open","dept_id":1,"staff_id":null},{"status":"open","dept_id":1,"staff_id":{{limited_staff_id}}}]}'`
- Request: login as limited, `curl -s -b 'session={{limited_cookie}}' http://localhost:3701/api/staff/tickets?status=open`
- Expect: 200, items array contains only the assigned ticket; unassigned Support ticket excluded
- Status: [x]

### AC-6: The visibility predicate is applied to quick stats. [API-ONLY]
- Setup: agent has access to Support only (no Sales)
- Setup: `curl -X POST http://localhost:3701/api/dev/seed-tickets -H 'Content-Type: application/json' -d '{"tickets":[{"status":"open","dept_id":1},{"status":"open","dept_id":1},{"status":"open","dept_id":2}]}'` (2 Support, 1 Sales)
- Request: `curl -s -b 'session={{agent_cookie}}' http://localhost:3701/api/staff/tickets/stats`
- Expect: 200, `"open": 2` (not 3, Sales ticket excluded by visibility)
- Status: [x]

### AC-7: Empty department list contributes no department matches. [API-ONLY]
- Setup: `curl -X POST http://localhost:3701/api/dev/seed-staff -H 'Content-Type: application/json' -d '{"username":"nodept","password":"Nodept123!","dept_ids":[]}'` (no dept access)
- Setup: `curl -X POST http://localhost:3701/api/dev/seed-tickets -H 'Content-Type: application/json' -d '{"tickets":[{"status":"open","dept_id":1},{"status":"open","dept_id":1,"staff_id":{{nodept_staff_id}}}]}'`
- Request: login as nodept, `curl -s -b 'session={{nodept_cookie}}' http://localhost:3701/api/staff/tickets?status=open`
- Expect: 200, items array contains only the ticket assigned to nodept; department-only ticket excluded
- Status: [x]

## Dependencies

- **M1 done**: staff auth, queue route shell.
- **TS-M3-prep**: seed expansion (second department Sales, group-dept access, team "Tier 2").
- **TS-M3-A1**: status param handling (wires this predicate into the query).
