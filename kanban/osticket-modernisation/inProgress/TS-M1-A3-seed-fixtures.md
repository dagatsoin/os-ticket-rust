# TS-M1-A3 — add(db): BS-091 seed fixtures (dept + group + staff) + reference enums

- **ID**: TS-M1-A3
- **Type**: Technical Story
- **Parent**: EPIC-M1-A
- **Labels**: Technical Story, M1
- **Scope**: small

## Context

The M1 slice has no admin CRUD yet, so the one department, one permission group, and one staff
account that the flows need are seeded by migration/fixture. This is the modernised stand-in
for the legacy installer's default-data seeding (FS-060) and first-run admin bootstrap.

## Impact

- add(tools): seed via a **Rust task** (e.g. `cargo run -p tools --bin seed`) using **idempotent
  upserts** — NOT a pure-SQL migration. (argon2id hashes cannot be generated in SQL, so seeding
  the staff password requires the TS-M1-A4a hashing util in Rust.) Inserts:
  - one `department` (e.g. "Support").
  - one permission `groups` row granting exactly the M1 flags: **`can_create_tickets`** +
    **`can_post_reply`** + **access to the seeded department**.
  - one `staff` account linked to the group + dept, with an **argon2id-hashed** password:
    username **`agent`**, password **`Agent123!`**.
  - the reference rows / config defaults the M1 flows read: status literal **`open`**, priority
    literal **`normal`** (FS-091 literals).
- add(infra): document the seeded staff credentials (`agent` / `Agent123!`) for QA in the ticket
  + **CLAUDE.md Quick Start**.

## Regressions

- None (greenfield).

## Acceptance Tests

### AC-1: After seeding, exactly one department, one group, and one staff account (agent) exist. [API-ONLY]
- Setup: `cargo run -p tools --bin seed` (DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_dev).
- Request: `psql ... -c 'select count(*) from department; select count(*) from groups; select count(*) from staff;'`.
- Expect: 1 department, 1 group, 1 staff (username `agent`).
- Status: [ ]

### AC-2: The seeded staff argon2id hash verifies against the documented plaintext (Agent123!). [API-ONLY]
- Request: covered by a Rust unit/integration test that loads the seeded hash and calls the TS-M1-A4a verify util with "Agent123!" → true, and with a wrong password → false. End-to-end equivalent: `curl -s -X POST http://localhost:3701/api/staff/login -d '{"username":"agent","password":"Agent123!"}' -H 'Content-Type: application/json'` succeeds (once C1 exists).
- Expect: the hash verifies for the correct plaintext only.
- Status: [ ]

### AC-3: The seeded group grants can_create_tickets + can_post_reply + access to the seeded department. [API-ONLY]
- Request: `psql ... -c 'select can_create_tickets, can_post_reply from groups;'` and inspect `group_dept_access` for the seeded dept.
- Expect: both flags true and a group→department access row for the seeded "Support" dept.
- Status: [ ]

### AC-4: The seed task is idempotent (re-running upserts produces no duplicates and no error). [API-ONLY]
- Request: run `cargo run -p tools --bin seed` a SECOND time, then re-count department/groups/staff.
- Expect: exit 0, still exactly 1/1/1 (no duplicates).
- Status: [ ]

### AC-5: The seeded reference data uses the FS-091 literals open (status) and normal (priority). [API-ONLY]
- Request: `psql ... ` to inspect the seeded status/priority reference rows / config defaults.
- Expect: status literal `open`, priority literal `normal` (exact FS-091 spelling).
- Status: [ ]

## Test Infrastructure

- The seeded staff credentials (`agent` / `Agent123!`) are the login QA uses for the M1 staff
  E2E (root AC-2).
- **The reseed command (`cargo run -p tools --bin seed`) is the M1 dev-reset mechanism** — a
  single idempotent command that returns the dev DB to a known state for repeatable browser
  testing.
- **Database**: seeds the **`osticket_dev`** database in the **existing shared `backend-db-1`**
  container (`postgres:16`, host port **5432**) via
  `DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_dev`. No project-owned
  Postgres is started (3703 freed); reseed must be safe to re-run against the shared instance and
  must only write to `osticket_dev`.

## Dependencies

- TS-M1-A2 (schema exists, applied to `osticket_dev` in the shared `backend-db-1` container),
  TS-M1-A4a (argon2id password hashing util) for hashing the seeded password.
