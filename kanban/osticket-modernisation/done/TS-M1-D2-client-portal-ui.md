# TS-M1-D2 — add(web): BS-010 client portal UI (login + ticket-thread view)

- **ID**: TS-M1-D2
- **Type**: Technical Story
- **Parent**: US-M1-4
- **Labels**: Technical Story, M1
- **Scope**: small

## Context

The customer-facing screens to close the loop: a login form (ticket number + email) and the
read-only ticket-thread view showing the agent's reply.

**M1 model notes:**
- The thread view is **READ-ONLY in M1** — **no client reply box** (client reply is out of M1;
  see the US-M1-4 note).
- The client realm uses a **distinct `ost_client_sess` cookie** so the staff and client realms
  **never collide** in the same browser (ROADMAP.md → Decisions → 1).
- The MobX **client store** is a **TDD target**.
- The thread view and login form reuse A5's realm-agnostic `ThreadView` (read-only, no reply slot)
  and `CredentialForm` primitives.

## Impact

- add(web): client portal login page (ticket number + email) consuming `POST /api/client/login`.
- add(web): **read-only** client ticket-thread view (no reply box) rendering the customer message
  + agent reply in order via `GET /api/client/ticket`.
- add(web): client MobX store (TDD target); sensible error on bad credentials.

## Regressions

- None (greenfield).

## Acceptance Tests

> Component/unit tests via Vitest + RTL with a mocked apiClient (MSW) — `npm test`. No live
> backend, no browser → all [API-ONLY]. (Browser-level client flow is covered by US-M1-4 and the
> M1 root E2E.)

### AC-1: Component — login form posts ticket#/email, handles success/failure (mocked client). [API-ONLY]
- Request: `npm test` — the client login component test posts ticket#/email; assert success routes to the thread view and failure shows a sensible error.
- Expect: both paths handled.
- Status: [x]

### AC-2: Component — thread view renders entries in chronological order and is read-only (no reply box) (mocked client). [API-ONLY]
- Request: `npm test` — assert the thread renders `M`/`R` entries oldest-first and that NO reply box is rendered (read-only in M1).
- Expect: ordered, read-only view.
- Status: [x]

### AC-3: Component — the thread view never renders an internal note (N) — only M/R entries returned by the route are shown (mocked client). [API-ONLY]
- Request: `npm test` — the mocked route returns only `M`/`R` (the route already excludes `N`); assert the view shows only those and renders no note affordance.
- Expect: no `N` content rendered.
- Status: [x]

### AC-4: The client store logic is covered by unit tests (TDD target). [API-ONLY]
- Request: `npm test` — the MobX client store unit tests cover login state + thread loading + error handling.
- Expect: client-store unit tests green.
- Status: [x]

## Test Infrastructure

- Drives the M1 client browser E2E (root AC-5) against the real routes in a fresh session.

## Dependencies

- TS-M1-A5 (scaffold), TS-M1-D1 (routes).
