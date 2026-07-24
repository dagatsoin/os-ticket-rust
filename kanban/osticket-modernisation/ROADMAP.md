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
| M2 | Attachments, Canned Responses & Email Basics | FS-022, FS-040 | **DONE (2026-07-24)** |
| M3 | Full Staff Workflow & Queue | FS-021, FS-020 | **DONE (2026-07-24)** |
| M4 | Admin Configuration | FS-030/031/032/033, FS-022 admin | **DONE (2026-07-24)** |
| M5 | Email Pipeline | FS-041/042/040 | Backlog stub |
| M6 | External API & Cron | FS-043 | Backlog stub |
| M7 | Knowledge Base & FAQ | FS-050 | Backlog stub |

> **Status note (2026-07-24):** M1, M2, M3 and M4 are all **DONE**. The M2 root E2E (5/5 root ACs
> `[x]`), the M3 root E2E (11/11 root ACs `[x]`) and the M4 root E2E (8/8 Integration ACs `[x]`,
> AC-7 re-tested green after the `log_level` fix) were all **verified green on 2026-07-24**;
> every M1–M4 ticket (roots, epics, user stories, technical stories) is now in `done/`. The M4
> deviations M4-D1 (minimal email_account/template_group model), M4-D2 (minimal write-side logging)
> and M4-D3 (M3 runtime carry-overs deferred to M5) were delivered as planned. **M5, M6 and M7
> remain in the backlog.**

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

## M2 — Attachments, Canned Responses & Email Basics (DONE — 2026-07-24)

**Completed 2026-07-24.** Root E2E verified green (5/5 root ACs `[x]`); all 27 M2 tickets
(1 root + 5 epics + 4 US + 16 TS) shipped to `done/`.

Builds on M1's round-trip so a ticket can carry **attachments**, an agent can answer with a
**canned response** whose body is personalised by a **`%{token}` engine**, and the M1 stub mailer is
replaced by **real outbound email** (autoresponse + reply notification) delivered over SMTP and
observable in **Mailpit**.

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

## M3 — Full Staff Workflow & Queue (DONE — 2026-07-24)

**Completed 2026-07-24.** Root E2E verified green (11/11 root ACs `[x]`); the entire M3 tree
(1 root + 8 epics + 9 US + seed prep + TS) shipped to `done/`.

Transforms the M1 proof-of-concept staff panel into a production-ready agent workspace. Covers the
**full queue experience** (status tabs, visibility scoping, sort/pagination, search, bulk actions)
and the **complete single-ticket workflow** (assign/claim/release/transfer, close/reopen, internal
notes, SLA/overdue, edit/delete, collaborative locking).

**Demo path (root E2E, browser-only):** *an agent navigates queue tabs, searches, claims a ticket,
transfers it, adds an internal note, closes it, bulk-reopens multiple tickets, and observes locking
when another session views the same ticket.*

### Epic split & dependency order

- **EPIC-M3-A — Queue Tabs + Visibility** (FS-020.1-4, 11): status param, Open/Answered/My Tickets/Overdue/Closed
  tabs with counts, department+assignment visibility scoping, rightmost column logic.
- **EPIC-M3-B — Sorting + Pagination** (FS-020.5-6): sortable columns, sticky per-queue sort, pagination with
  limit param. Depends on A.
- **EPIC-M3-C — Close/Reopen/Assign Workflow** (FS-021.7-12): assign/claim/release/transfer, close/reopen.
  Depends on A (visibility), D (SLA for transfer).
- **EPIC-M3-D — SLA + Overdue** (FS-021.13): seed SLA plans, due-date computation, overdue flag + tab.
  Should land early as C/I reference it.
- **EPIC-M3-E — Internal Notes** (FS-021.4): post note endpoint, note with state change. Depends on C
  (close/reopen via note).
- **EPIC-M3-F — Search** (FS-020.7-8): basic keyword search, advanced search dialog. Depends on A, B.
- **EPIC-M3-G — Bulk Actions** (FS-020.9, FS-021.21): mass close/reopen/delete. Depends on A, C.
- **EPIC-M3-I — Edit + Delete + Lock** (FS-021.15, 18, 19): edit properties, delete ticket, collaborative
  locking. Standalone / parallel with D.
- **TS-M3-prep** — seed data expansion: second department (Sales), team (Tier 2), SLA plans (Standard 24h,
  Urgent 4h), help topics (General, Billing), group-dept access, config keys. Blocker for all epics.

### Deferrals out of M3

