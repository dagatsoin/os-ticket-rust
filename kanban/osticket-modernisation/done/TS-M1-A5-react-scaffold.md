# TS-M1-A5 — add(web): React + Vite + TS app scaffold (routing, MUI, MobX, API client)

- **ID**: TS-M1-A5
- **Type**: Technical Story
- **Parent**: EPIC-M1-A
- **Labels**: Technical Story, M1
- **Scope**: small

## Context

The frontend platform every EPIC-M1-B screen is built on. A **SINGLE Vite SPA on 3702** with
Material-UI theming, MobX for state, React Router, and a typed API client pointed at the backend.
Includes a health indicator on the shell to prove FE→BE→DB connectivity (EPIC-M1-A AC-2).
See ROADMAP.md → Decisions → 6 for the pinned app shape.

**This ticket owns the frontend cross-cutting platform:**
- **ONE router with THREE branches**: `/` + `/open` (public open-ticket), `/tickets/*` (client
  portal), `/staff/*` (staff area). (The client-portal branch was previously omitted.)
- **TWO independent MobX auth stores** — staff and client — that never share state.
- The shared **`apiClient`**: injects the `X-CSRFToken` header from the realm XSRF cookie on
  mutating requests, parses the shared JSON error envelope, and on **401** redirects to the
  appropriate realm login (`/staff/login` or the client login).
- The **MSW / mock-API test harness** reused by B3/C4/D2 component tests.
- Test runner: **Vitest + React Testing Library**.
- The **root store wiring** and the **apiClient** are **TDD targets**.
- **Two realm-agnostic presentational primitives** (no store/realm knowledge) consumed by C4 and D2:
  - **`ThreadView`** — renders an ordered list of thread entries passed in as props; no store or
    realm awareness. C4 uses it with a reply slot; D2 uses it read-only (no slot).
  - **`CredentialForm`** — generic fields + submit + error display, including **422 field-error
    mapping** from the shared error envelope. Staff and client each supply their own field config
    and submit handler.

## Impact

- add(web): Vite + React + TS project under `frontend/`, dev server on port **3702**, proxy to API on **3701**.
- add(web): MUI theme + base layout shell; ONE React Router with **three branches** — `/` +
  `/open` (public), `/tickets/*` (client portal), `/staff/*` (staff) — empty placeholders for now.
- add(web): MobX root store wiring + **two independent auth stores** (staff, client).
- add(web): shared `apiClient` wrapper — `X-CSRFToken` header injection (realm XSRF cookie),
  shared error-envelope parsing, 401 → realm login redirect.
- add(web): **Vitest + React Testing Library + MSW** test harness (reused by B3/C4/D2).
- add(web): realm-agnostic presentational primitives **`ThreadView`** (ordered thread entries as
  props, no store/realm knowledge — C4 with a reply slot, D2 read-only) and **`CredentialForm`**
  (generic fields + submit + error display + 422 field-error mapping — staff/client supply field
  config + submit handler). Consumed by C4 and D2.
- add(web): shell "backend OK / down" indicator calling `GET /api/health`.

## Regressions

- None (greenfield).

## Acceptance Tests

### AC-1: npm run build and npm test (Vitest) pass. [API-ONLY]
- Request: in `frontend/`, run `npm run build` then `npm test`.
- Expect: both succeed (build emits the bundle; Vitest suite green).
- Status: [x]

### AC-2: The dev server serves the app shell at the configured frontend URL. [BROWSER]
- Setup: start the dev server — `npm run dev` (port 3702); API may be mocked/unavailable for this check.
- Navigate: http://localhost:3702/
- Verify: the React app shell renders (MUI layout visible) — not a blank page, build error overlay, or connection-refused.
- Status: [x]

### AC-3: The router resolves all three branches (/, /tickets/*, /staff/*) to their placeholder views. [BROWSER]
- Navigate: http://localhost:3702/ then http://localhost:3702/staff/login then http://localhost:3702/tickets.
- Verify: each branch resolves to its placeholder view (public, staff, client) without a router "no match" / blank screen.
- Status: [x]

### AC-4: The apiClient injects X-CSRFToken on a mutating request and redirects to the realm login on a 401 (TDD target). [API-ONLY]
- Request: `npm test` — the apiClient unit test (MSW-mocked) asserts a mutating request carries the `X-CSRFToken` header from the realm XSRF cookie, and that a 401 response triggers a redirect to the matching realm login (`/staff/login` or client login).
- Expect: both assertions pass.
- Status: [x]

### AC-5: The apiClient parses the shared JSON error envelope into field/top-level errors (unit test). [API-ONLY]
- Request: `npm test` — the apiClient unit test feeds `{ "error": { "message": "...", "fields": { "email": "..." } } }` and asserts it is parsed into top-level + per-field errors.
- Expect: parsing assertion passes.
- Status: [x]

### AC-6: The shell displays a healthy/unhealthy indicator reflecting the /api/health response (component test, MSW harness). [API-ONLY]
- Request: `npm test` — a component test with the MSW harness mocks `/api/health` as ok then down and asserts the shell indicator reflects "backend OK" / "down" accordingly.
- Expect: indicator reflects both states.
- Status: [x]

## Test Infrastructure

- This scaffold is the host for all qa-criterion-tester browser runs in M1.
- Vite dev proxy removes the need for CORS gymnastics during browser testing.
- The **Vitest + RTL + MSW** mock-API harness defined here is reused by the B3/C4/D2 component
  tests (mocked apiClient).

## Dependencies

- None hard (can scaffold against a mocked health response); integrates with TS-M1-A1's `/api/health`.
