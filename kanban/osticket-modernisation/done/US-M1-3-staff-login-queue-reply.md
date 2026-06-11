# US-M1-3 — Staff – Log in, see the queue, and reply

- **ID**: US-M1-3
- **Type**: User Story
- **Parent**: EPIC-M1-B
- **Labels**: User Story, M1
- **Scope**: medium

## Spec References

- FS-002 (staff login / logout / DB session / one permission gate)
- FS-020 (staff queue — minimal open-tickets list)
- FS-021 (single-ticket view + reply + open/close status)
- BS-021.* (reply appends a response entry; status transitions)

## Context

Epic: EPIC-M1-B — Ticket Round-Trip.
The agent side of the loop: authenticate, find the customer's ticket in the open queue, open
it, read the thread, and reply. Reply reuses the shared ticket core (TS-M1-B1 append entry).

## Description

As a support agent, I can log in, see the list of open tickets, open one to read the
customer's message, and post a reply that becomes part of the ticket thread.

## Impact

- Frontend (staff login page, open-tickets queue, single-ticket view + reply box)
- Backend (staff auth route, queue/list route, ticket-detail route, reply route over the shared core)
- Database (staff/session for auth; ticket/ticket_thread for queue + reply)
- Browser (desktop)

## Business Rules

- BS-002: only an authenticated staff account with the seeded permission can access the staff panel and reply.
- BS-020: the queue lists Open tickets with subject + ticket number (no scoping tabs/search in M1).
- BS-021: a reply appends a response entry to the ticket thread, authored by the agent, in chronological order.
- BS-021: posting a reply keeps the ticket Open (close/reopen toggle available but minimal in M1).

## Regressions

- Reply path shares the ticket core (TS-M1-B1) with US-M1-2; verify ticket creation still works after reply wiring.

## Acceptance Criteria

### AC-1: The staff login page accepts the seeded credentials and lands on the staff control panel; wrong credentials are rejected. [BROWSER]
- Setup: reseed dev DB — `cargo run -p tools --bin seed` (ensures the `agent` account exists).
- Navigate: http://localhost:3702/staff/login
- Action: type "agent" in the Username field and "wrongpass" in the Password field, submit.
- Verify: login is rejected with an error message; the URL stays on /staff/login (no session).
- Action: type "agent" / "Agent123!", submit.
- Verify: the browser lands on the staff control panel (e.g. /staff/tickets) showing the agent is logged in.
- Status: [x]

### AC-2: The staff panel shows an Open-tickets queue listing the ticket created in US-M1-2 (subject + number visible). [BROWSER]
- Setup: ensure at least one open ticket exists — either carry over the ticket from US-M1-2 AC-3, or seed one via `curl -s -X POST http://localhost:3701/api/dev/seed-ticket` → {ticketNumber, email}.
- Navigate: http://localhost:3702/staff/tickets (logged in as agent from AC-1).
- Verify: the Open-tickets queue lists the ticket with its subject ("Printer broken") and its 6-digit number, newest first (created DESC).
- Status: [x]

### AC-3: Clicking a ticket opens its detail view showing the customer's original message in the thread. [BROWSER]
- Navigate: http://localhost:3702/staff/tickets (logged in).
- Action: click the ticket row for the US-M1-2 ticket.
- Verify: the detail view opens and the thread shows the customer's original message (the `M` entry, "My printer won't print.") in chronological order (oldest first).
- Status: [x]

### AC-4: Typing a reply and posting it adds the reply to the thread, visible immediately. [BROWSER]
- Navigate: the ticket detail view from AC-3.
- Action: type "We are looking into your printer issue." in the reply box and post it.
- Verify: the thread refetches and the new `R` response appears below the customer message, authored by the agent. Ticket stays Open (no status checkboxes in M1).
- Status: [x]

### AC-5: An unauthenticated request to a staff route is rejected (401/redirect). [API-ONLY]
- Request: `curl -s -o /dev/null -w "%{http_code}" http://localhost:3701/api/staff/tickets?status=open` (no session cookie).
- Expect: 401 with the shared error envelope; no ticket data returned.
- Verify (browser variant): navigating to http://localhost:3702/staff/tickets with no staff session redirects to /staff/login (apiClient 401 handling).
- Status: [x]

## Checklist (children)

- [ ] TS-M1-C1 — Staff auth routes (login / logout / session) + permission gate
- [ ] TS-M1-C2 — Open-tickets queue route + ticket-detail route
- [ ] TS-M1-C3 — Staff reply route (over shared core)
- [ ] TS-M1-C4 — Staff UI: login page, queue list, ticket view + reply box

## Test Infrastructure

- Uses the seeded staff credentials `agent` / `Agent123!` (TS-M1-A3). Document them in CLAUDE.md Quick Start.
- The ticket from US-M1-2 (or a seeded dev ticket) is the queue fixture for browser runs.
- Dev endpoints consumed (owned/documented elsewhere): the reseed command
  `cargo run -p tools --bin seed` (TS-M1-A3) for a known DB state, and **`POST /api/dev/seed-ticket`**
  (TS-M1-D1, env-gated) to inject a queue fixture without depending on the US-M1-2 flow.

## Dependencies

- EPIC-M1-A (auth/session infra), US-M1-2 / TS-M1-B1 (shared core + a ticket to display).
