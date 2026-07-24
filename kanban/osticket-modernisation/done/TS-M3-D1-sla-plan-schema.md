# TS-M3-D1 — verify(schema): BS-091.6 sla table (already exists in M1)

- **ID**: TS-M3-D1
- **Type**: Technical Story
- **Parent**: US-M3-D1
- **Labels**: Technical Story, M3
- **Scope**: small

## Context

This technical story verifies that the `sla` table exists with the required columns for M3.
The SLA plan defines a grace period (hours) after which a ticket becomes overdue.

**Note**: The M1 schema (migration `0001_m1_schema_subset.sql`) already created the `sla` table
(FS-091.6, entity 12) with all required columns, and the `ticket.sla_id`, `ticket.duedate`,
`ticket.isoverdue` columns. This ticket is effectively a **no-op verification** that the
schema is ready for M3 SLA features.

The table is named `sla` (not `sla_plan`) per FS-091 entity 12 naming.

## Impact

- verify(schema): FS-091.6 `sla` table exists with columns:
  - `id` (primary key)
  - `name` (text, unique, required)
  - `grace_period` (integer, hours, required — default 48)
  - `isactive` (boolean, default true)
  - `enable_priority_escalation` (boolean, default true)
  - `disable_overdue_alerts` (boolean, default false)
  - `notes` (text, nullable)
  - `created` (timestamp)
  - `updated` (timestamp)
- verify(schema): BS-032.10 `ticket.duedate` column exists (timestamp, nullable)
- verify(schema): BS-032.10 `ticket.sla_id` column exists (FK to sla, nullable)
- verify(schema): `ticket.isoverdue` column exists (boolean, default false)

## Regressions

- None — this is a verification ticket, no schema changes.

## Acceptance Tests

### AC-1: FS-091.6 — sla table exists with required columns. [API-ONLY]
- Setup: Connect to database via psql or TEST_DATABASE_URL
- Request: SELECT id, name, grace_period, isactive, enable_priority_escalation, disable_overdue_alerts, notes FROM sla LIMIT 1
- Expect: Query succeeds (table and columns exist); no SQL error
- Status: [x]

### AC-2: BS-032.10 — ticket table has duedate and sla_id columns. [API-ONLY]
- Setup: Connect to database via psql or TEST_DATABASE_URL
- Request: SELECT duedate, sla_id FROM ticket LIMIT 1
- Expect: Query succeeds (columns exist, nullable); no SQL error
- Status: [x]

### AC-3: BS-091.6 — sla.name is unique. [API-ONLY]
- Setup: Connect to database via psql or TEST_DATABASE_URL
- Request: INSERT INTO sla (name, grace_period) VALUES ('Test SLA Unique', 8)
- Request: INSERT INTO sla (name, grace_period) VALUES ('Test SLA Unique', 16)
- Expect: Second insert fails with unique constraint violation
- Cleanup: DELETE FROM sla WHERE name = 'Test SLA Unique'
- Status: [x]

## Test Infrastructure

- Uses TEST_DATABASE_URL for direct database access during schema verification.
- No dev endpoints required (schema verification only).

## Dependencies

- **M1 done**: base schema exists (migration 0001 created sla table and ticket columns).