- Staff-initiated (phone) ticket creation (FS-021.20) -> M4
- PDF print (FS-021.17) -> M4
- Ban/unban requester email (FS-021.14) -> M4 (needs banlist infra)
- Dashboard activity chart and statistics (FS-020.12-13) -> M4
- CSV export (FS-020.10) -> M4
- Manager-only state flags (mark overdue/answered/release) -> M4 (needs dept-manager status)

### Decisions (M3)

1. **Lock time default** — seed `ticket_lock_time = 2` (minutes). The lock table exists in M1 schema.

2. **Visibility model** — department-based (FS-020.4). Agent sees tickets in accessible departments +
   direct assignments + team assignments (open tickets). The `showAssignedOnly` staff flag is M4.

3. **Manager gate deferral** — department-manager status is not implemented in M3; the manual mark-overdue,
   mark-answered, and release-via-note actions are hidden or fail permission. Close/reopen work via the
   `can_close_tickets` group flag.

## M4 — Admin Configuration (DONE — 2026-07-24)

**Completed 2026-07-24.** The M4 root E2E passed — all **8 Integration ACs** (`[BROWSER]`) are
`[x]` (AC-7 re-tested green after the `log_level` fix). All 9 epics and every leaf (14 user stories +
32 technical stories) are in `done/`; the milestone root is now in `done/`. The deviations
**M4-D1** (minimal email_account/template_group model), **M4-D2** (minimal write-side logging) and
**M4-D3** (M3 runtime carry-overs deferred to M5) were delivered as planned. M5/M6/M7 remain backlog.

**Planned 2026-07-24.** Full tree created: **1 root + 9 epics + 14 user stories + 32 technical
stories** — all shipped through `done/`.

Replaces M1's seed-only configuration with a real admin control panel: CRUD for departments, teams,
help topics, staff accounts and permission groups; the seven-tab system settings; SLA plans and
priorities; FAQ categories; site pages; a system-log viewer; and the M2-deferred canned-response
CRUD UI. This is the milestone that makes the product operator-configurable from the browser instead
of via seed SQL.

**Demo path (root E2E, browser-only):** *an admin creates a department that selects an SLA and a
priority, provisions a staff account and a permission group, authors a landing and a thank-you page
and binds them, delegates FAQ-category management to a non-admin via a group flag, and changes a
global setting that visibly takes effect in staff pagination and password aging — while the default
department/SLA and bound pages stay protected from deletion.*

### Epic split & dependency / build order

**Build order: PREP → A → (B ∥ D ∥ F ∥ G ∥ H) → C → E.** C is last among the routing objects
(consumes A, B, D, F); E follows B's `can_manage_faq` gate. The shared admin-panel FE shell
(TS-M4-A0, owned by EPIC-M4-A) is a dependency for every admin screen.

- **EPIC-M4-PREP — Schema & seed expansion** (blocker for all): additive admin columns on
  department/groups/staff; net-new tables `faq_category`, `page`, `syslog`, `timezone`,
  `email_account`, `template_group`; SLA transient-trump key; ~110 config keys seeded to FS-032
  defaults; `default_dept_id`/`default_sla_id`/`*_page_id` bindings. (TS PREP-A/B/C/D)
- **EPIC-M4-A — System Settings** (FS-032.1–.7): admin shell + nav + gate (TS-A0); settings
  GET/PUT + per-tab validation (TS-A1); System/Ticket tabs UI (TS-A2); Email/Pages/Autoresponder/
  Alerts/Attachments tabs UI (TS-A3). US-A1, US-A2.
- **EPIC-M4-B — Staff, Groups & Permissions** (FS-031): staff CRUD + last-admin/self protection +
  add-to-team; group CRUD + 11-flag set (incl. net-new `can_manage_faq`/`can_manage_premade`/
  `can_ban_emails`/`can_view_staff_stats`) + dept-access matrix; own-profile + directory. US-B1/B2/B3.
- **EPIC-M4-C — Departments, Teams & Help Topics** (FS-030): most cross-linked; consumes A, B, D, F.
  Department CRUD + delete re-home + default protection; team CRUD (members-from-profile);
  help-topic routing + one-level nesting + delete-promote. US-C1/C2/C3.
- **EPIC-M4-D — SLA Plans & Priorities** (FS-032.8–.12): SLA CRUD + deletion constraints; priorities
  read-only. US-D1/D2.
- **EPIC-M4-E — FAQ Categories** (FS-032.13–.16): gated by `can_manage_faq` (NOT admin); category
  CRUD + deletion cascade. US-E1.
