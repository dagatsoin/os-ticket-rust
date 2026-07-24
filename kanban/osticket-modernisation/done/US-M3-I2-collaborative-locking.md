# US-M3-I2 — Collaborative locking blocks conflicting reply

- **ID**: US-M3-I2
- **Type**: User Story
- **Parent**: EPIC-M3-I
- **Labels**: User Story, M3
- **Scope**: medium

## Spec References

- FS-021.18 (collaborative edit locking — TicketLock, auto-lock, AJAX renew/release)
- BS-021.3 (one active lock per ticket; locks block conflicting replies)
- EC-021.1 (reply while locked by another agent)
- EC-021.13 (lock acquisition failure on view)
- EC-021.14 (renew lock no longer owned)

## Context

Epic: EPIC-M3-I — Edit + Delete + Lock.
This story implements collaborative locking so two agents cannot simultaneously reply to the
same ticket. Opening a ticket auto-acquires a lock; the browser renews it periodically; another
agent viewing the same ticket sees a warning and cannot reply until the lock expires.

## Description

As a support agent, when I open a ticket the system auto-acquires a lock for me. If another
agent has the ticket locked, I see a warning "This ticket is currently locked by [name]" and
cannot post a reply until their lock expires or they leave.

## Impact

- Frontend (lock warning banner, auto-renew polling, reply form disable when locked by another)
- Backend (lock acquire/renew/release endpoints, lock TTL from config)
- Database (ticket_lock table)
- Browser (desktop)

## Business Rules

- BS-021.3: At most one non-expired lock exists per ticket. Expired locks are cleared on acquire.
  A staff member cannot post a reply while another staff holds a live lock.
- FS-021.18: Lock TTL is `ticket_lock_time` config (in minutes). Auto-acquire on view; periodic
  renew via AJAX; released when owner closes/assigns/leaves.
- EC-021.1: Reply rejected with "Action Denied. Ticket is locked by someone else!" if locked.

## Regressions

- Reply flow must work normally when no lock conflict (M1/M2 behavior).
- Single-user sessions must still acquire/renew locks without error.

## Acceptance Criteria

### AC-1: Opening a ticket auto-acquires a lock; the SAME staff's second session renews idempotently (no spurious warning). [BROWSER]
> Per FS-021.18 / BS-021.3 (legacy class.ticket.php:349-369, class.lock.php): the lock is keyed by STAFF
> (ticket_id + staff_id). The same staff opening a second session legitimately RENEWS their own lock and is
> NOT blocked. Acquire is idempotent for the lock owner — no "Unable to obtain a lock" warning may appear in
> the owner's sole/first session, nor in a same-staff second session.
- Setup: POST /api/dev/reset-db
- Setup: POST /api/dev/seed-config → {ticket_lock_time: 5}
- Setup: staff A `agent` / `Agent123!` is seeded (M1 seed); no extra staff needed for this AC
- Setup: POST /api/dev/seed-tickets → [{status: "open"}] → {ids: [100]}
- Navigate: http://localhost:3702/staff/login
- Action: Type "agent" in username field
- Action: Type "Agent123!" in password field
- Action: Click "Sign In" button
- Navigate: http://localhost:3702/staff/tickets/100
- Verify: No error banner is displayed (no spurious "Unable to obtain a lock" in the first session)
- Verify: Ticket view loads normally with thread and details visible
- Verify: Network request POST /api/staff/tickets/100/lock was made
- Action: In a SECOND session (incognito / second tab) logged in as the SAME staff `agent`, navigate to http://localhost:3702/staff/tickets/100
- Verify: No lock-warning banner and no error banner (same staff renews their own lock — idempotent acquire)
- Verify: The reply form is enabled in the second same-staff session
- Status: [x]

### AC-2: A DIFFERENT staff member sees the "locked by [name]" warning. [BROWSER]
> A genuine lock conflict requires TWO DIFFERENT staff accounts (the lock is keyed by staff_id). Staff A =
> `agent` / `Agent123!`; staff B = the second seeded account `agent2` / `Agent123!` (added by TS-M3-prep).
> The correct banner wording when another staff holds the lock is exactly "This ticket is currently locked by [name]".
- Setup: POST /api/dev/reset-db
- Setup: staff A `agent` / `Agent123!` (M1 seed) and staff B `agent2` / `Agent123!` (second seeded account, TS-M3-prep) both exist
- Setup: POST /api/dev/seed-tickets → [{status: "open"}] → {ids: [100]}
- Navigate: (Browser 1) http://localhost:3702/staff/login
- Action: Log in as staff A `agent` / `Agent123!`
- Navigate: (Browser 1) http://localhost:3702/staff/tickets/100
- Verify: Lock acquired (no warning banner)
- Navigate: (Browser 2 / Incognito) http://localhost:3702/staff/login
- Action: Log in as staff B `agent2` / `Agent123!`
- Navigate: (Browser 2) http://localhost:3702/staff/tickets/100
- Verify: Warning banner displays "This ticket is currently locked by [staff A's display name]" (the holder is staff A, `agent`)
- Status: [x]

