# TS-M4-D2 — add(frontend): FS-032.9/.10 SLA plan list + create/edit form UI

- **ID**: TS-M4-D2
- **Type**: Technical Story
- **Parent**: US-M4-D1
- **Labels**: Technical Story, M4
- **Scope**: small

## Context

The SLA plan screen inside the admin shell, wired to TS-M4-D1.

## Impact

- add(frontend): `SlaListPage` — table (name, grace period, status), sortable "Date Added"
  (KL-032.10 modernised), mass actions; the default SLA's checkbox disabled.
- add(frontend): `SlaFormDialog` — the reconciled field set: **name**, **grace_period** (hours),
  **isactive**, **enable_priority_escalation**, **transient**, **disable_overdue_alerts**, **notes**
  (`transient` is a first-class `sla` column, not a config key); inline 422 + success banner.

## Regressions

- None; additive.

## Acceptance Tests

### AC-1: FS-032.9 — SLA list renders with sortable Date Added + mass actions. [BROWSER]
- Setup: reseed (`cargo run -p tools --bin seed`).
- Navigate: http://localhost:3702/staff/login → log in `admin`/`Admin123!` → /staff/admin/sla.
- Verify: the SLA table shows name / grace period / status / Date Added.
- Action: click the Date Added header.
- Verify: row order toggles (sortable, KL-032.10 modernised); row checkboxes + Activate/Disable/Delete mass actions are present.
- Status: [x]

### AC-2: FS-032.10 — create an SLA plan. [BROWSER]
- Setup: continue as admin at /staff/admin/sla.
- Action: click "Add SLA Plan"; Name "Silver SLA", Grace Period "8", Active checked; Save.
- Verify: success banner; "Silver SLA" appears in the list.
- Status: [x] — RE-TESTED after the camelCase fix: SlaAdminStore.writeBody() now sends `gracePeriod`/`enablePriorityEscalation`/`disableOverdueAlerts`. In-browser: Add SLA Plan → Name "Gold SLA", Grace Period 24, Active + Enable priority escalation → Save → green "Gold SLA added successfully" banner, row appears in the list and persists after reload. curl confirms gracePeriod:24 → 200 {id}.

### AC-3: FS-032.12 — default SLA delete blocked in the UI. [BROWSER]
- Setup: continue as admin at /staff/admin/sla.
- Action: attempt to delete the default SLA (select its checkbox + Delete).
- Verify: the checkbox is disabled OR the delete is blocked with a visible message ("default SLA cannot be deleted"); the row remains.
- Status: [x]

## Test Infrastructure

- Vitest + RTL + MSW mocking `/api/staff/admin/sla*`. Admin account.

## Dependencies

- **TS-M4-A0**: shell. **TS-M4-D1**: SLA endpoints.
