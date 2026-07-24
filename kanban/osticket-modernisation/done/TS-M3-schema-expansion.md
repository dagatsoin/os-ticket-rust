# TS-M3-schema — add(schema): M3 schema expansion migration

- **ID**: TS-M3-schema
- **Type**: Technical Story
- **Parent**: M3 (cross-cutting, not under a single epic)
- **Labels**: Technical Story, M3
- **Scope**: small

## Context

M3 introduces several features that require schema additions beyond the M1/M2 baseline.
This ticket creates migration `0005_m3_schema_expansion.sql` with all the structural
changes needed before the seed expansion (TS-M3-prep) can run.

The M1 schema already includes:
- `sla` table (FS-091.6, entity 12) with all required columns
- `ticket.sla_id`, `ticket.duedate`, `ticket.isoverdue` columns
- `groups` table with `can_create_tickets`, `can_post_reply`

This migration adds the missing M3 structures.

## Impact

- add(schema): `team` table (FS-091, entity 9)
  - `team_id` (integer, primary key, identity)
  - `name` (text, unique, required)
  - `isenabled` (boolean, default true)
  - `lead_id` (integer, nullable, FK to staff)
  - `noalerts` (boolean, default false)
  - `notes` (text, nullable)
  - `created` (timestamptz, default now)
  - `updated` (timestamptz, default now)

- add(schema): `team_member` table (FS-091, entity 10)
  - `team_id` (integer, FK to team, on delete cascade)
  - `staff_id` (integer, FK to staff, on delete cascade)
  - `updated` (timestamptz, default now)
  - Primary key: (team_id, staff_id)

- add(schema): `help_topic` table (FS-091, entity 11)
  - `topic_id` (integer, primary key, identity)
  - `topic_pid` (integer, default 0) -- parent topic for nesting
  - `topic` (text, required) -- display name
  - `isactive` (boolean, default true)
  - `ispublic` (boolean, default true)
  - `noautoresp` (boolean, default false)
  - `priority_id` (integer, default 2) -- normal priority
  - `dept_id` (integer, FK to department, nullable)
  - `staff_id` (integer, FK to staff, nullable) -- auto-assign
  - `team_id` (integer, FK to team, nullable) -- auto-assign
  - `sla_id` (integer, FK to sla, nullable)
  - `page_id` (integer, default 0) -- custom form page
  - `sort` (integer, default 0)
  - `notes` (text, nullable)
  - `created` (timestamptz, default now)
  - `updated` (timestamptz, default now)
  - Unique constraint: (topic, topic_pid)

- alter(staff): add `show_assigned_only` column
  - `show_assigned_only` (boolean, default false)
  - Per-staff visibility override: when true, staff sees only tickets assigned to them

- alter(ticket): add `closed_by_staff_id` column
  - `closed_by_staff_id` (integer, nullable, FK to staff)
  - Records which staff member closed the ticket (for Closed By column in queue)

- alter(groups): add M3 permission flags
  - `can_close_tickets` (boolean, default false)
  - `can_delete_tickets` (boolean, default false)
  - `can_assign_tickets` (boolean, default false)
  - `can_transfer_tickets` (boolean, default false)
  - `can_edit_tickets` (boolean, default false)

- add(schema): `ticket_lock` table (FS-021.18, collaborative locking)
  - `lock_id` (integer, primary key, identity)
  - `ticket_id` (integer, unique, FK to ticket on delete cascade)
  - `staff_id` (integer, FK to staff on delete cascade)
  - `expire` (timestamptz, required) -- lock expiration time
  - `created` (timestamptz, default now)

## Regressions

- All alterations are additive with sensible defaults; existing queries continue to work.
- The team foreign keys in ticket and help_topic reference the new team table.

## Test Infrastructure

- **Database**: `osticket_test` via `TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test`
- **Dev endpoint needed**: None (schema tests run directly against DB via SQLx)

## Acceptance Tests

### AC-1: FS-091 entity 9 — team table exists with required columns. [API-ONLY]
- Setup: `TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test sqlx migrate run --source migrations`
- Request: `psql -U postgres -d osticket_test -c "SELECT team_id, name, isenabled, lead_id, noalerts, notes, created, updated FROM team LIMIT 0"`
- Expect: 200, query succeeds (table and all columns exist)
- Status: [x]

