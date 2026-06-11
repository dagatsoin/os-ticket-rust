# TS-M1-C3 — add(route): BS-021 staff reply route (over shared core)

- **ID**: TS-M1-C3
- **Type**: Technical Story
- **Parent**: US-M1-3
- **Labels**: Technical Story, M1
- **Scope**: small

## Context

The staff-UI adapter over the shared ticket core's append operation (TS-M1-B1). Posting a reply
appends a response entry authored by the agent. The stub mailer records (does not send) the
client-notification email.

**M1 behaviour notes:**
- A reply is a **pure append** in M1 — it does **NOT mutate ticket status** (FS-021 `isanswered`
  semantics are **deferred** to a later milestone).
- The mailer intent is **recorded only AFTER the DB commit succeeds** (no notification recorded
  for a failed append).
- QA verifies the recorded notification via A4b's **`GET /api/dev/mailbox`** dev endpoint.

## Impact

- add(route): `POST /api/staff/tickets/{id}/reply` — body → append_thread_entry as a staff response.
- update(core): records an intended client notification via the stub mailer port (no real send in M1).
- gated by staff-realm + "can reply" permission (TS-M1-C1).

## Regressions

- Shares the ticket core with US-M1-2 create; regression-check ticket creation after wiring reply.

## Acceptance Tests

### AC-1: BS-021 — a valid reply appends a staff R response and returns the updated thread, without changing ticket status (pure append in M1). [API-ONLY]
- Setup: log in staff (`curl -c /tmp/c.txt ... /api/staff/login`, capture `XSRF-TOKEN-STAFF`); seed a ticket → capture id + note its current status.
- Request: `curl -s -b /tmp/c.txt -H 'Content-Type: application/json' -H 'X-CSRFToken: <xsrf>' -X POST http://localhost:3701/api/staff/tickets/{id}/reply -d '{"body":"We are looking into it."}'`.
- Expect: the response returns the updated thread with a new `R` entry appended; the ticket status is UNCHANGED (still `open` — no isanswered mutation in M1).
- Status: [ ]

### AC-2: BS-021 — the response author is the authenticated staff account. [API-ONLY]
- Request: read the ticket detail after AC-1 (`GET /api/staff/tickets/{id}`).
- Expect: the new `R` entry is authored by the seeded `agent` staff account.
- Status: [ ]

### AC-3: FS-040 — an intended client notification is recorded by the stub mailer ONLY after the DB commit, retrievable via GET /api/dev/mailbox (not real mail). [API-ONLY]
- Request: after the successful reply in AC-1, `curl -s -b /tmp/c.txt http://localhost:3701/api/dev/mailbox`.
- Expect: a recorded intended notification for the reply (recorded only after commit); no real mail dispatched. (A reply that fails to commit records NO notification.)
- Status: [ ]

### AC-4: It should be denied without a staff session / can_post_reply permission. [API-ONLY]
- Request: `curl -s -o /dev/null -w "%{http_code}" -X POST http://localhost:3701/api/staff/tickets/1/reply -d '{"body":"x"}' -H 'Content-Type: application/json'` (no session).
- Expect: 401; and a session lacking `can_post_reply` is denied 403.
- Status: [ ]

## Test Infrastructure

- QA verifies the recorded reply notification via A4b's env-gated **`GET /api/dev/mailbox`**
  endpoint (returns recorded stub-mailer sends; disabled in production). This is the required
  QA hook for the FS-040 mailer AC.

## Dependencies

- TS-M1-B1 (append core), TS-M1-C1 (auth/gate), TS-M1-A4b (stub mailer + `GET /api/dev/mailbox`).
