# US-M1-4 — Portal – Client views the staff reply

- **ID**: US-M1-4
- **Type**: User Story
- **Parent**: EPIC-M1-B
- **Labels**: User Story, M1
- **Scope**: small

## Spec References

- FS-010 (client portal lookup / view / reply)
- BS-010.* (ticket# + email login, client session, thread visibility)
- FS-091 (ticket / thread tables)

## Context

Epic: EPIC-M1-B — Ticket Round-Trip.
Closes the loop: the customer returns, authenticates with their ticket number + email, and
reads the agent's reply in the thread. Client reply-back is part of FS-010 but only the
**view** path is required to complete the M1 round-trip; client reply is a stretch within scope.

## Description

As a customer, I can log into the client portal with my ticket number and email and view my
ticket thread, including the agent's reply.

## Impact

- Frontend (client portal login + ticket-thread view)
- Backend (client-realm auth route, client ticket-thread route)
- Database (ticket/ticket_thread; session for client realm)
- Browser (desktop)

## Business Rules

- BS-010: a client authenticates with the correct ticket number + matching email; mismatches are rejected.
- BS-010: a client session can only view its own ticket's thread.
- BS-010: the thread shows the customer's original message and the agent's response in order.
- (Out of M1 scope: brute-force throttle, access-link login, "My Tickets" multi-ticket list, client reply — may be added if trivial.)

## Regressions

- None (greenfield). Client-realm gate must not expose staff-only data.

## Acceptance Criteria

### AC-1: The client portal login accepts a valid ticket number + email and rejects a wrong email for that number. [BROWSER]
- Setup: have a ticket with a known {ticketNumber, email} that already carries a staff reply — either carry over from US-M1-2/US-M1-3, or seed one with a reply via `curl -s -X POST http://localhost:3701/api/dev/seed-ticket -H 'Content-Type: application/json' -d '{"withReply":true}'` → {ticketNumber, email}.
- Setup: use a fresh browser session (no staff cookie) to keep realms separate.
- Navigate: http://localhost:3702/tickets (client portal login).
- Action: enter the ticket number with a WRONG email (e.g. "wrong@example.com"), submit.
- Verify: login is rejected with a sensible error; no client session established.
- Action: enter the ticket number with the correct email, submit.
- Verify: login succeeds and the client lands on the ticket-thread view.
- Status: [ ]

### AC-2: After login, the client sees the ticket thread with their original message and the agent's reply, in order. [BROWSER]
- Navigate: the client ticket-thread view from AC-1 (logged in as the client).
- Verify: the thread renders the customer's original message (`M`) followed by the agent's reply (`R`) in chronological order; the view is read-only (no reply box in M1) and shows no internal note (`N`).
- Status: [ ]

### AC-3: A client session cannot retrieve another ticket's thread. [API-ONLY]
- Setup: seed two tickets — `curl -s -X POST http://localhost:3701/api/dev/seed-ticket` twice → ticketA {numberA, emailA}, ticketB {numberB, emailB}.
- Request: log in the client realm as ticketA — `curl -s -c /tmp/cli.txt -X POST http://localhost:3701/api/client/login -H 'Content-Type: application/json' -d '{"ticketNumber":"<numberA>","email":"<emailA>"}'`.
- Request: `curl -s -b /tmp/cli.txt http://localhost:3701/api/client/ticket` — confirm it returns ONLY ticketA's thread (the session is ticket-scoped; there is no path to request ticketB).
- Expect: the client session returns its own ticket (`M`+`R` only, never `N`) and there is no route by which it can read ticketB's thread; an attempt to reach a staff route with the client cookie is denied (401/403).
- Status: [ ]

## Checklist (children)

- [ ] TS-M1-D1 — Client-realm auth route (ticket# + email) + client thread route
- [ ] TS-M1-D2 — Client portal UI: login + ticket-thread view

## Test Infrastructure

- Uses the ticket + email from US-M1-2 and the reply from US-M1-3 as the browser fixture.
- A fresh browser session (no staff cookie) is used for the client E2E to keep realms separate.
- Dev endpoint consumed (owned/documented in TS-M1-D1): **`POST /api/dev/seed-ticket`** (env-gated,
  supports `{"withReply":true}`) to obtain a fresh `{ ticketNumber, email }` that already carries
  a staff reply, so AC-1/AC-2 can run standalone (decoupled from the US-M1-2/US-M1-3 state).

## Dependencies

- EPIC-M1-A (client-realm gate), US-M1-2 (ticket), US-M1-3 (a reply to view).
