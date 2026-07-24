# TS-M4-PREP-B — add(schema): FS-091 net-new admin tables + SLA transient trump key

- **ID**: TS-M4-PREP-B
- **Type**: Technical Story
- **Parent**: EPIC-M4-PREP
- **Labels**: Technical Story, M4
- **Scope**: small

## Context

Six tables required by M4 do not exist yet. This TS creates them as an additive migration, plus the
SLA transient-trump key column that supports the FS-021.13 explicit-trump precedence slot.

## Impact

- add(schema): `faq_category` (id, name, ispublic, description, notes, created, updated) — FS-032.13/.16.
- add(schema): `page` (id, name unique, type CHECK `landing|offline|thank-you|other`, body, isactive,
  notes, created, updated) — FS-033.11–.16.
- add(schema): `syslog` (id, log_type CHECK `Error|Warning|Debug`, title, log, ip_address, created) —
  FS-033.1–.8, DEVIATION M4-D2.
- add(schema): `timezone` (id, offset, name/label, dst flag) — FS-031.11 preference reference set.
- add(schema): `email_account` (id, email, name, active, created, updated) — MINIMAL, DEVIATION M4-D1
  (M5/FS-040 extends with fetch/SMTP columns).
- add(schema): `template_group` (id, name, isactive, notes) — MINIMAL, DEVIATION M4-D1.
- add(schema): `sla_plan.transient` flag / trump-support column (FS-021.13 explicit-trump slot).
- All CHECK-based enum columns (no native PG enum types), per project convention.

## Regressions

- Additive only; no existing table altered except `sla_plan` (one nullable/defaulted column).
- `help_topic.page_id` (thank-you) already exists from M3 — FK/constraint tightened, not re-created.

## Implementation note — the SLA table is named `sla` (not `sla_plan`)

M1 created the table as `sla` (`0001_m1_schema_subset.sql`), and `0006` added
`department.sla_id REFERENCES sla(id)`. The Impact/ACs below say "sla_plan" for FS parity, but
the transient/trump column must be added to the **existing `sla`** table. Reconcile the name in
the migration (add to `sla`), and use `ADD COLUMN IF NOT EXISTS transient boolean NOT NULL DEFAULT false`.

## Acceptance Tests

### AC-1: FS-091 — the six net-new tables exist. [API-ONLY]
- Setup: apply migrations (`DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_dev sqlx migrate run --source migrations`)
- Request: `psql -U postgres -d osticket_dev -c "SELECT to_regclass('faq_category'), to_regclass('page'), to_regclass('syslog'), to_regclass('timezone'), to_regclass('email_account'), to_regclass('template_group')"`
- Expect: all six columns resolve non-null (no `NULL` = table missing).
- Verify (columns): `psql -U postgres -d osticket_dev -c "SELECT column_name FROM information_schema.columns WHERE table_name='page' ORDER BY column_name"` includes `type`, `body`, `isactive`, `name`.
- Status: [x]

### AC-2: FS-033 — page.type and syslog.log_type CHECK constraints reject invalid values. [API-ONLY]
- Request: `psql -U postgres -d osticket_dev -c "INSERT INTO page(name,type,body) VALUES('x','bogus','y')"`
- Expect: `ERROR:  new row ... violates check constraint` (non-zero exit).
- Request: `psql -U postgres -d osticket_dev -c "INSERT INTO page(name,type,body) VALUES('ok','landing','y')"`
- Expect: succeeds (a valid enum literal is accepted) — then clean up: `DELETE FROM page WHERE name='ok'`.
- Request: `psql -U postgres -d osticket_dev -c "INSERT INTO syslog(log_type,title,log) VALUES('Bogus','t','l')"`
- Expect: CHECK violation error.
- Request: `psql -U postgres -d osticket_dev -c "INSERT INTO syslog(log_type,title,log) VALUES('Error','t','l')"`
- Expect: succeeds — then `DELETE FROM syslog WHERE title='t'`.
- Status: [x]

### AC-3: FS-021.13 — the SLA transient/trump column exists on the `sla` table, default false. [API-ONLY]
- Request: `psql -U postgres -d osticket_dev -c "SELECT column_name, data_type, column_default FROM information_schema.columns WHERE table_name='sla' AND column_name='transient'"`
- Expect: one row, `data_type=boolean`, `column_default=false`.
- Verify: `psql -U postgres -d osticket_dev -c "SELECT transient FROM sla LIMIT 1"` succeeds and returns `f` for the seeded SLA.
- Status: [x]

### AC-4: Migration is idempotent. [API-ONLY]
- Setup: run `DATABASE_URL=... sqlx migrate run --source migrations` twice
- Expect: second run applies 0 migrations, exits 0, no "relation already exists" / "column already exists" error.
- Status: [x]

## Test Infrastructure

- DB-backed test reads `TEST_DATABASE_URL` (skip-pass when unset).
- Downstream dev-endpoint dependency (flag for consuming epics): admin-CRUD E2Es on these tables
  (M4-E FAQ, M4-F pages, M4-G logs) create/edit/delete rows, but the current `seed --reset` only
  purges ticket-scoped data. These tables need either a reset path or dev seed endpoints — see the
  epic report's "dev-endpoint gaps" list. `syslog` in particular has no writer yet: M4-G's log
  viewer needs seeded/generated rows to display.

## Dependencies

- **M1/M3 schema (done)**: `sla_plan`, `help_topic` tables.
- Blocks: TS-M4-PREP-A (FK targets), TS-M4-PREP-C/D (seed targets), and all consuming epics.
