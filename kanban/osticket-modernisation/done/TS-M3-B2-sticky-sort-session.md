# TS-M3-B2 — add(service): BS-020.8 sticky sort (session-based per queue)

- **ID**: TS-M3-B2
- **Type**: Technical Story
- **Parent**: US-M3-B1
- **Labels**: Technical Story, M3
- **Scope**: small

## Context

When a staff member sorts a queue (e.g., Open by Date DESC), that preference should be
remembered in their session and automatically re-applied when they return to that queue. Each
queue (open, answered, assigned, overdue, closed) keeps its own independent sticky sort.

This implements BS-020.8: "The chosen sort column and direction are remembered per queue within
the session and re-applied when the staff member returns to that queue without an explicit sort."

## Impact

- add(session): store `{queue}_sort` and `{queue}_order` keys in the session when a sort is
  explicitly requested (e.g., `open_sort=date`, `open_order=DESC`).
- update(route): `GET /api/staff/tickets?status={queue}` — when no `sort` param is supplied,
  check session for `{queue}_sort` and `{queue}_order`; if present, use them instead of the
  default sort. When `sort` param IS supplied, store it in session.
- add(route): `GET /api/staff/tickets/sort-prefs` returns the current sticky sort preferences
  for all queues (for frontend initialization).

## Session Keys

The session stores sticky sort under keys:
- `open_sort`, `open_order`
- `answered_sort`, `answered_order`
- `assigned_sort`, `assigned_order`
- `overdue_sort`, `overdue_order`
- `closed_sort`, `closed_order`

## Sort Resolution Order

1. If `sort` query param is present: use it (and store in session).
2. Else if session has `{queue}_sort`: use session value.
3. Else: use per-queue default sort from TS-M3-B1.

## Regressions

- When no sort param and no session preference, must fall back to queue default sorts.
- Session storage must not interfere with other session data (auth, CSRF, etc.).

## Acceptance Tests

### AC-1: BS-020.8 — sorting with explicit param stores the preference in session. [API-ONLY]
- Setup: POST /api/auth/login {"username":"agent","password":"Agent123!"} -> {session_cookie}
- Request: GET http://localhost:3701/api/staff/tickets?status=open&sort=date&order=ASC
- Headers: Cookie: {session_cookie}
- Expect: 200, tickets returned
- Request: GET http://localhost:3701/api/staff/tickets/sort-prefs
- Headers: Cookie: {session_cookie}
- Expect: 200, `{"open":{"sort":"date","order":"ASC"}}`
- Status: [x]

### AC-2: BS-020.8 — returning to queue without sort param uses session preference. [API-ONLY]
- Setup: POST /api/dev/seed-tickets {"count":5,"status":"open","requesters":["Alice","Charlie","Bob"]} -> {ids}
- Setup: GET /api/staff/tickets?status=open&sort=name&order=DESC (stores preference)
- Request: GET http://localhost:3701/api/staff/tickets?status=open (no sort param)
- Headers: Cookie: {session_cookie}
- Expect: 200, tickets sorted by name DESC (Charlie, Bob, Alice); session preference used
- Status: [x]

### AC-3: BS-020.8 — each queue has independent sticky sort. [API-ONLY]
- Setup: GET /api/staff/tickets?status=open&sort=name&order=ASC (store open pref)
- Setup: GET /api/staff/tickets?status=closed&sort=date&order=DESC (store closed pref)
- Request: GET http://localhost:3701/api/staff/tickets/sort-prefs
- Headers: Cookie: {session_cookie}
- Expect: 200, `{"open":{"sort":"name","order":"ASC"},"closed":{"sort":"date","order":"DESC"}}`
- Status: [x]

### AC-4: BS-020.8 — explicit sort param overrides session preference. [API-ONLY]
- Setup: session has open queue with sort=name&order=ASC (from AC-3)
- Request: GET http://localhost:3701/api/staff/tickets?status=open&sort=ID&order=DESC
- Headers: Cookie: {session_cookie}
- Expect: 200, tickets sorted by ID DESC (explicit wins)
- Request: GET http://localhost:3701/api/staff/tickets/sort-prefs
- Expect: open preference updated to `{"sort":"ID","order":"DESC"}`
- Status: [x]

### AC-5: BS-020.8 — new session with no preferences uses default sorts. [API-ONLY]
- Setup: POST /api/auth/logout
- Setup: POST /api/auth/login {"username":"agent","password":"Agent123!"} -> {new_session_cookie}
- Request: GET http://localhost:3701/api/staff/tickets?status=open (no sort param)
- Headers: Cookie: {new_session_cookie}
- Expect: 200, tickets sorted by priority urgency ASC, effective_date DESC (default sort)
- Status: [x]

### AC-6: Sort prefs endpoint returns empty for queues with no stored preference. [API-ONLY]
- Setup: fresh session (POST /api/auth/login)
- Setup: GET /api/staff/tickets?status=open&sort=date&order=ASC (only store open pref)
- Request: GET http://localhost:3701/api/staff/tickets/sort-prefs
- Headers: Cookie: {session_cookie}
- Expect: 200, `{"open":{"sort":"date","order":"ASC"}}` (closed/answered/etc absent or null)
- Status: [x]

### AC-7: Only valid sort keys are stored in session. [API-ONLY]
- Setup: fresh session
- Request: GET http://localhost:3701/api/staff/tickets?status=open&sort=bogus&order=ASC
- Headers: Cookie: {session_cookie}
- Expect: 200, tickets returned with default sort (invalid key ignored)
- Request: GET http://localhost:3701/api/staff/tickets/sort-prefs
- Expect: 200, open queue absent or null (invalid key not persisted)
- Status: [x]

## Test Infrastructure

### Dev Endpoints Required for Test Setup

| Endpoint | Purpose |
|----------|---------|
| `POST /api/auth/login` | Get session cookie for authenticated requests |
| `POST /api/auth/logout` | Clear session for fresh state tests |
| `POST /api/dev/seed-tickets` | Create test tickets for sort verification |

## Dependencies

- **TS-M3-B1**: sort param handling + per-queue default sorts.
- **M1 session infrastructure**: session storage for staff (already exists from auth).
