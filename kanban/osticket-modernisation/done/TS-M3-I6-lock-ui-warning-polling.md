# TS-M3-I6 — add(frontend): FS-021.18 lock UI (warning banner + auto-renew polling)

- **ID**: TS-M3-I6
- **Type**: Technical Story
- **Parent**: US-M3-I2
- **Labels**: Technical Story, M3
- **Scope**: small

## Context

Implements the frontend locking UI per FS-021.18. On ticket view load, the frontend calls
acquire lock. If successful, it starts a polling interval to renew the lock before expiry.
If another agent holds the lock, a warning banner is shown and reply/note forms are disabled.

On page unload (or navigation away), the lock is released.

## Impact

- update(component): `TicketView` — on mount, call acquire lock; store lock_id + remaining time;
  start renew polling (e.g., every 60 seconds). On unmount, call release lock.
- add(component): `LockWarningBanner` — displays "This ticket is currently locked by [name]"
  when the ticket is locked by another agent.
- update(component): `ReplyForm` / `NoteForm` — disable submit when `lockedByAnother=true`.
- add(hook): `useLock(ticketId)` — manages lock state, acquire/renew/release calls.

## Regressions

- Ticket view must still load correctly if lock service is down (degrade gracefully).
- Reply/note should work normally when user holds the lock.

## Acceptance Tests

### AC-1: Ticket view acquires lock on mount. [BROWSER]
- Setup: POST /api/dev/reset-db
- Setup: POST /api/dev/seed-config → {ticket_lock_time: 5}
- Setup: POST /api/dev/seed-staff → {username: "agent1", password: "Agent123!"}
- Setup: POST /api/dev/seed-tickets → [{status: "open"}] → {ids: [10]}
- Navigate: http://localhost:3702/staff/login
- Action: Type "agent1" in username field
- Action: Type "Agent123!" in password field
- Action: Click "Sign In" button
- Navigate: http://localhost:3702/staff/tickets/10
- Verify: Network shows POST /api/staff/tickets/10/lock request
- Verify: No warning banner is displayed
- Verify: Ticket view loads with thread and details visible
- Status: [x]

### AC-2: Warning banner shown when locked by another. [BROWSER]
- Setup: POST /api/dev/seed-staff → [{username: "agent1"}, {username: "agent2", password: "Agent123!"}]
- Setup: POST /api/dev/seed-tickets → [{status: "open"}] → {ids: [11]}
- Navigate: (Browser 1) http://localhost:3702/staff/login as agent1
- Action: Log in as agent1 / Agent123!
- Navigate: (Browser 1) http://localhost:3702/staff/tickets/11
- Verify: Lock acquired (no warning banner)
- Navigate: (Browser 2 / Incognito) http://localhost:3702/staff/login
- Action: Log in as agent2 / Agent123!
- Navigate: (Browser 2) http://localhost:3702/staff/tickets/11
- Verify: Warning banner is visible
- Verify: Banner text contains "locked by agent1"
- Status: [x]

### AC-3: Reply form disabled when locked by another. [BROWSER]
- Setup: agent1 holds lock on ticket 11 (from AC-2)
- Navigate: (Browser 2 as agent2) http://localhost:3702/staff/tickets/11
- Verify: Reply form submit button has disabled attribute
- Verify: Reply form submit button has tooltip "Ticket is locked by another agent"
- Status: [x]

### AC-4: Lock is renewed periodically. [BROWSER]
- Setup: POST /api/dev/seed-config → {ticket_lock_time: 2}
- Setup: POST /api/dev/seed-tickets → [{status: "open"}] → {ids: [12]}
- Navigate: (Browser 1 as agent1) http://localhost:3702/staff/tickets/12
- Verify: Initial POST /api/staff/tickets/12/lock request made
- Manual: Wait 90 seconds while keeping page open
- Verify: Network shows additional POST /api/staff/tickets/12/lock request (renew)
- Navigate: (Browser 2 as agent2) http://localhost:3702/staff/tickets/12
- Verify: Warning banner still shows "locked by agent1"
- Status: [x]

### AC-5: Lock is released on navigation away. [BROWSER]
- Setup: POST /api/dev/seed-tickets → [{status: "open"}] → {ids: [13]}
- Navigate: (Browser 1 as agent1) http://localhost:3702/staff/tickets/13
- Verify: Lock acquired
- Action: (Browser 1) Click "Back to Queue" link or navigate to /staff/tickets
- Verify: Network shows DELETE /api/staff/tickets/13/lock request
- Navigate: (Browser 2 as agent2) http://localhost:3702/staff/tickets/13 immediately
- Verify: No warning banner (agent2 acquires lock)
- Status: [x]

### AC-6: Lock released on page close. [BROWSER]
- Setup: POST /api/dev/seed-config → {ticket_lock_time: 1}
- Setup: POST /api/dev/seed-tickets → [{status: "open"}] → {ids: [14]}
- Navigate: (Browser 1 as agent1) http://localhost:3702/staff/tickets/14
- Verify: Lock acquired
- Action: (Browser 1) Close browser tab
- Manual: Wait 70 seconds for lock to expire
- Navigate: (Browser 2 as agent2) http://localhost:3702/staff/tickets/14
- Verify: No warning banner (agent2 can acquire lock)
- Status: [x]

### AC-7: Graceful degradation if lock acquire fails. [BROWSER]
- Setup: POST /api/dev/simulate-error → {endpoint: "POST /api/staff/tickets/15/lock", status: 500}
- Setup: POST /api/dev/seed-tickets → [{status: "open"}] → {ids: [15]}
- Navigate: http://localhost:3702/staff/tickets/15
- Verify: Ticket view loads (not blocked)
- Verify: Warning appears "Unable to obtain a lock on the ticket" (non-blocking)
- Verify: Reply form submit button is NOT disabled (graceful fallback)
- Status: [x]

### AC-8: View-only is not blocked by lock. [BROWSER]
- Setup: agent1 holds lock on ticket 11
- Navigate: (Browser 2 as agent2) http://localhost:3702/staff/tickets/11
- Verify: Warning banner is displayed
- Verify: Ticket thread/messages are visible and scrollable
- Verify: Ticket details (requester, priority, status) are visible
- Verify: Attachments section is visible and files are clickable
- Verify: Only reply/note form submit buttons are disabled
- Status: [x]

## Test Infrastructure

**Credentials:**
- Primary: `agent1` / `Agent123!`
- Secondary: `agent2` / `Agent123!`

**Browser Requirements:**
- Two concurrent browser sessions required (use incognito for second session)

**Dev Endpoints Required (Setup only):**
- `POST /api/dev/reset-db` — reset to clean state
- `POST /api/dev/seed-config` — set config values (`ticket_lock_time`)
- `POST /api/dev/seed-staff` — create multiple staff accounts
- `POST /api/dev/seed-tickets` — create test tickets
- `POST /api/dev/simulate-error` — inject API error for graceful degradation testing

**Notes:**
- AC-4 and AC-6 require manual wait periods for lock timing tests
- AC-7 requires ability to simulate backend errors

## Dependencies

- **TS-M3-schema**: ticket_lock table exists in schema.
- **TS-M3-I3**: backend lock endpoints.
- **EPIC-M3-C**: reply form component.
- **EPIC-M3-E**: note form component.
- **TS-M3-prep**: second staff account, ticket_lock_time config.
