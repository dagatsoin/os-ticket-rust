# TS-M4-D1 — add(route): FS-032.9/.10/.12 SLA CRUD + deletion constraints + priority read

- **ID**: TS-M4-D1
- **Type**: Technical Story
- **Parent**: US-M4-D1
- **Labels**: Technical Story, M4
- **Scope**: small

## Context

Backend for SLA plan management + the read-only priority list endpoint.

## Impact

- add(route): `GET /api/staff/admin/sla`, `POST`, `PUT /:id`, mass endpoint (activate/disable/delete) — admin-gated.
- add(service): FS-032.10 validation (name + grace period required); KL-032.4 error text corrected.
  `transient` is a **first-class `sla` column** (NOT a config-namespace key). The SLA form field set is:
  `name`, `grace_period` (hours), `isactive`, `enable_priority_escalation`, `transient`,
  `disable_overdue_alerts`, `notes`.
- add(service): FS-032.12 deletion — **re-home dependents on delete**:
  - `department.sla_id` and `help_topic.sla_id` → **NULL** (nullable FK — NOT the legacy `0` sentinel).
  - `ticket.sla_id` → `default_sla_id`.
  - **Refuse** deleting the default SLA (`id == default_sla_id`) **or** when `default_sla_id` is unset
    (no default to re-home tickets to) — 409/422 with an explicit message.
- add(route): `GET /api/staff/admin/priorities` — read-only fixed set (BS-032.13), no CRUD; ordered
  `ORDER BY urgency DESC` (Low, Normal, High, Emergency). For US-M4-D2 / TS-M4-D3.
- add(dev): `POST /api/dev/reset-sla` — **owned by this TS** (restores the M3 seeded SLA rows +
  default binding without a full reseed, for the delete/re-home ACs).

## Regressions

- The M3 due-date/overdue math is unchanged (this only manages the plan rows).

## Acceptance Tests

### AC-1: FS-032.9/.10 — list + create (admin only). [API-ONLY]
- Setup: reseed; admin session cookie (POST /api/staff/login admin/Admin123!); an agent session (agent/Agent123!).
- Request: GET http://localhost:3701/api/staff/admin/sla (admin) → 200 with the seeded plans.
- Request: POST /api/staff/admin/sla {name:"Gold", grace_period:24, isactive:true} (admin) → 201/200 created; the plan is returned/listable.
- Request: GET /api/staff/admin/sla as `agent` (non-admin) → 403.
- Status: [x]

### AC-2: FS-032.10 — validation (KL-032.4 corrected text). [API-ONLY]
- Setup: admin session.
- Request: POST /api/staff/admin/sla {} (no name, no grace period).
- Expect: 422; `error.fields` names `name` and `grace_period` with SLA-appropriate copy (KL-032.4 corrected — NOT the API-key copy-paste artifact text).
- Status: [x]

### AC-3: FS-032.12 — default SLA delete-protected; others re-home dependents. [API-ONLY]
- Setup: admin session; bind a non-default plan to a department (POST /api/dev/set-dept-sla).
- Request: DELETE /api/staff/admin/sla/{default_sla_id} → refused (409/422 with a "default cannot be deleted" message); the default row still exists.
- Request: DELETE /api/staff/admin/sla/{non-default plan id with a dependent department} → 200; GET the department → its `sla_id` is now **NULL** (dept/topic FK re-homed to NULL, NOT 0), and any ticket bound to the deleted plan now has `sla_id == default_sla_id`.
- Status: [x]

### AC-4: BS-032.13 — priorities read endpoint returns the ranked fixed set. [API-ONLY]
- Setup: admin session.
- Request: GET http://localhost:3701/api/staff/admin/priorities → 200; the 4 fixed priorities ordered `ORDER BY urgency DESC` (Low, Normal, High, Emergency).
- Verify: no POST/PUT/DELETE priority route exists (mutation attempts 404/405).
- Status: [x]

## Test Infrastructure

- Admin account (`admin`/`Admin123!`) + `agent` for the 403 case. Dev `POST /api/dev/seed-sla`, `POST /api/dev/set-dept-sla`. `.sqlx` cache updated.
- **Owns `POST /api/dev/reset-sla`** (build it here) — restores the M3 seeded SLA rows + default binding cheaply between the delete/re-home ACs; reseed (`cargo run -p tools --bin seed`) remains a fallback.

## Dependencies

- **EPIC-M4-PREP**: sla_plan transient key, default_sla_id. **TS-M4-A0**: admin gate.
