# osTicket Modernisation Project

## What this repo is

The **osTicket modernisation project**. The repository began as a spec
reverse-engineering effort against a snapshot of **osTicket 1.7** (an open-source PHP web
helpdesk / support-ticket system, ~290 PHP files / ~59,600 lines: a public client portal,
a staff control panel, an admin configuration area, an email pipeline (POP3/IMAP fetch +
mail piping + outbound mail), an external API + cron, and a knowledge base / FAQ, all
backed by MySQL `ost_`-prefixed tables).

That reverse-engineering run is **complete**, and the repo has now entered its
**modernisation phase**: we are building a clean reimplementation on a modern stack, driven
by the specs in [`specs/`](./specs/).

## Current layout

- **`legacy/`** — the original osTicket 1.7 PHP source, archived verbatim and **frozen**.
  The pre-modernisation state is captured by the annotated git tag **`php-1.7-final`**.
- **`specs/`** — technology-agnostic functional specifications (FS-XXX docs with embedded
  BS-XXX business rules, EC-XXX edge cases, KL-XXX known limitations). Start with
  [`specs/CLAUDE.md`](./specs/CLAUDE.md) for the index and conventions.
- **`work/`** — process artifacts from the reverse-engineering run (plan, trackers,
  gap / coverage / dedupe reports). The method is
  [`work/SPEC_REVERSE_ENGINEER_PLAN.md`](./work/SPEC_REVERSE_ENGINEER_PLAN.md).
- **`kanban/`** — modernisation planning (to be created by planning).
- **Rust backend** — a Cargo workspace at the repo root (to be created).
- **React/TypeScript frontend** — (to be created).

## The new implementation

- **Backend**: **Rust** — a Cargo workspace at the repo root (to be created).
- **Frontend**: **React + TypeScript**.
- **Source of truth**: the functional specifications under `specs/` define the behaviour to
  reproduce. The frozen PHP under `legacy/` is the authoritative reference whenever a spec
  is ambiguous.

## Stack decision (WORKING PROPOSAL — confirm before scaffolding)

> Marked as a proposal; awaiting user confirmation. Tracked in
> `kanban/osticket-modernisation/ROADMAP.md` under "Architecture / scope decisions".

- **Backend**: Rust — **Axum** (HTTP) + **SQLx** (DB), Cargo workspace at the repo root.
- **Database**: **PostgreSQL** (the legacy app used **MySQL** — modernisation switches to
  PostgreSQL, **confirmed**). The project does **not** run its own Postgres: it uses the
  **existing shared Docker container `backend-db-1`** (`postgres:16`) already running on this
  machine, host port **5432**, with a dedicated database `osticket_dev` (later
  `osticket_staging`).
- **Frontend**: React + TypeScript — **Vite**, **MobX**, **Material-UI (MUI)**.
- **Migrations / setup**: standard SQLx migrations + a seed fixture + a first-run admin
  bootstrap **replace** the legacy installer/upgrader (FS-060 / FS-061). The legacy CLI /
  packaging tooling (FS-092) is **dropped** in favour of Cargo + Docker + CI + pg_dump.

## Ports (machine convention — range 37xx)

| Service | Dev | Staging | Notes |
|---------|-----|---------|-------|
| Backend API (Rust/Axum) | **3701** | 3711 | REST API |
| Frontend (Vite) | **3702** | 3712 | React dev server |
| Mailpit SMTP (M2 test infra) | **3704** | 3714 | Mailpit container SMTP intake; backend `SMTP_HOST=localhost SMTP_PORT=3704` to exercise M2 deviation D3 |
| Mailpit web UI (M2 test infra) | **3705** | 3715 | Mailpit web inbox (`http://localhost:3705`) — in-browser oracle for E2E email assertions |
| PostgreSQL | **5432** (shared) | 5432 (shared) | **Existing shared container `backend-db-1`** (`postgres:16`); databases `osticket_dev` / `osticket_staging`. Not a project-owned port. |

Registered in `~/.claude/port-registry.md` under `osticket-modernisation`. Ports **3703/3713
are freed** — the project does not start its own Postgres; it reuses the shared `backend-db-1`
container on **5432**.

### Database — existing shared container

- **Container**: `backend-db-1` — image `postgres:16` — host port **5432** (maps to container
  5432). Same instance shared with other projects on this machine (ptidonjon, sandwich, brio_dev…).
- **Superuser**: `postgres` / `pass123`.
- **Dev database**: `osticket_dev` (created). **Test database**: `osticket_test` (created — used
  by the backend DB integration tests via `TEST_DATABASE_URL`). **Staging**: `osticket_staging` (later).