### AC-2: FS-091 entity 10 — team_member junction table exists with updated column. [API-ONLY]
- Setup: migration applied (from AC-1)
- Request: `psql -U postgres -d osticket_test -c "SELECT team_id, staff_id, updated FROM team_member LIMIT 0"`
- Expect: 200, query succeeds (table and columns exist)
- Status: [x]

### AC-3: FS-091 entity 11 — help_topic table exists with required columns. [API-ONLY]
- Setup: migration applied (from AC-1)
- Request: `psql -U postgres -d osticket_test -c "SELECT topic_id, topic_pid, topic, isactive, ispublic, noautoresp, priority_id, dept_id, staff_id, team_id, sla_id, page_id, sort, notes FROM help_topic LIMIT 0"`
- Expect: 200, query succeeds (table and all columns exist)
- Status: [x]

### AC-4: BS-091 — help_topic unique constraint on (topic, topic_pid). [API-ONLY]
- Setup: `psql -U postgres -d osticket_test -c "INSERT INTO help_topic (topic, topic_pid) VALUES ('Test', 0)"`
- Request: `psql -U postgres -d osticket_test -c "INSERT INTO help_topic (topic, topic_pid) VALUES ('Test', 0)"`
- Expect: ERROR, unique constraint violation on (topic, topic_pid)
- Status: [x]

### AC-5: staff.show_assigned_only column exists. [API-ONLY]
- Setup: migration applied (from AC-1)
- Request: `psql -U postgres -d osticket_test -c "SELECT show_assigned_only FROM staff LIMIT 0"`
- Expect: 200, query succeeds, column exists (boolean type)
- Status: [x]

### AC-6: ticket.closed_by_staff_id column exists. [API-ONLY]
- Setup: migration applied (from AC-1)
- Request: `psql -U postgres -d osticket_test -c "SELECT closed_by_staff_id FROM ticket LIMIT 0"`
- Expect: 200, query succeeds, column exists (nullable integer)
- Status: [x]

### AC-7: groups M3 permission flags exist. [API-ONLY]
- Setup: migration applied (from AC-1)
- Request: `psql -U postgres -d osticket_test -c "SELECT can_close_tickets, can_delete_tickets, can_assign_tickets, can_transfer_tickets, can_edit_tickets FROM groups LIMIT 0"`
- Expect: 200, query succeeds, all 5 boolean columns exist
- Status: [x]

### AC-8: team.name has unique constraint. [API-ONLY]
- Setup: `psql -U postgres -d osticket_test -c "INSERT INTO team (name) VALUES ('Tier 2')"`
- Request: `psql -U postgres -d osticket_test -c "INSERT INTO team (name) VALUES ('Tier 2')"`
- Expect: ERROR, unique constraint violation on name
- Status: [x]

### AC-9: FS-021.18 — ticket_lock table exists with required columns. [API-ONLY]
- Setup: migration applied (from AC-1)
- Request: `psql -U postgres -d osticket_test -c "SELECT lock_id, ticket_id, staff_id, expire, created FROM ticket_lock LIMIT 0"`
- Expect: 200, query succeeds (table and all columns exist)
- Status: [x]

### AC-10: FS-021.18 — ticket_lock.ticket_id has unique constraint. [API-ONLY]
- Setup: insert ticket and staff records, then `psql -c "INSERT INTO ticket_lock (ticket_id, staff_id, expire) VALUES (1, 1, NOW() + interval '2 minutes')"`
- Request: `psql -U postgres -d osticket_test -c "INSERT INTO ticket_lock (ticket_id, staff_id, expire) VALUES (1, 2, NOW() + interval '2 minutes')"`
- Expect: ERROR, unique constraint violation on ticket_id (only one lock per ticket)
- Status: [x]

## Dependencies

- **M1/M2 done**: base schema exists with department, staff, groups, ticket, sla tables.
- **Migrations infrastructure**: uses SQLx migrations at workspace root.

## Blockers

- This ticket **blocks** TS-M3-prep (seed expansion requires these tables/columns).
- This ticket **blocks** TS-M3-A3 (closed_by_staff_id column needed for Closed By display).
