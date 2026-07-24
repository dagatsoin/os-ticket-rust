# TS-M4-D3 — add(frontend): FS-032.8 read-only priorities reference panel

- **ID**: TS-M4-D3
- **Type**: Technical Story
- **Parent**: US-M4-D2
- **Labels**: Technical Story, M4
- **Scope**: small

## Context

The read-only Priorities panel inside the admin shell, reading the priority list endpoint (TS-M4-D1).
KL-032.1 is PRESERVED — no CRUD controls.

## Impact

- add(frontend): `PrioritiesPage` — a read-only table of the fixed priority set (name, urgency rank,
  color), with no add/edit/delete controls.

## Regressions

- None; read-only view.

## Acceptance Tests

### AC-1: FS-032.8 — priorities render read-only in urgency order. [BROWSER]
- Setup: reseed (`cargo run -p tools --bin seed`).
- Navigate: http://localhost:3702/staff/login → log in `admin`/`Admin123!` → /staff/admin/priorities.
- Verify: the 4 priorities render in urgency order — `ORDER BY urgency DESC` → **Low, Normal, High, Emergency** — with rank + color.
- Verify: no Add/Edit/Delete controls and no mass-action checkboxes exist (read-only, KL-032.1 preserved).
- Status: [x]

## Test Infrastructure

- Vitest + RTL + MSW mocking `/api/staff/admin/priorities`. Admin account.

## Dependencies

- **TS-M4-A0**: shell. **TS-M4-D1**: priority read endpoint.
