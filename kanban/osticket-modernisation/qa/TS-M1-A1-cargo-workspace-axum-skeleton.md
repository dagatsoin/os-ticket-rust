# TS-M1-A1 — add(infra): Cargo workspace + Axum server skeleton + health endpoint

- **ID**: TS-M1-A1
- **Type**: Technical Story
- **Parent**: EPIC-M1-A
- **Labels**: Technical Story, M1
- **Scope**: small

## Context

First brick of the modernised backend. Establishes the Cargo workspace at the repo root and
a minimal Axum HTTP server with config loading, structured logging, a router, error-handling
middleware, and a `/api/health` endpoint that reports DB reachability. Supports
EPIC-M1-A AC-2 (FE→BE→DB connectivity) and underpins every later route.

## Impact

- add(infra): Cargo workspace at repo root (e.g. `backend/` crate(s): `api`, `core`, `db`).
- add(infra): Axum server bound to port **3701**, config from env (`.env` / `APP_*`).
- add(route): `GET /api/health` returning `{ status, db: ok|down }` (FS-001 health concept) —
  the DB ping uses a **short timeout** so a db-down state returns a fast structured response and
  never hangs the request.
- add(infra): tracing/log setup, CORS for the Vite origin (`http://localhost:3702`), and the
  **shared JSON error envelope** (this ticket owns the contract; see ROADMAP.md → Decisions → 4):
  `{ "error": { "message": "<top-level>", "fields": { "<field>": "<msg>" } } }` with 422
  validation / 401 unauthenticated / 403 forbidden / 404 not found.
- add(infra): **SQLx offline-mode convention** — committed `.sqlx/` query cache via
  `cargo sqlx prepare` so CI builds without a live DB (ROADMAP.md → Decisions → 5).

## Regressions

- None (greenfield).

## Acceptance Tests

### AC-1: It should boot the Axum server and bind port 3701. [API-ONLY]
- Setup: start the API — `cargo run -p api` (DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_dev).
- Request: `curl -s -o /dev/null -w "%{http_code}" http://localhost:3701/api/health`.
- Expect: the port is listening and returns an HTTP response (not connection-refused).
- Status: [ ]

### AC-2: GET /api/health returns 200 with status "ok" when the DB is reachable. [API-ONLY]
- Request: `curl -s http://localhost:3701/api/health` (shared backend-db-1 Postgres up).
- Expect: 200, body `{ "status": "ok", "db": "ok" }`.
- Status: [ ]

### AC-3: GET /api/health reports db "down" (not crash) within the short ping timeout when the DB is unreachable. [API-ONLY]
- Setup: run the API pointed at an unreachable DB (e.g. DATABASE_URL pointing at a dead port) OR temporarily stop DB reachability.
- Request: time the call — `curl -s -w "\n%{time_total}\n" http://localhost:3701/api/health`.
- Expect: a fast structured response (within the short ping timeout, no hang) reporting `db: "down"`; the process does not crash.
- Status: [ ]

### AC-4: Error responses use the shared JSON error envelope with the correct status codes (422/401/403/404). [API-ONLY]
- Request: hit an unknown route — `curl -s -w "\n%{http_code}\n" http://localhost:3701/api/does-not-exist`.
- Expect: 404 with shape `{ "error": { "message": "...", "fields": {...} } }`. (422/401/403 are exercised by downstream tickets B2/C1; the envelope shape is owned and asserted here.)
- Status: [ ]

### AC-5: cargo build and cargo test pass on the workspace. [API-ONLY]
- Request: `cargo build` then `cargo test` at the repo root.
- Expect: both succeed (exit 0).
- Status: [ ]

### AC-6: The build succeeds in CI without a live DB using the committed .sqlx/ cache (SQLx offline mode). [API-ONLY]
- Request: `SQLX_OFFLINE=true cargo build` with no DATABASE_URL / no DB reachable.
- Expect: build succeeds using the committed `.sqlx/` query cache.
- Status: [ ]

## Test Infrastructure

- `/api/health` doubles as the smoke endpoint the frontend shell calls (EPIC-M1-A AC-2).

## Dependencies

- None.