- **EPIC-M4-F — Site Pages & Content** (FS-033.9–.16): page CRUD + in-use protection + content/config
  AJAX read endpoints. US-F1.
- **EPIC-M4-G — System Logs** (FS-033.1–.8): minimal write-side logging (M4-D2) + viewer
  (filter/sort/paginate/detail/bulk-delete) + purge sweep function (dev/manual trigger). US-G1.
- **EPIC-M4-H — Canned Response CRUD UI** (FS-022 admin surface): gated by `can_manage_premade`;
  model reused from M2. US-H1.

### Cross-epic Integration ACs (at the M4 milestone root — all [BROWSER])

1. **A→C/D**: a department can select an SLA (D) and priority; default dept/SLA delete-protected.
2. **B→C**: staff from B usable as dept manager / team lead / topic auto-assign; B's group appears in the dept access matrix.
3. **B↔C**: team member added from B's staff profile; removing staff clears membership.
4. **A↔F**: a page from F is bindable as landing/offline/thank-you in A; binding makes it in-use-protected in F.
5. **F→C**: a thank-you page is selectable on a help topic.
6. **B→E**: `can_manage_faq` on a non-admin group grants FAQ-category access without settings access.
7. **A→wide**: page-size/login-window/default keys written in A take effect in B pagination & password aging.
8. **D1 gate**: department Email/Template selects resolve against the minimal email_account/template_group model.

### Deviations from legacy (pinned — M4)

- **M4-D1 (RISK-1) — Minimal email_account + template_group model forward-ported into EPIC-M4-PREP.**
  The department Email/Template required selects (FS-030.4, BS-030-02/03) and the FS-032 Emails tab
  resolve against **real rows**, not system-default stubs. M5/FS-040 extends this model. (PREP-B/C/D)
- **M4-D2 (RISK-3) — Minimal write-side logging facility built in EPIC-M4-G.** An
  `osTicket::log()`-equivalent records `syslog` rows on notable admin/system events so the FS-033
  viewer shows real data. Full FS-003 logging is NOT required. The purge sweep **function** lands in
  M4-G (dev/manual trigger); the cron purge **trigger** is a noted **M6** forward-dependency. (G1/G2)
- **M4-D3 — M3 runtime carry-overs deferred to M5, NOT M4**: PDF print (FS-021.17), CSV export
  (FS-020.10), dashboard stats/activity chart (FS-020.12/.13), phone/staff-initiated ticket create
  (FS-021.20), ban/unban (FS-021.14). Dept-manager status IS included in M4-B (isadmin + dept
  manager pointer + effective-access rule), unblocking the M3-deferred manager-gated flags for M5.

### KL treatment (M4)

