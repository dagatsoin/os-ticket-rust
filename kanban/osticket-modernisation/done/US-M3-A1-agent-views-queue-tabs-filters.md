# US-M3-A1 — Agent views queue tabs with counts and filters by status

- **ID**: US-M3-A1
- **Type**: User Story
- **Parent**: EPIC-M3-A
- **Labels**: User Story, M3
- **Scope**: medium

## Spec References

- FS-020.1 (queue entry, default landing → Open queue)
- FS-020.2 (predefined queues / status tabs: Open, Answered, My Tickets, Overdue, Closed)
- FS-020.3 (ticket listing table & columns)
- FS-020.4 (visibility scoping: department + assignment)
- FS-020.11 (quick ticket stats for tab counts)
- BS-020.1 (status param is overloaded: queue selector vs real status)
- BS-020.2 (department + assignment visibility is always enforced)
- BS-020.4 (Open queue answered/assigned visibility toggles)
- BS-020.5 (rightmost column varies by queue)

## Context

Epic: EPIC-M3-A — Queue Tabs + Visibility.
M1 delivered a single "open tickets" list. This story adds the full tab structure with counts,
visibility scoping by department/assignment, and the proper column layout per queue.

## Description

As a support agent, I can see queue tabs (Open, Answered, My Tickets, Overdue, Closed) with
ticket counts, click a tab to filter the listing to that queue, and only see tickets I am
authorized to view (my departments or tickets assigned to me).

## Impact

- Frontend (queue tabs UI, tab-based routing, count badges)
- Backend (status param handling, visibility scoping, quick-stats endpoint, column selection)
- Database (department access, staff assignments, ticket status/flags)
- Browser (desktop)

## Business Rules

- BS-020.1: The `status` parameter selects a queue (open/answered/assigned/overdue/closed) distinct
  from the real ticket status filter applied.
- BS-020.2: Every listing is visibility-scoped: assigned-to-me-open OR my-team-open OR my-department
  tickets (unless access-limited).
- BS-020.4: On the Open queue, answered tickets are hidden unless `show_answered_tickets=1`;
  assigned tickets are hidden unless `show_assigned_tickets=1`.
- BS-020.5: Rightmost column is "Assigned To" when assigned tickets are shown, "Closed By" on
  Closed queue, "Department" otherwise.
- BS-020.14: Warning banner when >10 assigned tickets; notice when >10 overdue tickets.

## Regressions

- The M1 open-tickets queue route must continue to work (default to Open queue when no status).
- Staff auth must still be enforced on queue access.

## Acceptance Criteria

### AC-1: The queue tabs render with counts (My Tickets appears only once the agent has an assignment); clicking a tab filters the listing. [BROWSER]
> Per FS-020.2 / BS-020.14 (legacy scp/tickets.php:536-544 `if($stats['assigned'])`): with the standard
> queue config (`show_answered_tickets=0`), the Open / Overdue / Closed tabs always render with count badges;
> the "Answered" tab is hidden while `show_answered_tickets=0`; and the "My Tickets" (status=assigned) tab
> renders ONLY when the logged-in agent currently has ≥1 ticket assigned to them — it is NOT gated on
> `show_assigned_tickets` and shows no count-0 state. It therefore appears only after the agent claims a ticket.
- Setup: `cargo run -p tools --bin seed -- --reset`
- Setup: `curl -X POST http://localhost:3701/api/dev/seed-tickets -H 'Content-Type: application/json' -d '{"tickets":[{"status":"open"},{"status":"open"},{"status":"open"},{"status":"open"},{"status":"open","isoverdue":true},{"status":"closed"},{"status":"closed"}]}'` (5 open — one overdue — and 2 closed; none assigned to agent)
- Navigate: http://localhost:3702/staff/login
- Action: Type "agent" in username field
- Action: Type "Agent123!" in password field
- Action: Click Login button
- Navigate: http://localhost:3702/staff/tickets
- Verify: Open tab visible with a count badge ("5")
- Verify: Overdue tab visible with a count badge ("1")
- Verify: Closed tab visible with a count badge ("2")
- Verify: "Answered" tab is NOT rendered (hidden while `show_answered_tickets=0`)
- Verify: "My Tickets" tab is NOT rendered initially (agent has zero assigned tickets)
- Action: Open an unassigned open ticket from the listing and click "Claim Ticket"
- Navigate: http://localhost:3702/staff/tickets
- Verify: "My Tickets" tab is now rendered with a count badge ("1")
- Action: Click the "Closed" tab
- Verify: URL contains `?status=closed`
- Verify: Ticket listing shows exactly 2 closed tickets
- Verify: Closed tab has active/highlighted styling
- Status: [x]