### AC-3: Locked ticket blocks a reply from a DIFFERENT staff member. [BROWSER]
- Setup: staff A `agent` holds the lock on ticket 100 (from AC-2 setup)
- Navigate: (Browser 2 as staff B `agent2`) http://localhost:3702/staff/tickets/100
- Verify: Reply form submit button is disabled OR has tooltip indicating the ticket is locked by staff A
- Action: Attempt to type a reply and click submit
- Verify: Either the button is disabled OR the reply is rejected with "Action Denied. Ticket is locked by someone else!"
- Verify: staff B cannot post a reply until staff A's lock expires or is released
- Status: [x]

### AC-4: Lock expires and other agent can reply. [BROWSER]
- Setup: POST /api/dev/seed-config → {ticket_lock_time: 1}
- Setup: POST /api/dev/seed-tickets → [{status: "open"}] → {ids: [101]}
- Navigate: (Browser 1 as agent1) http://localhost:3702/staff/tickets/101
- Verify: Lock acquired
- Action: Close Browser 1 tab (no navigation away, simulates browser close)
- Manual: Wait 70 seconds for lock to expire
- Navigate: (Browser 2 as agent2) http://localhost:3702/staff/tickets/101
- Verify: No warning banner displayed
- Verify: Reply form is enabled
- Action: Type "Test reply" and click submit
- Verify: Reply is posted successfully
- Status: [x]

### AC-5: Lock renew extends the lock. [BROWSER]
- Setup: POST /api/dev/seed-config → {ticket_lock_time: 2}
- Setup: POST /api/dev/seed-tickets → [{status: "open"}] → {ids: [102]}
- Navigate: (Browser 1 as agent1) http://localhost:3702/staff/tickets/102
- Verify: Lock acquired
- Manual: Wait 3 minutes while keeping page open (polling should renew)
- Verify: Network shows repeated POST /api/staff/tickets/102/lock requests (renew)
- Navigate: (Browser 2 as agent2) http://localhost:3702/staff/tickets/102
- Verify: Warning banner still shows "locked by agent1"
- Status: [x]

### AC-6: Leaving the ticket releases the lock. [BROWSER]
- Setup: POST /api/dev/seed-tickets → [{status: "open"}] → {ids: [103]}
- Navigate: (Browser 1 as agent1) http://localhost:3702/staff/tickets/103
- Verify: Lock acquired
- Action: (Browser 1) Click "Back to Queue" or navigate to http://localhost:3702/staff/tickets
- Verify: Network shows DELETE /api/staff/tickets/103/lock request
- Navigate: (Browser 2 as agent2) http://localhost:3702/staff/tickets/103 immediately
- Verify: No warning banner (agent2 acquires lock)
- Status: [x]

### AC-7: Assign/close releases the lock. [BROWSER]
- Setup: POST /api/dev/seed-tickets → [{status: "open"}] → {ids: [104]}
- Navigate: (Browser 1 as agent1) http://localhost:3702/staff/tickets/104
- Verify: Lock acquired
- Action: (Browser 1) Click "Assign" and assign to agent2
- Verify: agent1 is redirected to queue
- Navigate: (Browser 2 as agent2) http://localhost:3702/staff/tickets/104
- Verify: No warning banner (agent2 acquires lock normally)
- Status: [x]

### AC-8: Lock warning does not block read-only viewing (DIFFERENT staff). [BROWSER]
- Setup: staff A `agent` holds the lock on ticket 100
- Navigate: (Browser 2 as staff B `agent2`) http://localhost:3702/staff/tickets/100
- Verify: Warning banner "This ticket is currently locked by [staff A's display name]" is displayed
- Verify: Ticket thread/messages are visible and readable
- Verify: Ticket details (requester, priority, status) are visible
- Verify: Attachments section is visible and clickable
- Verify: Only reply/note form submit is disabled
- Status: [x]

## Checklist (children)

- [ ] TS-M3-I3 — Backend: lock acquire/renew/release endpoints
- [ ] TS-M3-I6 — Frontend: lock UI (warning banner + auto-renew polling)

## Test Infrastructure

