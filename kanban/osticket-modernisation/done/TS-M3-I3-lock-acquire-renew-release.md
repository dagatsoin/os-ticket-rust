# TS-M3-I3 — add(route): FS-021.18 lock acquire/renew/release endpoints

- **ID**: TS-M3-I3
- **Type**: Technical Story
- **Parent**: US-M3-I2
- **Labels**: Technical Story, M3
- **Scope**: small

## Context

Implements the backend locking system per FS-021.18. A lock auto-acquires when viewing a ticket
(via an acquire call), can be renewed via AJAX to extend expiry, and is released when the owner
leaves/closes/assigns. Lock TTL comes from the `ticket_lock_time` config (in minutes).

Key behaviors:
- `acquire`: deletes expired locks first, then inserts a new lock (or renews own existing lock).
  Returns lock_id + remaining time. If another non-expired lock exists, returns error.
- `renew`: extends the lock by its original window. If lock is gone/expired, re-acquires.
- `release`: deletes the lock if owned by requester; or all of requester's locks on the ticket.

## Impact

- add(route): `POST /api/staff/tickets/{ticketId}/lock` — acquires or renews lock.
- add(route): `DELETE /api/staff/tickets/{ticketId}/lock` — releases lock.
- add(service): `lock_service::acquire_lock(staff_id, ticket_id, lock_time_minutes)` — returns
  `{ lock_id, remaining_seconds }` or error if held by another.
- add(service): `lock_service::renew_lock(lock_id, staff_id)` — extends expiry; returns updated
  remaining time.
- add(service): `lock_service::release_lock(lock_id, staff_id)` — deletes lock if owned.
- add(cron): `lock_service::cleanup()` — deletes all expired locks (called by cron sweep).

## Regressions

- Ticket view must call acquire on load; reply endpoint must check lock ownership.

## Acceptance Tests

> Setup (all): `cargo run -p tools --bin seed -- --reset` with `ticket_lock_time=5` (minutes);
> backend on :3701; staff login agent1 → `/tmp/qa-agent1.jar`, agent2 → `/tmp/qa-agent2.jar`.

### AC-1: FS-021.18 — acquire lock returns lock_id and remaining time. [API-ONLY]
- Setup: POST /api/dev/seed-config → {ticket_lock_time: 5}
- Setup: POST /api/dev/seed-staff → [{username: "agent1", password: "Agent123!"}]
- Setup: POST /api/dev/seed-tickets → [{status: "open"}] → {ids: [10]}
- Setup: POST /api/dev/login → {username: "agent1"} → cookie jar /tmp/qa-agent1.jar
- Request: `curl -s -b /tmp/qa-agent1.jar -X POST http://localhost:3701/api/staff/tickets/10/lock`
- Expect: HTTP 200, body contains `"lock_id"` and `"remaining_seconds"` between 290-300
- Status: [x]

### AC-2: FS-021.18 — another agent cannot acquire while lock is held. [API-ONLY]
- Setup: POST /api/dev/seed-staff → [{username: "agent2", password: "Agent123!"}]
- Setup: POST /api/dev/login → {username: "agent2"} → cookie jar /tmp/qa-agent2.jar
- Setup: agent1 holds lock on ticket 10 (from AC-1)
- Request: `curl -s -b /tmp/qa-agent2.jar -X POST http://localhost:3701/api/staff/tickets/10/lock`
- Expect: HTTP 409, body contains `"Ticket is currently locked by agent1"`
- Status: [x]

### AC-3: FS-021.18 — renew extends the lock. [API-ONLY]
- Setup: agent1 holds lock on ticket 10 (acquired ~1 minute ago)
- Request: `curl -s -b /tmp/qa-agent1.jar -X POST http://localhost:3701/api/staff/tickets/10/lock`
- Expect: HTTP 200, body contains `"remaining_seconds"` between 290-300 (reset to full window)
- Status: [x]

### AC-4: FS-021.18 — release deletes the lock. [API-ONLY]
- Setup: agent1 holds lock on ticket 10
- Request: `curl -s -b /tmp/qa-agent1.jar -X DELETE http://localhost:3701/api/staff/tickets/10/lock`
- Expect: HTTP 200, body contains `"released": true`
- Request: `curl -s -b /tmp/qa-agent2.jar -X POST http://localhost:3701/api/staff/tickets/10/lock`
- Expect: HTTP 200 (agent2 successfully acquires lock)
- Status: [x]

### AC-5: EC-021.14 — release by non-owner is no-op. [API-ONLY]
- Setup: POST /api/dev/seed-tickets → [{status: "open"}] → {ids: [11]}
- Setup: agent1 acquires lock on ticket 11
- Request: `curl -s -b /tmp/qa-agent2.jar -X DELETE http://localhost:3701/api/staff/tickets/11/lock`
- Expect: HTTP 200, body contains `"released": false`
- Request: `curl -s -b /tmp/qa-agent2.jar -X POST http://localhost:3701/api/staff/tickets/11/lock`
- Expect: HTTP 409 (agent1's lock still active)
- Status: [x]

### AC-6: Expired lock is cleared on acquire. [API-ONLY]
- Setup: POST /api/dev/seed-config → {ticket_lock_time: 1}
- Setup: POST /api/dev/seed-tickets → [{status: "open"}] → {ids: [12]}
- Setup: agent1 acquires lock on ticket 12
- Manual: Wait 70 seconds for lock to expire
- Request: `curl -s -b /tmp/qa-agent2.jar -X POST http://localhost:3701/api/staff/tickets/12/lock`
- Expect: HTTP 200 (agent2 acquires the lock, expired lock was cleared)
- Status: [x]

### AC-7: Acquire on non-existent ticket returns 404. [API-ONLY]
- Request: `curl -s -b /tmp/qa-agent1.jar -X POST http://localhost:3701/api/staff/tickets/99999/lock`
- Expect: HTTP 404, body contains `"Ticket not found"`
- Status: [x]

### AC-8: Lock check on reply — blocks reply when locked by another. [API-ONLY]
- Setup: POST /api/dev/seed-tickets → [{status: "open"}] → {ids: [13]}
- Setup: agent1 acquires lock on ticket 13
- Request: `curl -s -b /tmp/qa-agent2.jar -X POST http://localhost:3701/api/staff/tickets/13/reply -H "Content-Type: application/json" -d '{"response":"test"}'`
- Expect: HTTP 409, body contains `"Action Denied. Ticket is locked by someone else!"`
- Status: [x]

## Test Infrastructure

**Dev Endpoints Required (Setup only):**
- `POST /api/dev/seed-config` — set config values including `ticket_lock_time`
- `POST /api/dev/seed-staff` — create staff accounts
- `POST /api/dev/seed-tickets` — create test tickets; returns `{ids: [...]}`
- `POST /api/dev/login` — authenticate and return session cookie

**Notes:**
- AC-6 requires manual wait (70 seconds) for lock expiration; can be accelerated with shorter `ticket_lock_time` in test environment

## Dependencies

- **TS-M3-schema**: ticket_lock table exists in schema.
- **TS-M3-prep**: `ticket_lock_time` config key, second staff account for testing.
- **EPIC-M3-C**: reply endpoint to integrate lock check.