- **Modernised** (correct behaviour, each a deliberate deviation): KL-030-11 (no-change dept/team
  update guard), KL-030-12 (team "Last Updated" sort), KL-031-002, KL-032.3 (`send_sys_errors`
  stored value), KL-032.4 (SLA error text), KL-032.9 (transfer-alert error), KL-032.10 (SLA "Date
  Added" sort), KL-032.11 (`Category::lookup` row check), KL-033.2, KL-033.7 (purge).
- **Preserved** (faithful 1.7): one-level topic nesting (KL-030-02), single-group-per-staff
  (KL-031-001), fixed priority set / no priority CRUD (KL-032.1), no page versioning (KL-033.1).

### Decisions (M4) — to be firmed during consolidation

1. **Admin FE shell** — a single `/staff/admin/*` branch + `AdminLayout` + capability-aware nav
   (TS-M4-A0), owned by EPIC-M4-A and reused by every admin screen. Delegated screens (FAQ, Canned)
   declare a capability gate instead of the admin gate.
2. **`admin` seed account** — `admin` / `Admin123!` (isadmin) added by the M4 seed for browser E2E,
   alongside the existing `agent`. Supersedes the M1 seed as the source of dept/group/staff once M4 ships.
3. **Dept-manager status** — included in EPIC-M4-B (small); may be trimmed to the manager pointer if
   non-trivial (decision recorded on EPIC-M4-B).

## Decisions (M2 cross-cutting — PINNED)

Pinned during the M2 consolidation feasibility review. Binding for all M2 tickets; affected
tickets reference this section by number.

1. **Blob root** — env `BLOB_ROOT`, default `<workspace root>/var/blobs`, resolved to an absolute
   path at startup; all binaries and tests honor it. (A1, A3, A5, B1, prep)

2. **Multipart strategy** — enable the axum `multipart` feature. `POST /api/tickets` and
   `POST /api/staff/tickets/:id/reply` **DUAL-ACCEPT by Content-Type**: `application/json` (the M1
   contract, unchanged, no attachment) OR `multipart/form-data` (fields as individual form parts —
   `name`/`email`/`subject`/`message` for create, `body`/`cannedId` for reply, plus an optional
   `attachment` file part). The 422 field key for file errors is **`attachment`**. (A3, A5, D4, A4)

3. **isanswered semantics (M2)** — legacy-faithful: a client message (create) → `isanswered=false`;
   **ANY staff reply (plain or canned-assisted) → `isanswered=true`**. BS-022.15's "mark unanswered"
   applies to the *filter-driven SYSTEM auto-reply path* → **DEFERRED to M5** (documented deviation).
   The staff UI shows an Answered / Unanswered badge on both detail and queue. (D4, US-M2-2, C4-adjacent UI)

4. **Crates** — SMTP = **`lettre`** (tokio1 + rustls, custom headers); new deps **`sha2`**, **`hex`**,
   **`tokio-util`** (io feature); **`reqwest` as a DEV-dependency** for Mailpit API assertions in tests.
   (A1, B1, E1, E2)

5. **Mailpit lifecycle** — add a `docker-compose.yml` at the repo root (service `mailpit`, image
   `axllent/mailpit`, ports `3704:1025` + `3705:8025`); document `docker compose up -d mailpit` in
   CLAUDE.md Quick Start. Mailpit-dependent cargo tests are **skip-pass when `MAILPIT_URL` is unset**
   (same pattern as `TEST_DATABASE_URL`). (E1, E2, E3)

6. **helpdesk_url** — seed config key `helpdesk_url` = `http://localhost:3702`, seeded by **TS-M2-A2**
   alongside the attachment config keys. The substitution engine takes the base URL as an input
   argument; the wiring reads the `helpdesk_url` config key and feeds it in. (C1, E3, A2)

7. **Attachment list JSON key** — `attachments: [{id, name, size, mime}]` — the **SAME shape** in
   staff detail entries, client thread entries, and canned detail responses. (Resolves the prior
   A5-vs-D2 `files` inconsistency.) (A5, D2, B2, D3)

8. **Client download route** — session-bound, **no ticketId param**:
   `GET /api/client/ticket/attachments/{attachmentId}` (the session already pins the ticket). Staff:
   `GET /api/staff/tickets/{ticketId}/attachments/{attachmentId}`. Cross-ticket / unknown id → **404**
   (no existence leak). (B1, B2)

9. **Download UX** — chips use **fetch-with-credentials → blob → object-URL download** so a 403/404
   surfaces a VISIBLE inline error (US-M2-3 AC-3); not plain anchors. (B2)

10. **Reply-composer ownership de-dup** — **TS-M2-A5 = BACKEND only** (reply multipart hook +
    attachments in thread payloads); **TS-M2-D3 = the ENTIRE reply composer UI** (canned dropdown +
    file input + read-only carried chips + the retained `cannedId` in the multipart POST);
    **TS-M2-B2 = chips rendering + click-to-download** in BOTH the client portal and staff detail.
    (A5, D3, B2)

11. **A4 specifics** — the confirmation chip label comes from the locally-selected `File.name` (no API
    echo needed); client-side validation hard-codes the seeded allow-list / 1 MB as a UX pre-check
    (the backend remains authoritative — noted as a KL on A4). (A4)

12. **E3 always-send in M2** — the autoresponse + reply notification are **always sent** in M2;
    department auto-response flags are honored in M5. (E3)

13. **apiClient FormData support (A0)** — `frontend/src/api/apiClient.ts` is extended so that when
    `body instanceof FormData` it skips `JSON.stringify` and omits the `Content-Type` header (the
    browser sets the multipart boundary), while preserving the CSRF header, error-envelope parsing,
    and the 401 → realm-login redirect. The MSW harness gains multipart-capable handlers
    (`request.formData()`). This is **TS-M2-A0**, a blocker for A4, the D4 reply path (via D3), and D3.

14. **Shared frontend primitives ownership** — the **AttachmentChip presentational shell**
    (`{label, icon, onClick?, readOnlyMarker?}` — render-only, no fetch) + the client-side
    **`validateAttachment(file)` pre-check helper** are OWNED by **TS-M2-A4**; **TS-M2-B2** injects the
    download `onClick` (fetch→blob→objectURL per §9); **TS-M2-D3** injects the readOnly / "from canned
    response" marker and reuses `validateAttachment` for its own-file input. (A4, B2, D3)

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
