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
| M2 | Attachments, Canned Responses & Email Basics | FS-022, FS-040 | **Consolidation (2026-06-11)** |
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

## M2 — Attachments, Canned Responses & Email Basics (CONSOLIDATION — 2026-06-11)

Builds on M1's round-trip so a ticket can carry **attachments**, an agent can answer with a
**canned response** whose body is personalised by a **`%{token}` engine**, and the M1 stub mailer is
replaced by **real outbound email** (autoresponse + reply notification) delivered over SMTP and
observable in **Mailpit**. Tickets live in `consolidation/` (1 root + 5 epics + 4 US + 16 TS).

**Demo path (root E2E, browser-only):** *a client opens a ticket WITH an attachment → an agent
replies USING a canned response → the client RECEIVES the reply email (Mailpit) and DOWNLOADS the
attachment.*

### Epic split & dependency order (A → B; C standalone; D needs A+C; E needs C)

- **EPIC-M2-A — Attachment storage & upload**: TS-A1 SHA-256 filesystem blob store + `attachment_file`
  / `ticket_attachment` schema; TS-A2 upload validation (extension allow-list + max size) + config
  keys (`allow_attachments`, `allowed_filetypes` seed `.pdf,.png,.jpg,.txt,.doc`, `max_file_size`
  1 MB); **US-M2-1** client attaches a file on `/open` (TS-A3 create-route hook, TS-A4 file input +
  chips); TS-A5 staff-reply attachment hook + thread chips.
- **EPIC-M2-B — Authorized attachment download**: TS-B1 client + staff download routes (stream blob,
  `Content-Disposition`, parent-ticket session auth); **US-M2-3** client downloads the attachment
  (TS-B2 clickable chips).
- **EPIC-M2-C — Variable substitution engine**: TS-C1 `%{token}` engine in `ost_core` (TDD) + M2
  token catalog.
- **EPIC-M2-D — Canned response consumption**: TS-D1 `canned_response` / `canned_attachment` schema +
  seed two samples (one with a variable, one carrying a seeded `.txt`); **US-M2-2** staff replies with
  attachment + canned response (TS-D2 fetch route, TS-D3 dropdown UI, TS-D4 post wiring). **No CRUD UI
  — deferred to M4.**
- **EPIC-M2-E — Real outbound email basics**: TS-E1 SMTP mailer behind the mailer port + Mailpit;
  TS-E2 auto-reply/notice wrappers (anti-loop headers) + packaged-default templates; **US-M2-4**
  client receives the reply by email (TS-E3 wire autoresponse on create + notification on reply).
- **TS-M2-prep** — `--reset` purge flag on the seed binary (test infra; done first for clean E2E
  sweeps).

### Deviations from legacy (pinned)

- **D1 — Filesystem blob store, not chunked-DB.** Blobs written to `var/blobs/aa/bb/<sha256>` keyed by
  content SHA-256 → **real byte-level dedup**; `attachment_file.storage_key` points at the blob; no
  chunk table (obsoletes the legacy FS-022.12 chunk store + KL-022.4 time-salt non-dedup).
- **D2 — Session-on-parent-ticket download auth, not session-bound MD5 hash.** BS-022.8's observable
  behaviour (you can only download an attachment whose parent ticket your session can access) is
  preserved via a plain parent-ticket session check; the legacy `md5(file_id+session_id+file_hash)`
  scheme (KL-022.5) is dropped.
- **D3 — Env-driven single SMTP transport, not per-account/template-set DB config.** One transport
  from `SMTP_HOST/PORT/USER/PASS/FROM`; **SMTP active when `SMTP_HOST` is set**, otherwise the M1 stub
  mailer + `GET /api/dev/mailbox` is retained. Packaged-default templates (FS-040.10) + one seeded
  template set stand in for the template-set admin UI.
- **Documented divergence (D-area):** a posted canned reply keeps the **posting agent** as author
  (legacy BS-022.15 records "SYSTEM (Canned Reply)" — no SYSTEM actor in the M1 model yet); the
  unanswered flag, substitution, and attachment-carry ARE preserved.

### Deferrals out of M2

- Canned-response **CRUD UI** → **M4** (admin); the two seeded responses stand in.
- Per-email-account + template-set **admin UI**, email **filters/banlist** → **M4/M5**.
- Inbound email **fetch/pipe/parse** (POP3/IMAP/MTA) → **M5** (email pipeline).
- Inline image display, download-all, FAQ/logo file types, the chunked-DB store, HTML email → later.

### Mailpit test infrastructure (ports)

| Service | Dev | Staging |
|---------|-----|---------|
| Mailpit SMTP | 3704 | 3714 |
| Mailpit web UI | 3705 | 3715 |

Backend points at Mailpit via `SMTP_HOST=localhost SMTP_PORT=3704` to exercise D3; the Mailpit web UI
(`http://localhost:3705`) is the in-browser oracle for the root E2E "client receives the email" step.

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
| Mailpit SMTP (M2 test infra) | 3704 | 3714 |
| Mailpit web UI (M2 test infra) | 3705 | 3715 |
| PostgreSQL | 5432 (shared `backend-db-1`) | 5432 (shared) |

Registered in `~/.claude/port-registry.md` under `osticket-modernisation`. **3703/3713 are freed** —
Postgres is the shared `backend-db-1` container on 5432 (db `osticket_dev` / `osticket_staging`),
not a project-owned port.