### AC-2: Default landing with no status param shows the Open queue. [BROWSER]
- Setup: logged in as agent, seed includes open tickets
- Navigate: http://localhost:3702/staff/tickets
- Verify: URL has no `?status=` param
- Verify: Open tab has active/highlighted styling
- Verify: Listing shows open tickets only (answered tickets hidden when show_answered_tickets=0)
- Status: [x]

### AC-3: My Tickets queue shows only tickets assigned to me. [BROWSER]
- Setup: `curl -X POST http://localhost:3701/api/dev/seed-tickets -H 'Content-Type: application/json' -d '{"tickets":[{"status":"open","staff_id":1},{"status":"open","staff_id":null}]}'`
- Navigate: http://localhost:3702/staff/tickets?status=assigned
- Verify: My Tickets tab has active/highlighted styling
- Verify: Listing shows only the ticket assigned to agent (staff_id=1)
- Verify: "Assigned To" column is NOT displayed (it would always show "me")
- Status: [x]

### AC-4: Visibility scoping — agent sees only tickets in their departments or assigned to them. [BROWSER]
- Setup: Create External dept without agent access: `psql -c "INSERT INTO department (name) VALUES ('External') ON CONFLICT DO NOTHING"`
- Setup: `curl -X POST http://localhost:3701/api/dev/seed-tickets -H 'Content-Type: application/json' -d '{"tickets":[{"status":"open","dept_id":1},{"status":"open","dept_id":3,"staff_id":null}]}'` (Support=1, External=3)
- Navigate: http://localhost:3702/staff/tickets?status=open
- Verify: Support ticket appears in listing
- Verify: External ticket does NOT appear (no dept access, not assigned)
- Setup: Assign External ticket to agent: `curl -X POST http://localhost:3701/api/dev/assign-ticket -H 'Content-Type: application/json' -d '{"ticket_id":{{external_ticket_id}},"staff_id":1}'`
- Action: Refresh page (F5)
- Verify: External ticket now appears in listing (visible via assignment)
- Status: [x]

### AC-5: Rightmost column varies by queue (Assigned To / Closed By / Department). [BROWSER]
- Setup: `psql -U postgres -d osticket_dev -c "UPDATE config SET value='1' WHERE key='show_assigned_tickets'"`
- Navigate: http://localhost:3702/staff/tickets?status=open
- Verify: Rightmost table column header is "Assigned To"
- Navigate: http://localhost:3702/staff/tickets?status=closed
- Verify: Rightmost table column header is "Closed By"
- Setup: `psql -U postgres -d osticket_dev -c "UPDATE config SET value='0' WHERE key='show_assigned_tickets'"`
- Navigate: http://localhost:3702/staff/tickets?status=open
- Action: Refresh page if needed
- Verify: Rightmost table column header is "Department"
- Status: [x]

