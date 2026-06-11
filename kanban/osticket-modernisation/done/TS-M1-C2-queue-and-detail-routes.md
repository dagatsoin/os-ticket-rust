# TS-M1-C2 — add(route): BS-020/BS-021 open-tickets queue route + ticket-detail route

- **ID**: TS-M1-C2
- **Type**: Technical Story
- **Parent**: US-M1-3
- **Labels**: Technical Story, M1
- **Scope**: small

## Context

Read routes for the staff side: the minimal Open-tickets list (no scoping/tabs/search) and the
single-ticket detail with its thread.

**M1 sort + scope notes:**
- Queue sort: **`created DESC`** (newest first). Thread sort: **`created ASC`** (oldest first).
- The staff detail route returns **ALL entry types (`M`/`R`/`N`)** — staff see internal notes.
  (The client thread route in TS-M1-D1 **excludes `N`**.)
- **Department scoping is intentionally OUT of M1 scope** (note for QA — the single seeded agent
  sees every open ticket).

## Impact

- add(route): `GET /api/staff/tickets?status=open` — list of open tickets (number, subject, created-at).
- add(route): `GET /api/staff/tickets/{id}` — ticket detail incl. ordered thread entries.
- both gated by the staff-realm middleware (TS-M1-C1).

## Regressions

- None new.

## Acceptance Tests

### AC-1: BS-020 — the list route returns Open tickets with number + subject, created DESC (newest first). [API-ONLY]
- Setup: log in staff (`curl -c /tmp/c.txt ... /api/staff/login`); ensure ≥2 open tickets exist (seed via `POST /api/dev/seed-ticket` twice, or `POST /api/tickets`).
- Request: `curl -s -b /tmp/c.txt 'http://localhost:3701/api/staff/tickets?status=open'`.
- Expect: a list of open tickets each with number + subject + created-at, ordered created DESC (the most recently created first).
- Status: [x]

### AC-2: BS-021 — the detail route returns the ticket with its thread entries created ASC (chronological), including all entry types M/R/N. [API-ONLY]
- Setup: seed a ticket that has a staff reply AND an internal note — `curl -s -X POST http://localhost:3701/api/dev/seed-ticket -d '{"withReply":true,"withNote":true}' -H 'Content-Type: application/json'` → capture its id (via the list route).
- Request: `curl -s -b /tmp/c.txt http://localhost:3701/api/staff/tickets/{id}`.
- Expect: the ticket plus thread entries in chronological order (created ASC), INCLUDING the `N` internal note (staff see notes).
- Status: [x]

### AC-3: Both routes are denied without a staff session (401). [API-ONLY]
- Request: `curl -s -o /dev/null -w "%{http_code}" 'http://localhost:3701/api/staff/tickets?status=open'` and `.../api/staff/tickets/1` with NO cookie.
- Expect: 401 for both.
- Status: [x]

## Dependencies

- TS-M1-C1 (staff gate), TS-M1-B1 (ticket data).
