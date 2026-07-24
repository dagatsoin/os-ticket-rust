# TS-M3-D2 — add(seed): FS-091.6 SLA plans (Standard 24h, Urgent 4h, Default 48h)

- **ID**: TS-M3-D2
- **Type**: Technical Story
- **Parent**: US-M3-D1
- **Labels**: Technical Story, M3
- **Scope**: small

## Context

This ticket **owns all SLA plan seeding** for M3. It seeds three SLA plans and assigns them
to departments/topics for the M3 overdue and queue tests.

**Note**: The `sla` table (not `sla_plan`) was created by M1 migration 0001. This ticket
adds seed data to that existing table.

## Impact

- add(seed): FS-091.6 three SLA plans in the `sla` table:
  - "Default SLA" — 48h grace_period, active, priority escalation enabled (per FS-091.6)
  - "Standard" — 24h grace_period, active
  - "Urgent" — 4h grace_period, active
- update(seed): assign default SLA to system default config key `default_sla_id`
- update(seed): assign "Standard" SLA to department "Support"
- update(seed): assign "Urgent" SLA to help topic "Billing" (created by TS-M3-prep)

## Regressions

- The seed must be idempotent (upsert by name).
- Existing seed fixtures (agent, dept, group) must not break.

## Acceptance Tests

### AC-1: FS-091.6 — Three SLA plans are seeded in the `sla` table. [API-ONLY]
- Setup: Run `cargo run -p tools --bin seed`
- Request: SELECT name, grace_period FROM sla ORDER BY grace_period
- Expect: Rows: "Urgent" (4), "Standard" (24), "Default SLA" (48)
- Status: [x]

### AC-2: FS-091.6 — Default SLA is assigned to system default. [API-ONLY]
- Setup: Run seed if not already seeded
- Request: SELECT s.name FROM config c JOIN sla s ON c.value::int = s.id WHERE c.key='default_sla_id'
- Expect: name = "Default SLA"
- Status: [x]

### AC-3: FS-021.13 — Support department has "Standard" SLA. [API-ONLY]
- Setup: Run seed if not already seeded
- Request: SELECT s.name FROM department d JOIN sla s ON d.sla_id = s.id WHERE d.dept_name='Support'
- Expect: name = "Standard"
- Status: [x]

### AC-4: FS-021.13 — Billing help topic has "Urgent" SLA. [API-ONLY]
- Setup: Run seed if not already seeded
- Request: SELECT s.name FROM help_topic h JOIN sla s ON h.sla_id = s.id WHERE h.topic='Billing'
- Expect: name = "Urgent"
- Status: [x]

### AC-5: Idempotent seed. [API-ONLY]
- Setup: Run `cargo run -p tools --bin seed` twice
- Request: SELECT COUNT(*) FROM sla
- Expect: No errors during seed runs; count = 3 (not 6)
- Status: [x]

## Test Infrastructure

- Uses `cargo run -p tools --bin seed` for seeding.
- Uses TEST_DATABASE_URL for direct database verification.
- No dev endpoints required (uses seed binary).

## Dependencies

- **TS-M3-D1**: verifies `sla` table exists (created by M1 migration 0001).
- **TS-M3-prep**: creates help topics ("Billing" topic) — must run first so this ticket can assign SLA to it.