### AC-6: Quick-stats endpoint returns correct counts for each queue. [API-ONLY]
- Setup: `cargo run -p tools --bin seed -- --reset`
- Setup: `curl -X POST http://localhost:3701/api/dev/seed-tickets -H 'Content-Type: application/json' -d '{"tickets":[{"status":"open"},{"status":"open"},{"status":"open","isanswered":true},{"status":"open","isoverdue":true},{"status":"open","staff_id":1},{"status":"closed"},{"status":"closed"}]}'`
- Setup: Login: `curl -c cookies.txt -X POST http://localhost:3701/api/staff/login -H 'Content-Type: application/json' -d '{"username":"agent","password":"Agent123!"}'`
- Request: `curl -s -b cookies.txt http://localhost:3701/api/staff/tickets/stats`
- Expect: 200, JSON with `{"open":2,"answered":1,"overdue":1,"assigned":1,"closed":2}` (counts reflect visibility scope)
- Status: [x]

## Checklist (children)

- [ ] TS-M3-A1 — Backend: status param handling + quick-stats endpoint
- [ ] TS-M3-A2 — Backend: visibility scoping (dept + assignment predicate)
- [ ] TS-M3-A3 — Backend: listing columns per queue (rightmost column logic)
- [ ] TS-M3-A4 — Frontend: queue tabs UI with counts + status routing

## Test Infrastructure

- **Frontend**: http://localhost:3702 (Vite dev server must be running)
- **Backend**: http://localhost:3701 (must be running)
- **Credentials**: `agent` / `Agent123!` (seeded by TS-M1-A3)
- **Prerequisite**: TS-M3-prep seed expansion (second department Sales, group-dept access, teams, config keys)

### Dev Endpoints Needed (for test setup, NOT validation)

| Endpoint | Purpose | Request Shape |
|----------|---------|---------------|
| `POST /api/dev/seed-tickets` | Create test tickets with configurable status/flags | `{"tickets":[{"status":"open","isanswered":false,"isoverdue":false,"staff_id":null,"team_id":null,"dept_id":1}]}` |
| `POST /api/dev/assign-ticket` | Assign a ticket to staff (minimal helper for testing) | `{"ticket_id":1,"staff_id":1}` |
| `POST /api/dev/reset` | Reset DB to seed state (alternative to re-running seed binary) | `{}` |

## Dependencies

- **M1 done**: provides the baseline queue route and staff auth.
- **TS-M3-prep**: seed expansion (second department, group-dept access, teams, config keys).

## Review feedback

Returned from M3 root E2E (pre-merge review failure). AC-1 reset to `[ ]`.

**CORRECTION (spec review vs. frozen legacy — supersedes the earlier note below).** The original
feedback was WRONG: it claimed the "My Tickets" tab must always render from `show_assigned_tickets=1`
with a possible count of 0. That is not the specified behaviour, and the implementation is CORRECT.

- **Verified rule (FS-020.2, BS-020.14; legacy `scp/tickets.php:536-544` `if($stats['assigned'])`):**
  the "My Tickets" (status=assigned) tab renders ONLY when the logged-in agent currently has ≥1
  ticket assigned to them. It is NOT gated on `show_assigned_tickets` and it never shows a count-0
  state. It therefore appears only after the agent has an assignment (e.g. after claiming a ticket),
  and is absent when the agent has zero assignments. The observed behaviour ("MY TICKETS 1" only
  materialising after the AC-4 claim; only Open/Overdue/Closed at empty-assignment state) is CORRECT.
- **Action for re-test:** AC-1 has been reworded to assert this rule (Open/Overdue/Closed always render
  with counts; "Answered" hidden while `show_answered_tickets=0`; "My Tickets" absent initially, then
  present with its count after a claim). No implementation change is required for this AC; re-run the
  browser test against the reworded AC-1. AC-1 Status remains `[ ]` (untested).

<details><summary>Superseded (INCORRECT) original feedback — kept for audit</summary>

> - **Root AC-1 (US-M3-A1) FAIL** — "My Tickets" queue tab does not render when the agent has
>   0 assigned tickets even though `show_assigned_tickets=1`; ... Correct behaviour under spec-review:
>   the My Tickets tab must always render from `show_assigned_tickets=1` regardless of assignment count.
>
> This rationale was incorrect — see the corrected rule above.

</details>