- **Migrations** (TS-M1-A2): SQLx migrations live at the workspace-root `migrations/` directory
  and are applied on app startup (`db::migrate`). Apply manually with
  `DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_dev sqlx migrate run --source migrations`
  (re-running is a safe no-op). Schema is the FS-091-aligned M1 subset (ticket/thread/staff/dept/
  groups/session/config); enum-valued columns are `text` + `CHECK` (no native PG enum types).
- **Connection string template**: `postgres://<user>:<password>@localhost:5432/<database>`
- **`.env` example** (dev):
  ```env
  DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_dev
  ```
- The container is **not** owned by this repo — never `docker rm`/recreate it, and only ever
  touch the `osticket_dev` / `osticket_staging` databases inside it.

## Services

> Backend scaffolded by TS-M1-A1; frontend by TS-M1-A5.

- **Backend API** — Cargo workspace at the **repo root** (`Cargo.toml`), member crates under
  `backend/`: `backend/api` (bin+lib — router, health, CORS, error envelope), `backend/core`
  (shared JSON error envelope + TS-M1-A4a validation/sanitize/argon2id-hashing pure functions),
  `backend/db` (Postgres pool + short-timeout health ping + embedded migrator); plus the
  workspace-member `tools/` crate (the `seed` binary). Port
  **3701**; start: `cargo run -p api`. Config from `backend/.env` (template `backend/.env.example`):
  `DATABASE_URL`, `APP_PORT`, `APP_FRONTEND_ORIGIN`. Health: `GET /api/health` →
  `{ "status": "ok", "db": "ok" | "down" }` (db-down responds fast, never hangs). Shared error
  envelope `{ "error": { "message", "fields" } }` with 422/401/403/404.
- **Frontend** — location: `frontend/` (Vite + React + TS + MobX + MUI; **scaffolded by
  TS-M1-A5**); port **3702**; start: `npm --prefix frontend run dev` (Vite proxies `/api`
  → `http://localhost:3701`). Tests: `npm --prefix frontend test` (Vitest + RTL + MSW).
  Build: `npm --prefix frontend run build`. Lint: `npm --prefix frontend run lint`.
- **PostgreSQL** — **existing shared container `backend-db-1`** (`postgres:16`), host port
  **5432**; database `osticket_dev`. Already running — no `docker compose up` needed.
  Verify: `docker exec backend-db-1 psql -U postgres -d osticket_dev -c "select version();"`.

## Application URLs

> Provisional — valid once M1 / EPIC-M1-A scaffolding lands.

- Frontend: `http://localhost:3702`
- Backend API: `http://localhost:3701/api`
- Health endpoint: `http://localhost:3701/api/health`

## Quick Start

> Provisional — exact commands finalised when scaffolding lands (TS-M1-A1 / A5).

```sh
# PostgreSQL: already running as the shared container `backend-db-1` on :5432
# (db `osticket_dev`); no compose step needed. One-time, if missing:
#   docker exec backend-db-1 psql -U postgres -c "CREATE DATABASE osticket_dev;"
# Copy backend/.env.example → backend/.env (sets DATABASE_URL, APP_PORT=3701, APP_FRONTEND_ORIGIN).
cargo run -p api               # backend on :3701 — applies migrations on startup, then serves
cargo run -p tools --bin seed  # idempotent seed / M1 dev-reset (dept + group + staff + defaults)
npm --prefix frontend run dev  # frontend on :3702
```

**Migrations** are applied automatically on `cargo run -p api` startup (and by the seed task);
apply manually with `sqlx migrate run --source migrations` (see the Database section above).

**Seeded staff credentials** (M1 seed fixture, TS-M1-A3) — used for the staff browser E2E:

| Username | Password    |
|----------|-------------|
| `agent`  | `Agent123!` |

The seed (`cargo run -p tools --bin seed`) is **idempotent** and is the **M1 dev-reset
mechanism** — re-run it any time to return the dev DB to a known state. It upserts one
department (`Support`), one permission group (flags `can_create_tickets` + `can_post_reply`
+ access to `Support`), the `agent` staff account (argon2id-hashed password via the TS-M1-A4a
util), and the FS-091 reference defaults (`default_ticket_status=open`,
`default_priority=normal`). It targets `DATABASE_URL` (default `osticket_dev`) only.

## Testing

> Provisional command set.