**Credentials:**
- Staff A (lock holder): `agent` / `Agent123!` (M1 seed)
- Staff B (conflicting agent): `agent2` / `Agent123!` — a SECOND seeded staff account added by TS-M3-prep.
  A two-DIFFERENT-staff flow is required to exercise a genuine lock conflict, because the lock is keyed by
  `ticket_id + staff_id` (a same-staff second session renews the owner's own lock and is not blocked).

**Browser Requirements:**
- Two concurrent browser sessions required (use incognito for the second session).
- AC-1 additionally uses a same-staff second session (both logged in as `agent`) to verify idempotent acquire.

**Dev Endpoints Required (Setup only, NOT validation):**
- `POST /api/dev/reset-db` — reset to clean state
- `POST /api/dev/seed-config` — set config values
  - Body: `{ticket_lock_time: N}` (minutes)
- `POST /api/dev/seed-staff` — create multiple staff accounts
  - Body: `[{username, password}]`
- `POST /api/dev/seed-tickets` — create test tickets
  - Returns: `{ids: [...]}`

**Seed Expansion Required (TS-M3-prep):**
- `ticket_lock_time` config key in ost_config
- A SECOND staff account (`agent2` / `Agent123!`) must be seeded (dev endpoint) so the two-DIFFERENT-staff
  lock-conflict flow (AC-2 / AC-3 / AC-8) can be exercised end-to-end. The lock is keyed by staff_id, so a
  real conflict cannot be reproduced with a single account.

## Dependencies

- **TS-M3-schema**: ticket_lock table exists in schema.
- **EPIC-M3-C**: reply workflow exists (lock check added).
- **TS-M3-prep**: `ticket_lock_time` config key, second staff account.

## Review feedback

Returned from M3 root E2E (pre-merge review failure). AC-1, AC-2, AC-3, AC-8 reset to `[ ]`
(all depend on the lock warning/blocking mechanism).

**CORRECTION (spec review vs. frozen legacy — supersedes the "per-session lock token" finding).**
The verified ruling (FS-021.18/FS-021.20, BS-021.3; legacy `class.ticket.php:349-369`, `class.lock.php`,
`staff/ticket-view.inc.php:36`) is:

- **The lock is keyed by STAFF (`ticket_id + staff_id`), NOT per-session — and this is CORRECT.** The SAME
  staff opening a second session legitimately RENEWS their own lock and must NOT be blocked (the earlier
  E2E observation of session B bumping the expire time is the *intended* renewal, not a defect). A genuine
  lock conflict requires a DIFFERENT staff account. Do NOT introduce a per-session/lock-code token. The ACs
  have been reworded to drive the conflict with two DIFFERENT staff (`agent` vs `agent2`).
- **Correct banner wording is exactly "This ticket is currently locked by [name]" — this is CORRECT, keep it.**
  (The earlier note that the UI "never shows the named warning" reflects a still-valid *implementation* gap:
  the UI currently renders only a generic "Unable to obtain a lock" — the named banner still needs implementing
  for AC-2 / AC-8. The AC wording itself is correct.)

Remaining genuine implementation defects to fix (AC wording is now correct; behaviour still needs work):
- **Named-lock warning not implemented.** Implement the banner "This ticket is currently locked by [name]"
  shown to a DIFFERENT staff member who opens a locked ticket (AC-2 / AC-8).
- **Conflicting-reply blocking not demonstrated.** A DIFFERENT staff member's reply composer / Send Reply
  must be blocked (disabled submit / "Action Denied. Ticket is locked by someone else!") while another staff
  holds a live lock (AC-3).
- **Spurious lock error in the FIRST session.** The generic "Unable to obtain a lock" fires spuriously in the
  owner's own first session (observed on tickets 668844 / 637700 / 128280) — acquire is non-idempotent for the
  same staff (a double-invoked acquire whose second call conflicts with the lock the first just created).
  Acquire must be idempotent for the lock owner: no error banner in the first session, and no warning/error in
  a same-staff second session (AC-1). Fixed separately.
- **Seeding note for re-test:** the M1 seed has only one staff account, so a genuine "locked by a DIFFERENT
  staff" scenario cannot be exercised end-to-end. TS-M3-prep must seed a SECOND staff account (`agent2` /
  `Agent123!`) so the root E2E and AC-2/AC-3/AC-8 can drive a true two-staff lock conflict.

<details><summary>Superseded (INCORRECT) original finding — kept for audit</summary>

> - **No per-session lock token.** `ticket_lock` is keyed by `ticket_id + staff_id` only, so a same-agent
>   second session is treated as the lock owner and allowed in ... Key the lock by a per-session/lock-code
>   token (not staff_id alone) so a second session (even same agent) is a distinct holder and can be blocked.
>
> This was incorrect — the staff-keyed lock and same-staff renewal are the specified behaviour; see above.

</details>
