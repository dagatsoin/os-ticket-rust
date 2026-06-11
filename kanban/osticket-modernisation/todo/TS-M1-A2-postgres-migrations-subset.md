# TS-M1-A2 — add(db): BS-091 PostgreSQL migrations for the M1 schema subset

- **ID**: TS-M1-A2
- **Type**: Technical Story
- **Parent**: EPIC-M1-A
- **Labels**: Technical Story, M1
- **Scope**: small

## Context

Replaces the legacy MySQL install stream with standard Rust/SQLx migrations (decision: this
supersedes FS-060/FS-061 installer + upgrader — see ROADMAP.md). Only the tables the M1 slice
touches are created: ticket, thread/messages, staff, group, department, session, config, plus
the enums/reference sets they need. Later milestones add the rest of the 34-table model
incrementally.

## Impact

- add(db): SQLx migration(s) creating the M1 subset, PostgreSQL dialect:
  - `department`, `sla` (minimal, nullable), **`groups`** (NOT `group` — `group` is a PG reserved
    word) / `group_dept_access`, `staff`, `ticket`, `ticket_thread`
    (message/response/note entries), `ticket_status`/status, `session`, `config` (key/value).
  - **Dual ticket identifiers** (enumerate explicitly): an internal **`ticket_id`** primary key
    (surrogate) AND an external user-visible **`ticketID`** 6-digit number. Add a **UNIQUE
    `(ticketID, email)`** constraint (BS-091.1).
  - Map legacy MySQL types → Postgres (e.g. `tinyint(1)`→`boolean`,
    `int unsigned`→`integer`/`bigint`, `datetime`→`timestamptz`). **Enums are modelled as `text`
    + a `CHECK` constraint** (NOT native PG enum types) so later milestones extend the value set
    without `ALTER TYPE` churn.
- add(infra): `sqlx migrate` wired into app startup or a `make migrate` task. Migrations follow
  the SQLx offline-mode convention (committed `.sqlx/` cache; ROADMAP.md → Decisions → 5).

## Regressions

- None (greenfield). Note for the team: column/enum literals should match FS-091 names so later
  milestones extend cleanly.

## Acceptance Tests

### AC-1: Migrations apply cleanly to osticket_dev in the shared backend-db-1 container. [API-ONLY]
- Setup: shared backend-db-1 Postgres up, `osticket_dev` present.
- Request: `DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_dev sqlx migrate run`.
- Expect: all migrations apply with exit 0, no errors.
- Status: [ ]

### AC-2: Re-running migrations is a safe no-op (idempotency). [API-ONLY]
- Request: run `sqlx migrate run` a SECOND time against the already-migrated `osticket_dev`.
- Expect: no error, no duplicate objects (migration tracking table reports nothing new to apply).
- Status: [ ]

### AC-3: The schema exposes ticket + thread + staff + dept + groups + session + config tables with FS-091-aligned columns. [API-ONLY]
- Request: `psql postgres://postgres:pass123@localhost:5432/osticket_dev -c '\dt'` and `\d ticket`, `\d ticket_thread`, `\d staff`, `\d department`, `\d groups`, `\d session`, `\d config`.
- Expect: all listed tables exist with FS-091-aligned columns; `groups` is named `groups` (not the reserved `group`).
- Status: [ ]

### AC-4: The ticket table exposes both ticket_id PK and the external 6-digit ticketID, with the UNIQUE (ticketID, email) constraint (BS-091.1). [API-ONLY]
- Request: `psql ... -c '\d ticket'` and inspect constraints.
- Verify: surrogate `ticket_id` primary key AND external `ticketID` column both present; a UNIQUE constraint over `(ticketID, email)` exists.
- Request (enforcement): attempt to insert two rows with the same `(ticketID, email)` via psql → the second must fail with a unique-violation.
- Status: [ ]

### AC-5: Enum-valued columns are text + CHECK constraint (no native PG enum types). [API-ONLY]
- Request: `psql ... -c '\dT'` (expect NO custom enum types for status/priority/thread-type) and `\d+` on the relevant tables to see the CHECK constraints.
- Expect: status/priority/thread-entry-type columns are `text` with a `CHECK (... IN (...))` constraint; no native PG `CREATE TYPE ... AS ENUM`.
- Status: [ ]

## Test Infrastructure

- Provides the schema the seed fixture (TS-M1-A3) and all EPIC-M1-B flows write against.
- **Database**: the **existing shared Docker container `backend-db-1`** (`postgres:16`) on host
  port **5432** — database **`osticket_dev`** (already created). The project does **not** start
  its own Postgres (3703/3713 freed). Only the `osticket_dev` / `osticket_staging` databases may
  be touched inside that shared container — never drop/recreate the container or other databases.
- `DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_dev` (dev). Migrations run
  against `osticket_dev`; CI/test may use an ephemeral `osticket_test` DB in the same instance.

## Dependencies

- TS-M1-A1 (workspace exists).
- Existing shared `backend-db-1` Postgres container running on :5432 with `osticket_dev` present.