- Backend: `cargo test` (workspace unit/integration tests); `cargo clippy --all-targets -- -D warnings`
  (lint); `SQLX_OFFLINE=true cargo build` (CI build without a live DB — uses the committed `.sqlx/`
  query cache; see `.sqlx/README.md` for the `cargo sqlx prepare` convention).
  - **DB-backed tests** (the `db` migration tests and the `tools` seed tests) read
    `TEST_DATABASE_URL`; without it they skip (pass) so DB-less CI stays green. Run them with
    `TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test cargo test`.
- Frontend: `npm --prefix frontend test` (component tests) and `npm --prefix frontend run build`.
- **Browser E2E**: leaf user stories with UI flows and the M1 milestone root are validated by
  **qa-test-plan** + **qa-criterion-tester** against the running frontend (`http://localhost:3702`).
  Unit/API tests are never sufficient for `[BROWSER]` acceptance criteria.

## Planning / Kanban

Modernisation work is planned in `kanban/osticket-modernisation/`
(columns `backlog → consolidation → todo → inProgress → review → qa → done`). The roadmap and
milestone breakdown live in `kanban/osticket-modernisation/ROADMAP.md`. Current focus:
**Milestone M1 — First Ticket Round-Trip** (all M1 leaf tickets in `consolidation/`).

## Hard rules

- **Never push to any remote.** All git operations for this project stay **local** —
  no `git push`, no `git push --tags`, no remote `gh` operations.
- **The legacy PHP source is frozen.** Never modify any file under `legacy/`. The original
  "report spec only — never modify any PHP / source file" rule still applies in full to
  everything under `legacy/`. The only edits ever made to that source were the additive
  `@implements` comment tags inserted during the reverse-engineering coverage phase; no
  further source edits are permitted.
- **Spec / `@implements` paths are pre-move.** File paths referenced inside `specs/` and in
  the `@implements` tags refer to the original (pre-move) repo-root paths. They are now
  located under `legacy/` — e.g. a spec referencing `include/class.ticket.php` now means
  `legacy/include/class.ticket.php`. Prefix legacy references with `legacy/` when resolving
  them.

## Reverse-engineering run summary (complete)

**ALL 4 PHASES COMPLETE — RUN COMPLETE (2026-06-10).** Phase 0 (scouting),
**Phase 1 (GENERATE)**, **Phase 2 (GAP-CLOSE)**, **Phase 3 (COVERAGE)** and
**Phase 4 (DEDUPE)** complete. **22 specs** (21 from Phase 1 + FS-092 added in
Phase 3 round-2 for the `setup/cli/**` tooling), all **Final (Phase 4 de-duplicated)**.

- **Final corpus (heading-counted, post gap-close + dedupe):** 22 specs · 13,426 lines ·
  365 FR · 413 BS · 353 EC · 221 KL.
- **Phase 1 baseline:** 318 FR · 341 BS · 261 EC · 155 KL (before gap-close additions).
- **Phase 2:** 249 gaps found / 249 fixed / 0 open (MISSING 120 · IMPRECISE 84 ·
  INCORRECT 45), which added/expanded FR/BS/EC/KL across all specs.
- **Phase 3:** additive `@implements` tagging, 2 rounds, dry — declaration-level coverage
  **100% of in-scope units, DRY after round 2** (244 in-scope files; **229 modified /
  2293 additive comment insertions / 2189 `@implements` ids (FS 1709 + BS 399 + EC 44 +
  KL 37); 0 bracket tags remain**; 229/229 files `php -l` pass under php 5.6). Tags use
  the requirement-level form `@implements FS-XXX.N: <Title> — <note>` (one id per line,
  BS/EC/KL first-class), normalized from bracketed `[FS-XXX]` in Round 3.
  Out of scope: 46 vendored third-party files, 11 self-test harness files, 2 bare
  redirect stubs. See `work/UNCOVERED_CODE_REPORT.md`.
- **Phase 4:** 107 raw findings → 38 distinct → 24-item worklist across 14 specs; the
  SLA-precedence contradiction resolved; adversarial verification **CLEAN after 4
  post-verification fixes**. See `work/DUPLICATE-ANALYSIS-REPORT.md` +
  `work/dedupe/ADVERSARIAL-VERIFICATION.md`.

See the convergence summary (`work/FINAL_CONVERGENCE_REPORT.md`), the plan
(`work/SPEC_REVERSE_ENGINEER_PLAN.md`), the tracker
(`work/REVERSE_ENGINEER_PROGRESS.md`), the consolidated gap report
(`work/GAP_REPORT.md`), and the coverage report
(`work/UNCOVERED_CODE_REPORT.md`).
