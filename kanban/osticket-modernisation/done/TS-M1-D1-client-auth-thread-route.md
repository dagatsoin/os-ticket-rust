# TS-M1-D1 — add(route): BS-010 client-realm auth (ticket# + email) + client thread route

- **ID**: TS-M1-D1
- **Type**: Technical Story
- **Parent**: US-M1-4
- **Labels**: Technical Story, M1
- **Scope**: small

## Context

The client-realm adapter: account-less login via ticket number + matching email, establishing a
client session scoped to that one ticket, and a route returning the ticket's thread.

**M1 security + scope notes:**
- The thread route returns **`M` + `R` entries ONLY**. **Internal notes (`N`) MUST NEVER be
  returned to the client** (security AC, FS-010).
- The client session is **ticket-scoped** (the `ost_client_sess` cookie carries only the bound
  ticket), supported by the A4b session abstraction (ROADMAP.md → Decisions → 1).

## Impact

- add(route): `POST /api/client/login` (ticket number + email → `ost_client_sess` client session
  scoped to that ticket).
- add(route): `GET /api/client/ticket` — returns the logged-in client's ticket + thread entries,
  **`M` and `R` only (never `N`)**.
- update(core): client-realm gate ensures a client session can read only its own ticket.

## Regressions

- Realm isolation: ensure the client gate cannot reach staff routes/data (cross-check TS-M1-A4b gate).

## Acceptance Tests

### AC-1: BS-010 — correct ticket# + email establishes a client session; wrong email for that number is rejected. [API-ONLY]
- Setup: seed a ticket — `curl -s -X POST http://localhost:3701/api/dev/seed-ticket` → {ticketNumber, email}.
- Request (wrong email): `curl -s -i -X POST http://localhost:3701/api/client/login -H 'Content-Type: application/json' -d '{"ticketNumber":"<n>","email":"wrong@example.com"}'` → expect 401/422, no session cookie.
- Request (correct): `curl -s -i -c /tmp/cli.txt -X POST http://localhost:3701/api/client/login -H 'Content-Type: application/json' -d '{"ticketNumber":"<n>","email":"<email>"}'` → expect a `ost_client_sess` cookie set.
- Status: [x]

### AC-2: BS-010 — the thread route returns only the session's own ticket and its entries in order. [API-ONLY]
- Request: `curl -s -b /tmp/cli.txt http://localhost:3701/api/client/ticket`.
- Expect: the logged-in client's ticket + thread entries in chronological order; no other ticket's data.
- Status: [x]

### AC-3: BS-010 — a client session is denied access to a different ticket's thread. [API-ONLY]
- Setup: seed ticketB separately. The client session is ticket-scoped (cookie binds ticketA only).
- Request: confirm there is no parameterised path letting the client read ticketB — `GET /api/client/ticket` always returns the bound ticketA; and a staff route accessed with the client cookie returns 401/403.
- Expect: no access to another ticket's thread.
- Status: [x]

### AC-4: FS-010 (security) — the client thread route returns M + R entries and NEVER an N internal note, even if the ticket has one. [API-ONLY]
- Setup: seed a ticket WITH an internal note — `curl -s -X POST http://localhost:3701/api/dev/seed-ticket -H 'Content-Type: application/json' -d '{"withReply":true,"withNote":true}'` → {ticketNumber, email}; log in the client realm for it.
- Request: `curl -s -b /tmp/cli.txt http://localhost:3701/api/client/ticket`.
- Expect: only `M` and `R` entries returned; the `N` internal note is absent (security-critical).
- Status: [x]

## Test Infrastructure

- add(route): **`POST /api/dev/seed-ticket`** (env-gated, disabled in production) that creates an
  isolated ticket (optionally with a staff reply and an internal note) and returns
  `{ ticketNumber, email }`. This gives qa-criterion-tester a fresh ticket + matching email to log
  the client portal in for the M1 client E2E (root AC-5) without coupling to staff-flow state.

## Dependencies

- TS-M1-A4b (client-realm gate/session), TS-M1-B1 (ticket/thread data).
