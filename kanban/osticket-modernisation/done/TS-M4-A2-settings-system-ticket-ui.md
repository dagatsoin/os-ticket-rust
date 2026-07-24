# TS-M4-A2 — add(frontend): FS-032.3/.4 System + Ticket settings tabs UI

- **ID**: TS-M4-A2
- **Type**: Technical Story
- **Parent**: US-M4-A1
- **Labels**: Technical Story, M4
- **Scope**: medium

## Context

The System and Ticket Settings tab bodies inside the admin shell (TS-M4-A0), wired to the settings
endpoints (TS-M4-A1).

## Impact

- add(frontend): `SettingsPage` with a tab strip; the System and Ticket tab forms rendering their
  FS-032.3 / FS-032.4 fields (text, selects backed by depts/priorities/SLA/topics, numeric, checkbox).
- add(frontend): save posts `{ tab, values }`; success banner; inline 422 field errors surfaced per field.
- add(frontend): timezone / default-object selects populated from the settings GET payload.

## Regressions

- None beyond additive routes; reuses the shared apiClient error-envelope parsing.

## Acceptance Tests

### AC-1: FS-032.3 — System tab renders fields and saves; value persists. [BROWSER]
- Setup: `cargo run -p tools --bin seed`; `POST http://localhost:3701/api/dev/reset-config`.
- Navigate: http://localhost:3702/staff/login → log in as `admin` / `Admin123!`.
- Navigate: http://localhost:3702/staff/admin (System tab active).
- Verify: the System tab renders its FS-032.3 fields (Helpdesk Name, Helpdesk URL, default timezone, default page size, password-reset period, session timeouts).
- Action: Clear "Helpdesk Name" and Type "Acme Helpdesk"; Click "Save".
- Verify: success banner.
- Action: reload; re-open the System tab.
- Verify: "Helpdesk Name" shows "Acme Helpdesk".
- Status: [x]

### AC-2: FS-032.4 — Ticket tab renders default selects and saves; value persists. [BROWSER]
- Navigate: click the "Ticket Settings" tab.
- Verify: the tab renders FS-032.4 fields with selects backed by depts/priorities/SLA/help-topics (default status, default priority, default department, default SLA, default help-topic) plus numeric lock-time / max-open-tickets and captcha checkbox.
- Action: change the "Default Priority" select to "High"; Click "Save".
- Verify: success banner.
- Action: reload; re-open the Ticket Settings tab.
- Verify: "Default Priority" shows "High".
- Status: [x]

### AC-3: FS-032.2 — inline field error on invalid input, no navigation. [BROWSER]
- Navigate: Ticket Settings tab.
- Action: Clear "Max Open Tickets" and Type "abc"; Click "Save".
- Verify: an inline error renders under the "Max Open Tickets" field; the tab stays open with the entered value (no navigation, no success banner).
- Status: [x]

## Test Infrastructure

- Component tests via Vitest + RTL + MSW mocking `/api/staff/admin/settings`.
- Admin account for the browser AC.

## Dependencies

- **TS-M4-A0**: admin shell + tab host.
- **TS-M4-A1**: settings GET/PUT endpoints.
