# TS-M4-B1 — add(route): FS-031.3/.4/.5 staff CRUD + validation + protections + add-to-team

- **ID**: TS-M4-B1
- **Type**: Technical Story
- **Parent**: US-M4-B1
- **Labels**: Technical Story, M4
- **Scope**: medium

## Context

Backend for staff management: list/filter/sort/paginate, create/edit with validation, mass
enable/lock/delete with the last-admin and self-action safeguards, and the add-to-team-from-profile
operation (BS-030-14). Also seeds the `admin` account used across M4 browser ACs.

## Impact

- add(migration): additive `ALTER TABLE staff ADD COLUMN mobile text NOT NULL DEFAULT ''`
  (**PREP missed it** — verified: `migrations/0011_m4_prep_additive_columns.sql` adds `max_page_size`
  etc. but no `mobile`). Required by the staff create/edit form (mobile field) and the directory
  columns (TS-M4-B5/B6 list `mobile`). New forward migration, no backfill needed (default `''`).
- fix(queue): repoint the queue page-size read from the **non-existent** `page_limit` column to
  `max_page_size`. Verified: `resolve_page_size` in `backend/api/src/staff.rs` (~line 1073) runs
  `SELECT page_limit FROM staff …`, but the staff table only has `max_page_size` (migration 0011,
  line 104); `page_limit` was never created. This read currently mis-resolves; switch the column
  to `max_page_size` (keep the `>0` / clamp semantics). Regression-covered below.
- add(route): `GET /api/staff/admin/staff` (list: filter `q`, sort keys, pagination) — admin-gated.
- add(route): `POST /api/staff/admin/staff` (create), `PUT /api/staff/admin/staff/:id` (edit) —
  validation per FS-031.5 / BS-031-001 / BS-031-013; argon2id hashing via the TS-M1-A4a util.
  Email uniqueness is enforced **app-level, case-insensitive, excluding self** (`lower(email)` compare,
  ignore the row being edited) → 422 field error on `email` (BS-031-001); username uniqueness likewise.
- add(route): `POST /api/staff/admin/staff/mass` (enable/lock/delete) — BS-031-014 last-admin guard,
  BS-031-015 self-action guard, BS-031-016 deletion side effects (clear assignments/membership).
- add(route): `POST /api/staff/admin/staff/:id/teams` + `DELETE …/teams/:teamId` — add/remove team
  membership from the profile (BS-030-14).
- add(seed): `admin` / `Admin123!` isadmin account.
- add(dev): `POST /api/dev/reset-staff`, `POST /api/dev/age-password` (test infra).

## Regressions

- Existing `agent` login + effective-access (BS-031-005) unaffected; new columns nullable/defaulted.
- **Queue page size** (`/staff/tickets`): after repointing `resolve_page_size` to `max_page_size`,
  verify the staff queue still paginates (personal page-size honoured when `max_page_size > 0`,
  falls back to the `default_page_size` config then the 25 default). This touches the M1/M3 queue
  read path — regression-test the queue list still returns the right page count.
- The additive `mobile` column defaults to `''`; existing staff rows and the `agent` login are
  unaffected.

## Acceptance Tests

### AC-1: FS-031.2 — list supports filter, sort, pagination. [API-ONLY]
- Setup: `POST /api/dev/reset-staff`; authenticate as `admin` / `Admin123!` (session cookie + CSRF).
- Request: `GET /api/staff/admin/staff?q=age&sort=name&page=1` (admin) → 200, filtered to matching rows, name-sorted, paged (page/total metadata present).
- Request: same endpoint authenticated as `agent` (non-admin) → 403.
- Status: [x]

### AC-2: BS-031-001/013 — create validation. [API-ONLY]
- Setup: authenticate as `admin`.
- Request: `POST /api/staff/admin/staff` with username `agent` (existing) → 422 unique field error on `username`.
- Request: `POST /api/staff/admin/staff` with an email that equals an existing staff email in a **different case**
  (e.g. `AGENT@osticket.local` when `agent@osticket.local` exists) → 422 unique field error on `email`
  (case-insensitive uniqueness).
- Request: `PUT /api/staff/admin/staff/:id` re-saving a staff row with its OWN unchanged email → 200
  (self is excluded from the uniqueness check).
- Request: `POST …/staff` with a 4-char password → 422 "Must be at least 6 characters".
- Request: `POST …/staff` with password/confirm mismatch → 422 confirmation error.
- Status: [x]

### AC-3: BS-031-014 — cannot demote/lock the only active admin (edit form). [API-ONLY]
- Setup: `POST /api/dev/reset-staff` (exactly one active admin `admin`); authenticate as `admin`.
- Request: `PUT /api/staff/admin/staff/:adminId` clearing isadmin → 422 "Cowardly refusing to remove or lock out the only active administrator".
- Request: `PUT …/:adminId` setting status Locked → 422 same message.
- Status: [x]

### AC-4: BS-031-015 — self-action blocked in mass action. [API-ONLY]
- Setup: authenticate as `admin`.
- Request: `POST /api/staff/admin/staff/mass` {action: lock, ids: [<admin's own id>, …]} → rejected with "You can not disable/delete yourself - you could be the only admin!". Repeat with action delete → same block.
- Status: [x]

### AC-5: BS-030-14 — add-to-team from profile, and removal clears membership. [API-ONLY]
- Setup: authenticate as `admin`; a team + a staff member exist (seed or `POST /api/dev/seed-staff`).
- Request: `POST /api/staff/admin/staff/:id/teams` {teamId} → 200; team_member row created (verify via the team roster / GET).
- Request: `DELETE /api/staff/admin/staff/:id/teams/:teamId` → 200; row removed.
- Status: [x]

### AC-6: BS-031-016 — deleting staff clears their associations. [API-ONLY]
- Setup: authenticate as `admin`; create a staff member, assign them an open ticket + a team membership.
- Request: `POST /api/staff/admin/staff/mass` {action: delete, ids:[<that id>]} → 200; the open-ticket assignment is cleared (unassigned) and the team membership row is removed; the staff row is gone.
- Status: [x]

## Test Infrastructure

- Seeds `admin`; dev endpoints `reset-staff`, `age-password`. `.sqlx` cache updated.
- Owns the additive `mobile`-column migration and the `page_limit`→`max_page_size` queue fix
  (see Impact); after the migration, regenerate `.sqlx` for any query that now selects `mobile`.
- **Note (ownership split):** the extended/bulk `seed-staff` used by the list pagination/filter ACs
  is owned by **TS-M4-B3** (`seed-staff` groupId/deptId/isvisible/isadmin/onvacation/count, or a
  `seed-staff-bulk`). TS-M4-B1 only relies on it being present for its own AC-1 pagination setup.

## Dependencies

- **EPIC-M4-PREP**: staff/groups additive columns.
- **TS-M4-A0**: admin gate. **M1**: argon2id util, session realm.
