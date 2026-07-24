# US-M4-A1 — Admin edits the System & Ticket settings tabs

- **ID**: US-M4-A1
- **Type**: User Story
- **Parent**: EPIC-M4-A
- **Labels**: User Story, M4
- **Scope**: medium

## Spec References

- FS-032.1 (settings panel entry & tab routing)
- FS-032.2 (settings save dispatch & per-tab validation)
- FS-032.3 (System settings tab fields)
- FS-032.4 (Ticket settings & options tab fields)
- FS-032.7 (settings persistence semantics)
- BS-032.4 (checkbox settings persist as 0/1 by presence)

## Context

Epic: EPIC-M4-A — System Settings. This story delivers the admin's first landing screen: a tabbed
settings panel where the System and Ticket tabs can be edited and saved. It also establishes the
shared admin shell (nav + admin gate) every other admin screen reuses.

## Description

As an administrator, I open the admin panel and see a System Settings screen with tabs. On the
**System** tab I can change global values (helpdesk name/URL, default timezone, default page size,
password-reset period, session timeouts); on the **Ticket Settings** tab I can change ticket defaults
(default status/priority/department/SLA/help-topic, lock time, max open tickets, captcha). Saving
validates the tab and shows a success banner; my changes persist and take effect app-wide.

## Impact

- Frontend (admin shell + nav + System/Ticket tabs + save)
- Backend (settings GET/PUT endpoints + per-tab validation + persistence)
- Database (`config` table)
- Browser (desktop)

## Business Rules

- BS-032.4: checkbox settings persist as 0/1 by presence in the submission.
- FS-032.2: each tab validates only its own fields; a validation error re-renders the tab with values.
- FS-032.7: only the submitting tab's keys are written; other tabs' keys are untouched.

## Regressions

- The staff area routing (`/staff/*`) must keep working; the admin shell adds `/staff/admin/*`.
- Keys shared with M2/M3 (`max_page_size`, `ticket_lock_time`) keep their runtime meaning.

## Acceptance Criteria

### AC-1: The admin panel opens on a tabbed System Settings screen; non-admins are denied. [BROWSER]
- Setup: `cargo run -p tools --bin seed` (idempotent — seeds `admin`/`Admin123!` isadmin + `agent`/`Agent123!` + config defaults + email_account/template_group).
- Setup: `POST http://localhost:3701/api/dev/reset-config` (restore FS-032 config defaults to a known state).
- Navigate: http://localhost:3702/staff/login
- Action: Type "admin" in the username field.
- Action: Type "Admin123!" in the password field.
- Action: Click "Sign In".
- Navigate: http://localhost:3702/staff/admin
- Verify: the admin settings screen renders with a tab strip exposing all seven tabs — System, Ticket Settings & Options, Email, Site Pages, Knowledgebase, Autoresponder, Alerts & Notices (Attachments is a **separate** settings screen, not one of the seven tabs — per FS-032.1; covered in US-M4-A2).
- Verify: the "System" tab is active by default and renders its fields (Helpdesk Name, default page size, etc.).
- Action: in a fresh session (log out, or a private window) log in as `agent` / `Agent123!`.
- Navigate: http://localhost:3702/staff/admin
- Verify: the non-admin `agent` is redirected to the staff dashboard (`/staff/tickets`) with a denial notice; no admin nav renders.
- Status: [x]

### AC-2: Admin edits a System-tab field and it persists after reload. [BROWSER]
- Setup: logged in as `admin` on http://localhost:3702/staff/admin, System tab (AC-1 state).
- Action: Clear the "Helpdesk Name" field and Type "Acme Helpdesk".
- Action: Click "Save".
- Verify: a success banner appears (e.g. "System settings updated").
- Action: reload the page and re-open the System tab.
- Verify: the "Helpdesk Name" field shows "Acme Helpdesk".
- Status: [x]

### AC-3: Admin edits a Ticket-tab field and it persists after reload. [BROWSER]
- Navigate: click the "Ticket Settings" tab.
- Verify: the tab renders ticket-default fields (default priority select, default SLA, default help-topic, max open tickets, lock time, captcha).
- Action: change the "Default Priority" select to a different value (e.g. "High").
- Action: Click "Save".
- Verify: success banner.
- Action: reload and re-open the Ticket Settings tab.
- Verify: the "Default Priority" select shows "High".
- Status: [x]

### AC-4: A per-tab validation error re-renders the tab inline without touching other tabs. [BROWSER]
- Navigate: Ticket Settings tab.
- Action: Clear "Max Open Tickets" and Type "abc" (non-numeric).
- Action: Click "Save".
- Verify: an inline field error appears under "Max Open Tickets" (e.g. "Enter a number"); the tab stays open with the entered values (no navigation, no success banner).
- Action: click the "System" tab.
- Verify: the System-tab values are unchanged (the failed Ticket save wrote nothing).
- Status: [x]

### AC-5: Only the submitted tab's keys are written. [API-ONLY]
- Setup: `POST http://localhost:3701/api/dev/reset-config`; obtain an admin session cookie (log in as `admin`/`Admin123!`).
- Request: GET http://localhost:3701/api/staff/admin/settings (Cookie: admin session) → record the System-tab `helpdesk_name` value.
- Action: PUT http://localhost:3701/api/staff/admin/settings with body `{ "tab": "ticket", "values": { "default_priority_id": <valid priority id> } }` (Cookie: admin session).
- Expect: 200.
- Request: GET http://localhost:3701/api/staff/admin/settings
- Expect: 200; the System-tab `helpdesk_name` is unchanged; the Ticket-tab `default_priority_id` reflects the new value (FS-032.7).
- Status: [x]

## Checklist (children)

- [ ] TS-M4-A0 — Frontend: admin panel shell + nav + admin-gate routing
- [ ] TS-M4-A1 — Backend: settings GET/PUT endpoints + per-tab validation + persistence
- [ ] TS-M4-A2 — Frontend: System + Ticket settings tabs UI

## Test Infrastructure

- Requires an **admin** account. **No dedicated `seed-admin` endpoint exists** — the `admin`/`Admin123!`
  isadmin account is seeded by the standard **`cargo run -p tools --bin seed`** (TS-M4-PREP-C, verified
  in `tools/src/lib.rs`) and preserved by `POST /api/dev/reset-staff` (KEEP list: agent, agent2, admin).
- Config reset for a known Setup baseline: **`POST /api/dev/reset-config`** (verified — restores the
  FS-032 defaults via `tools::restore_config_defaults`, single source of truth with the seed).
- New endpoints this US relies on (built by its children, not yet present): `GET/PUT /api/staff/admin/settings`
  and `isadmin` + capability flags on `GET /api/staff/me` (both **TS-M4-A1**; TS-M4-A0 is the frontend consumer).

## Dependencies

- **EPIC-M4-PREP**: seeded config keys.
- **TS-M4-A0**: admin shell hosts these tabs (this US owns A0).
