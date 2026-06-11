# TS-M1-C4 — add(web): BS-002/BS-020/BS-021 staff UI (login, queue, ticket view + reply)

- **ID**: TS-M1-C4
- **Type**: Technical Story
- **Parent**: US-M1-3
- **Labels**: Technical Story, M1
- **Scope**: medium

## Context

The agent-facing screens: login page, the Open-tickets queue, and the single-ticket view with
a reply box. Consumes the TS-M1-C1/C2/C3 routes.

**M1 model notes (cookie-session, not token):**
- The staff auth store holds the **PROFILE** from `GET /api/staff/me` (`{ id, username, name,
  deptId }`) — **never a token**. The session lives in the `ost_staff_sess` HttpOnly cookie
  (ROADMAP.md → Decisions → 1).
- The reply box is **reply-only** in M1 — **no status/close checkboxes** (status mutation is
  deferred).
- A **401** from any staff route → **redirect to `/staff/login`** (apiClient handles this; see
  TS-M1-A5).
- After posting a reply, the view **refreshes the thread from the reply response** (refetch — no
  optimistic local append).
- The MobX **staff auth store** and **ticket store** are **TDD targets**.
- The ticket-thread view and login form reuse A5's realm-agnostic `ThreadView` (with a reply slot)
  and `CredentialForm` primitives.

## Impact

- add(web): staff login page (username/password) → on success store session, route to panel.
- add(web): staff control-panel shell with an Open-tickets queue list (number + subject), links to detail.
- add(web): ticket detail view rendering the thread (customer message + agent responses) + a
  **reply-only** box that posts and then **refetches the thread from the reply response**.
- add(web): MobX staff auth store (holds the `/api/staff/me` profile, no token) + ticket store;
  logout control; **401 → redirect `/staff/login`**.

## Regressions

- None new (greenfield UI).

## Acceptance Tests

> Component/unit tests via Vitest + RTL with a mocked apiClient (MSW) — `npm test`. No live
> backend, no browser → all [API-ONLY]. (Browser-level staff flow is covered by US-M1-3 and the
> M1 root E2E.)

### AC-1: Component — login form posts credentials and handles success/failure (mocked client). [API-ONLY]
- Request: `npm test` — the staff login component test posts credentials; assert success routes to the panel and failure shows an error.
- Expect: both paths handled.
- Status: [x]

### AC-2: Component — queue renders a list of tickets from the list route (mocked client). [API-ONLY]
- Request: `npm test` — the mocked apiClient returns a list of open tickets; assert the queue renders number + subject rows.
- Expect: rows rendered from the mocked list.
- Status: [x]

### AC-3: Component — detail view renders thread entries in order; posting a reply refetches the thread from the reply response (mocked client). [API-ONLY]
- Request: `npm test` — assert the detail view renders entries in order; after posting a reply, the view uses the reply response to refetch (no optimistic local append).
- Expect: ordered render + refetch-on-reply behaviour.
- Status: [x]

### AC-4: Component — a 401 from a staff route redirects to /staff/login (mocked client). [API-ONLY]
- Request: `npm test` — the mocked apiClient returns 401 on a staff call; assert redirect to /staff/login.
- Expect: redirect occurs.
- Status: [x]

### AC-5: The staff auth store + ticket store logic is covered by unit tests (TDD targets). [API-ONLY]
- Request: `npm test` — unit tests cover the staff auth store (holds the /api/staff/me profile, no token) and the ticket store.
- Expect: store unit tests green.
- Status: [x]

## Test Infrastructure

- Drives the M1 staff browser E2E against the real routes with seeded staff + ticket.

## Dependencies

- TS-M1-A5 (scaffold), TS-M1-C1/C2/C3 (routes).
