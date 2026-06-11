# osTicket Modernisation — Roadmap

The modernisation rebuilds osTicket 1.7 on a modern stack, driven by the 22 functional specs
in `specs/`. Work is organised in `kanban/osticket-modernisation/` with columns
`backlog → consolidation → todo → inProgress → review → qa → done` and the hierarchy
Milestone → Epic → User Story → Technical Story. Parent columns are **derived** from children
(`parent.column = min(children.columns)`); only leaf tickets and the root (milestone, via its
own browser-only E2E ACs) are moved by hand.

## Target stack (confirmed)

- **Backend**: Rust — Axum + SQLx, Cargo workspace at repo root.
- **Database**: **PostgreSQL** — the **existing shared Docker container `backend-db-1`**
  (`postgres:16`) on host port **5432**, dedicated databases `osticket_dev` / `osticket_staging`.
  The project does **not** start its own Postgres.
- **Frontend**: React + TypeScript — Vite, MobX, Material-UI.

## Milestones

| ID | Milestone | Specs | Status |
|----|-----------|-------|--------|
| M1 | First Ticket Round-Trip (vertical slice) | FS-091/003/001/002/010/011/021/020 subset, FS-040 stub | **DONE (2026-06-11)** |
| M2 | Attachments, Canned Responses & Email Basics | FS-022, FS-040 | Backlog stub |
| M3 | Full Staff Workflow & Queue | FS-021, FS-020 | Backlog stub |
| M4 | Admin Configuration | FS-030/031/032/033 | Backlog stub |
| M5 | Email Pipeline | FS-041/042/040 | Backlog stub |
| M6 | External API & Cron | FS-043 | Backlog stub |
| M7 | Knowledge Base & FAQ | FS-050 | Backlog stub |

## M1 — First Ticket Round-Trip (DONE — 2026-06-11)

**Completed 2026-06-11.** 80 leaf ACs + 5 root E2E ACs + 11 playbook flows, all green; all 21
M1 tickets (root + 2 epics + 3 US + 15 TS) shipped to `done/`.

Vertical slice: *a client opens a ticket via the web form; a staff agent logs in, sees it in
the queue, opens it, and replies; the client sees the reply.* M1 root + epics derive their column
from children.

- **EPIC-M1-A — Foundation & Workspace Setup**: TS-A1 Cargo workspace + Axum skeleton + health;
  TS-A2 Postgres migrations (M1 schema subset); TS-A3 seed fixtures (dept/group/staff);
  TS-A4a validation/sanitization/argon2id hashing utils; TS-A4b sessions/CSRF/realm gates/stub
  mailer (+ `GET /api/dev/mailbox`); TS-A5 React/Vite/TS scaffold.
- **EPIC-M1-B — Ticket Round-Trip**:
  - US-M1-2 client opens a ticket (TS-B1 ticket service **core**, TS-B2 create route, TS-B3 form UI)
  - US-M1-3 staff login + queue + reply (TS-C1 auth, TS-C2 queue/detail routes, TS-C3 reply route, TS-C4 staff UI)
  - US-M1-4 client views reply (TS-D1 client auth + thread route, TS-D2 client portal UI)

**Architectural anchor**: TS-M1-B1 builds the **shared ticket create-and-append core** once.
The web form (M1), staff UI (M1), email (M5), and API (M6) are all thin adapters over it.

**Recommended FE implementation ordering**: **A5 → (B3 ∥ C4) → D2.** A5 lands the scaffold plus
the shared `ThreadView`/`CredentialForm` primitives first; B3 (form UI) and C4 (staff UI) can then
proceed in parallel on top of them; D2 (client portal) follows, reusing the same primitives
read-only.

## Architecture / scope decisions

1. **PostgreSQL replaces MySQL** — *DECISION POINT, confirm with user.* The legacy app is MySQL
   (`ost_`-prefixed tables, full schema in `legacy/setup/inc/streams/core/install-mysql.sql`).
   The proposal targets PostgreSQL (better fit with SQLx/Rust, types, migrations). This means
   MySQL-specific SQL is reimplemented, not ported. If the user requires MySQL, swap the SQLx
   driver and migration dialect — the spec-derived schema is technology-agnostic either way.

