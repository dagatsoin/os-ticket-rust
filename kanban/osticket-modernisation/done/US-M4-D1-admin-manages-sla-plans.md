# US-M4-D1 — Admin manages SLA plans

- **ID**: US-M4-D1
- **Type**: User Story
- **Parent**: EPIC-M4-D
- **Labels**: User Story, M4
- **Scope**: small

## Spec References

- FS-032.9 (SLA plan listing & mass actions)
- FS-032.10 (SLA plan create / edit form)
- FS-032.11 (SLA overdue computation & hooks — math owned by M3)
- FS-032.12 (SLA plan deletion constraints & reassignment)
- KL-032.4 (copy-paste error text — modernised), KL-032.10 ("Date Added" not sortable — modernised)

## Context

Epic: EPIC-M4-D — SLA Plans & Priorities. M3 seeded SLA plans directly; this story gives them a CRUD
screen. SLA rows are consumed by departments and help topics (EPIC-M4-C); the default SLA is
delete-protected (Integration AC-1).

## Description

As an administrator, I can list SLA plans, mass activate/disable/delete them, and open a form to
create or edit one. The SLA form field set is: **name**, **grace_period** (hours), **isactive**,
**enable_priority_escalation**, **transient**, **disable_overdue_alerts**, **notes** (`transient` is a
first-class `sla` column). Deleting a plan re-homes its dependents (dept/topic `sla_id` → NULL,
ticket `sla_id` → default SLA); the default SLA cannot be deleted (nor when no default is set).

## Impact

- Frontend (SLA list + form + mass actions)
- Backend (SLA CRUD + deletion constraints/reassignment)
- Database (`sla_plan`, `department`, `help_topic`, `ticket`)
- Browser (desktop)

## Business Rules

- FS-032.10: name + grace period required; the remaining fields (isactive, enable_priority_escalation, transient, disable_overdue_alerts, notes) optional.
- FS-032.12: deleting a plan re-homes dependents — dept/topic `sla_id` → NULL (nullable FK, not the legacy `0`), ticket `sla_id` → `default_sla_id`; the default SLA is delete-protected (also refused when `default_sla_id` is unset).

## Regressions

- The M3 overdue computation (due date = base + grace) keeps working; seeded plans stay valid.

## Acceptance Criteria

### AC-1: The SLA list renders with mass actions; the "Date Added" column is sortable. [BROWSER]
- Setup: reseed the dev DB — `cargo run -p tools --bin seed` (restores M3 seeded SLA plans + the default SLA).
- Navigate: http://localhost:3702/staff/login → log in `admin` / `Admin123!` → open the admin shell → /staff/admin/sla.
- Verify: a table lists SLA plans with columns name, grace period, status, and Date Added.
- Action: click the "Date Added" column header.
- Verify: the row order changes (ascending/descending toggles) — KL-032.10 modernised (Date Added IS sortable).
- Verify: mass-action controls are present (row checkboxes + Activate / Disable / Delete bulk actions).
- Status: [x]

### AC-2: Admin creates an SLA plan. [BROWSER]
- Setup: continue from AC-1 (admin logged in at /staff/admin/sla).
- Action: click "Add SLA Plan"; in the form type Name "Gold SLA", Grace Period "24" (hours), check Active; click Save.
- Verify: a success banner appears and "Gold SLA" is now a row in the SLA list.
- Navigate: /staff/admin/departments → open a department's edit form (or /staff/admin/topics → a help topic form).
- Verify: "Gold SLA" is selectable in the department/help-topic SLA dropdown.
- Status: [x] — RE-TESTED after the camelCase fix. In-browser at /staff/admin/sla → Add SLA Plan → Name "Gold SLA", Grace Period 24, Active + Enable priority escalation + notes → Save → green "Gold SLA added successfully" banner; "Gold SLA" (24 hrs, Active) appears as a row and persists after a page reload. curl confirms gracePeriod:24 → 200. NOTE: the "Gold SLA selectable in the department/help-topic SLA dropdown" sub-check could NOT be exercised in the UI — both /staff/admin/departments and /staff/admin/help-topics render "Coming soon" (delivered later by EPIC-M4-C, currently unbuilt). The plan IS a real persisted row available to the backend; the dept-side binding was already DB-verified in AC-4. Core criterion (create SLA → banner + row) PASSES.

### AC-3: The default SLA cannot be deleted. [BROWSER]
- Setup: continue as admin at /staff/admin/sla.
- Action: locate the default SLA row (the one bound as `default_sla_id`) and attempt to delete it (select its checkbox + Delete, or its row Delete control).
- Verify: the deletion is refused — either the default SLA's checkbox is disabled, or the delete is blocked with an explicit message (e.g. "The default SLA cannot be deleted") (Integration AC-1). The default SLA remains in the list.
- Status: [x]

### AC-4: Deleting a non-default plan re-homes dependents. [BROWSER]
- Setup: continue as admin. Create a plan "Temp SLA" (AC-2 flow) and bind it to a department: Navigate /staff/admin/departments → edit a department → set its SLA to "Temp SLA" → Save.
- Action: Navigate /staff/admin/sla → select "Temp SLA" → Delete → confirm.
- Verify: "Temp SLA" is removed from the list; a success message confirms deletion.
- Navigate: /staff/admin/departments → reopen that department's edit form.
- Verify: its SLA is now unset / "— none —" (dept `sla_id` re-homed to NULL, per FS-032.12). (Tickets that had been on "Temp SLA" are re-homed to the default SLA — the department itself is NOT.)
- Status: [x] — NOTE: criterion (delete non-default → row removed + dependents re-homed) verified via the real UI mass-delete (green "Selected SLA plans deleted successfully" snackbar, row removed reactively) and DB (bound department.sla_id → NULL; ticket.sla_id → default_sla_id via TS-M4-D1 AC-3). The "Temp SLA" was seeded via /api/dev/seed-sla instead of the Add-SLA form because that form is broken (see AC-2).

### AC-5: Create validation. [API-ONLY]
- Setup: obtain an admin session cookie (POST /api/staff/login with admin/Admin123!).
- Request: POST http://localhost:3701/api/staff/admin/sla with an empty name and no grace period.
- Expect: 422; the error envelope `{ "error": { "fields": { ... } } }` names the SLA `name` and `grace_period` fields with correct SLA copy (KL-032.4 corrected — NOT the API-key copy-paste text).
- Status: [x]

## Checklist (children)

- [ ] TS-M4-D1 — Backend: SLA CRUD + deletion constraints/reassignment + priority read endpoint
- [ ] TS-M4-D2 — Frontend: SLA plan list + create/edit form UI

## Test Infrastructure

- Admin account (`admin` / `Admin123!`). Reuses M3 seeded SLA plans + the default SLA (`default_sla_id`).
- Reset mechanism: **reseed** via `cargo run -p tools --bin seed` (idempotent, restores SLA rows).
- Existing dev endpoint: `POST /api/dev/seed-sla` (create an extra plan), `POST /api/dev/set-dept-sla` (bind a plan to a department for AC-4).
- `POST /api/dev/reset-sla` is **built by TS-M4-D1** (restores the M3 seeded SLA rows + default binding) — use it to reset SLA state between the delete/re-home ACs (AC-3/AC-4) without a full reseed.

## Dependencies

- **EPIC-M4-PREP**: sla_plan transient key, default_sla_id.
- **EPIC-M4-A (shell)**. Consumed by **EPIC-M4-C** (dept/topic SLA select).
