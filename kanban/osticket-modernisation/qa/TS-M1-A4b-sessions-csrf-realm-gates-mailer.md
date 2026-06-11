# TS-M1-A4b — add(core): BS-001/BS-002 sessions, CSRF, realm gates & stub mailer

- **ID**: TS-M1-A4b
- **Type**: Technical Story
- **Parent**: EPIC-M1-A
- **Labels**: Technical Story, M1
- **Scope**: medium

## Context

The second half of the cross-cutting backend layer, split out from the original TS-M1-A4
(scope too big). This ticket is the stateful/middleware tier: DB-backed sessions, CSRF
middleware, the two realm gates, a generic permission gate, and the stub mailer port.
Reimplemented from the FS-001/FS-002 infrastructure specs (only the parts M1 exercises).
Includes the stubbed mailer behind a port so later milestones (M2/M5) can swap in a real SMTP
transport without touching callers. See ROADMAP.md → Decisions for the pinned auth/CSRF model.

## Impact

- add(core): BS-001/BS-002 DB-backed sessions (FS-002.14) against the `session` table — TTL
  **86400**, **session-id regeneration on login** (fixation defence). The session abstraction
  MUST support **both realm shapes**: a staff account-scoped session and a client ticket-scoped
  session.
- add(core): BS-001 CSRF middleware — double-submit verification on authenticated mutating
  routes (see ROADMAP.md → Decisions → CSRF for the SPA double-submit model and the deliberate
  divergence from the legacy server-rendered model FS-001.11/FS-002.7).
- add(core): BS-001 realm gates — **staff** realm (account-scoped session) and **client** realm
  (ticket-scoped session).
- add(core): BS-001 **generic permission gate** — checks a *named* permission, not a hardcoded
  "can reply" (so C3 and later milestones reuse it).
- add(core): FS-040 mailer **port** (trait) with a stub implementation that **logs and records**
  intended sends (sends nothing).
- add(route): **`GET /api/dev/mailbox`** (env-gated, disabled in production) returning the
  recorded stub-mailer sends — required by C3's QA to verify reply notifications.

## Regressions

- None (greenfield). Shared code path: every authenticated route depends on session/CSRF/realm
  gates; flows in US-M1-2/3/4 must be regression-checked when this changes.

## Acceptance Tests

### AC-1: BS-001 — reject a state-changing authenticated request without a valid CSRF token; accept one with a matching double-submit token. [API-ONLY]
- Setup: log in staff to obtain `ost_staff_sess` + `XSRF-TOKEN-STAFF` — `curl -s -c /tmp/c.txt -X POST http://localhost:3701/api/staff/login -H 'Content-Type: application/json' -d '{"username":"agent","password":"Agent123!"}'`.
- Request (missing token): POST an authenticated mutating route (e.g. a staff reply) with the session cookie but NO `X-CSRFToken` header → expect 403.
- Request (matching token): repeat with `-H "X-CSRFToken: <value of XSRF-TOKEN-STAFF cookie>"` → expect the request is accepted (passes CSRF, proceeds to the route).
- Status: [ ]

### AC-2: BS-001 — deny a staff-realm route to a client-realm session and vice versa. [API-ONLY]
- Setup: obtain a client session — `curl -s -c /tmp/cli.txt -X POST http://localhost:3701/api/client/login -d '{"ticketNumber":"<n>","email":"<e>"}' -H 'Content-Type: application/json'`.
- Request: use the client cookie against a staff route — `curl -s -o /dev/null -w "%{http_code}" -b /tmp/cli.txt http://localhost:3701/api/staff/tickets?status=open` → expect 401/403.
- Request: use a staff cookie against the client route `GET /api/client/ticket` → expect 401/403.
- Status: [ ]

### AC-3: BS-001 — a DB-backed session expires after the 86400s TTL and the session id is regenerated on login. [API-ONLY]
- Request (regeneration): record the session id before login, log in, and confirm a NEW session id is issued (fixation defence) — assert via integration test or by comparing `ost_staff_sess` cookie values pre/post login.
- Request (TTL): covered by an integration test that inserts a session with an expiry past 86400s and asserts it is treated as expired/invalid. (TTL = 86400 in the `session` table.)
- Status: [ ]

### AC-4: BS-001 — the generic permission gate allows/denies based on a named permission. [API-ONLY]
- Request: `cargo test -p core permission_gate` — assert the gate grants when the session's group holds the named permission (`can_post_reply`) and denies when it does not.
- Expect: allow/deny purely on the named permission (not a hardcoded check).
- Status: [ ]

### AC-5: FS-040 — the stub mailer records an intended send without dispatching real mail. [API-ONLY]
- Request: `cargo test -p core stub_mailer` (or trigger a flow that sends) and assert the mailer port recorded the intended send while dispatching nothing.
- Expect: the recorded-sends store grows by one; no real SMTP transport is invoked.
- Status: [ ]

### AC-6: FS-040 — GET /api/dev/mailbox returns recorded sends when dev-enabled and is disabled (404/forbidden) in production. [API-ONLY]
- Request (dev): `curl -s http://localhost:3701/api/dev/mailbox` with the dev env flag enabled → returns the JSON list of recorded sends.
- Request (prod): build/run with the dev flag OFF → `curl -s -o /dev/null -w "%{http_code}" http://localhost:3701/api/dev/mailbox` returns 404/403.
- Status: [ ]

## Test Infrastructure

- **`GET /api/dev/mailbox`** (env-gated dev endpoint) surfaces recorded stub-mailer sends so
  qa-criterion-tester can verify C3's reply notification without a real SMTP transport. Disabled
  in production builds.

## Dependencies

- TS-M1-A1 (workspace). Pairs with TS-M1-A2 for the `session` table.
- Unblocks: TS-M1-C1/TS-M1-C3 (sessions/CSRF/permission gate, mailbox) and TS-M1-D1
  (client-realm session).