2. **FS-092 (CLI/packaging/deployment tooling) — DROPPED.** The legacy shell-only `setup/cli/**`
   subsystem (action dispatch, INCLUDE_DIR rewrite, tar/zip release packager, backup import) is
   replaced by standard modern tooling: **Cargo** (build/release), **Docker / docker-compose**
   (deployment), **CI** (packaging), and **pg_dump / pg_restore** (backup/restore). No kanban
   work item reproduces FS-092.

3. **FS-060 (installer wizard) + FS-061 (upgrader/migration streams) — REPLACED.** The web
   setup wizard and hash-chained schema-patch upgrader are replaced by **standard SQLx
   migrations + a seed fixture + a first-run admin bootstrap**. In M1 the single dept/group/staff
   come from a seed migration (TS-M1-A3); a real first-run admin bootstrap arrives with M4
   (Admin Configuration). No kanban work item reproduces the legacy installer/upgrader flows.

## Decisions (M1 cross-cutting architecture — PINNED)

These were pinned during the M1 consolidation feasibility review by the backend and frontend
leads. They are binding for all M1 tickets; affected tickets reference this section.

1. **Auth model** — Cookie sessions, **DB-backed** (`session` table). **TWO distinct cookies,
   one per realm**: `ost_staff_sess` and `ost_client_sess` (both `HttpOnly`, `SameSite=Lax`,
   `Secure` in non-dev). The cookies are **never shared** across realms. Owned by TS-M1-A4b;
   consumed by TS-M1-C1 (staff) and TS-M1-D1 (client).

2. **CSRF (SPA double-submit)** — On login (each realm) the server seeds a **non-HttpOnly**
   `XSRF-TOKEN-STAFF` / `XSRF-TOKEN-CLIENT` cookie. The frontend apiClient reflects it into an
   **`X-CSRFToken`** header on every mutating request; middleware verifies it on **authenticated
   mutating routes**. **No per-request rotation in M1** (re-seed at login only). **No CSRF on
   unauthenticated public endpoints** — `POST /api/tickets` and the two login POSTs are exempt.
   This **deliberately diverges** from the server-rendered legacy model (FS-001.11 / FS-002.7).
   Owned by TS-M1-A4b (backend) + TS-M1-A5 (apiClient header injection).

3. **Hashing** — **argon2id only**. No phpass/MD5/bcrypt legacy fallback (M1 has no legacy
   accounts). Owned by TS-M1-A4a; consumed by TS-M1-A3 (seed) and TS-M1-C1 (verify).

4. **JSON error envelope** (shared contract, owned by TS-M1-A1) —
   ```json
   { "error": { "message": "<top-level>", "fields": { "<field>": "<msg>" } } }
   ```
   Status codes: **422** validation failures, **401** unauthenticated, **403** forbidden,
   **404** not found. All routes emit this shape; the apiClient (TS-M1-A5) parses it.

5. **SQLx offline mode** — committed **`.sqlx/`** query cache via `cargo sqlx prepare`; CI builds
   **without a live DB**. Convention owned by TS-M1-A1; all DB-touching tickets follow it.

6. **Frontend app shape** (owned by TS-M1-A5) — a **SINGLE Vite SPA on 3702**, **ONE router**
   with **THREE branches**: `/` + `/open` (public open-ticket), `/tickets/*` (client portal),
   `/staff/*` (staff area). **TWO independent MobX auth stores** (staff, client). A5 also owns:
   the shared **apiClient** (CSRF header injection, error-envelope parsing, 401 → realm login
   redirect), the **MSW/mock-API test harness** reused by B3/C4/D2, and the test-runner choice
   **Vitest + React Testing Library**.

## Ports (machine convention — range 37xx)

| Service | Dev | Staging |
|---------|-----|---------|
| Backend API (Rust/Axum) | 3701 | 3711 |
| Frontend (Vite) | 3702 | 3712 |
| PostgreSQL | 5432 (shared `backend-db-1`) | 5432 (shared) |

Registered in `~/.claude/port-registry.md` under `osticket-modernisation`. **3703/3713 are freed** —
Postgres is the shared `backend-db-1` container on 5432 (db `osticket_dev` / `osticket_staging`),
not a project-owned port.
