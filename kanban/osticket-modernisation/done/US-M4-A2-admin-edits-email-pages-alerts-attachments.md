# US-M4-A2 — Admin edits the Email, Pages, Alerts & Attachment settings tabs

- **ID**: US-M4-A2
- **Type**: User Story
- **Parent**: EPIC-M4-A
- **Labels**: User Story, M4
- **Scope**: medium

## Spec References

- FS-032.5 (Email settings tab fields)
- FS-032.6 (Site Pages, Logos, Autoresponder, Knowledge-Base & Alerts tabs)
- FS-032.2 (per-tab validation), FS-032.7 (persistence semantics)
- BS-032.6 (attachment validation conditional on the master switch)
- BS-033.9 (default-page bindings drive in-use protection — consumed here)

## Context

Epic: EPIC-M4-A — System Settings. Completes the settings surface with the remaining tabs. The Email
tab binds the seeded `email_account` (M4-D1); the Pages tab binds `page` rows as landing/offline/
thank-you (Integration AC-4); the Attachments tab tunes the M2 upload limits behind a master switch.

## Description

As an administrator, I can edit the **Email**, **Pages**, **Knowledgebase**, **Autoresponder** and
**Alerts & Notices** settings tabs, and the **separate Attachments** settings screen. On the Email tab
I pick the default email account and template; on the Pages tab I bind the landing / offline / thank-you
pages from the pages I authored; on the Knowledgebase tab I toggle KB features (`enable_kb`,
`enable_premade`); on the Attachments screen (a standalone sub-route, not one of the seven tabs per
FS-032.1) I toggle attachments on/off and set the allowed types and max size. Saving each tab / the
screen validates and persists.

## Impact

- Frontend (Email/Pages/Knowledgebase/Autoresponder/Alerts tabs + the separate Attachments screen)
- Backend (shared settings PUT + per-tab validation)
- Database (`config`, referencing `email_account`, `template_group`, `page`)
- Browser (desktop)

## Business Rules

- BS-032.6: attachment-type/size validation applies only when the master "allow attachments" switch is on.
- BS-033.9: a page bound here becomes in-use and is delete-protected in EPIC-M4-F.
- FS-032.7: per-tab persistence — only the submitted tab's keys change.

## Regressions

- The M2 attachment keys keep their runtime meaning (upload validation still enforced server-side).
- Binding a page must not orphan an existing binding when the page list changes.

## Acceptance Criteria

### AC-1: The Email tab lists the seeded email account and template and saves the defaults. [BROWSER]
- Setup: `cargo run -p tools --bin seed` (seeds admin + email_account `support@osticket.local` + template_group "osTicket Default"); `POST http://localhost:3701/api/dev/reset-config`.
- Navigate: http://localhost:3702/staff/login → log in as `admin` / `Admin123!`.
- Navigate: http://localhost:3702/staff/admin → click the "Email" tab.
- Verify: the "Default Email" select lists `support@osticket.local` (M4-D1); the "Default Template" select lists "osTicket Default".
- Action: choose "Default Email" = support@osticket.local and "Default Template" = osTicket Default; Click "Save".
- Verify: success banner.
- Action: reload; re-open the Email tab.
- Verify: both selects retain the chosen values.
- Status: [x]

### AC-2: The Pages tab binds an authored page as the landing page and it persists. [BROWSER]
- Setup: seed a `landing` page via `POST http://localhost:3701/api/dev/seed-page` (TS-M4-A1 dev endpoint) — this AC is testable without EPIC-M4-F.
- Navigate: click the "Pages" tab.
- Action: select the authored page in the "Landing Page" select; Click "Save".
- Verify: success banner.
- Action: reload; re-open the Pages tab.
- Verify: the "Landing Page" select retains the bound page (Integration AC-4 then delete-protects it in EPIC-M4-F).
- Status: [x]

### AC-3: The Attachments master switch gates the sub-fields and limits persist (BS-032.6). [BROWSER]
- Navigate: open the **separate** Attachments settings screen at http://localhost:3702/staff/admin/settings/attachments (a standalone screen, **not** one of the seven tabs — FS-032.1).
- Verify: an "Allow Attachments" master switch is present alongside "Allowed File Types" and "Max File Size" sub-fields.
- Action: toggle "Allow Attachments" OFF.
- Verify: the "Allowed File Types" and "Max File Size" sub-fields become disabled/greyed (validation not applied while off, BS-032.6).
- Action: toggle "Allow Attachments" ON; change "Max File Size" to a new value; Click "Save".
- Verify: success banner.
- Action: reload; re-open the Attachments tab.
- Verify: the switch is ON and the new "Max File Size" persists.
- Status: [x]

### AC-4: An Alerts & Notices recipient checkbox persists by presence (BS-032.4). [BROWSER]
- Navigate: click the "Alerts & Notices" tab.
- Action: with "New Ticket Alert" enabled, toggle one of its recipient checkboxes (e.g. "Department Manager") while at least one recipient stays checked; Click "Save".
- Verify: success banner.
- Action: reload; re-open the Alerts & Notices tab.
- Verify: the checkbox retains its new state (persisted 0/1 by presence).
- Status: [x]

### AC-5: The Autoresponder tab edits and saves a representative toggle. [BROWSER]
- Navigate: click the "Autoresponder" tab.
- Verify: the tab renders autoresponder toggles (e.g. "New Ticket Autoresponse", "New Message Autoresponse", "Overlimit Notice").
- Action: toggle "New Ticket Autoresponse"; Click "Save".
- Verify: success banner.
- Action: reload; re-open the Autoresponder tab.
- Verify: the toggle retains its new state.
- Status: [x]

### AC-6: An enabled alert event with no recipients is rejected (BS-032.5). [BROWSER]
- Navigate: click the "Alerts & Notices" tab.
- Action: set "New Ticket Alert" active radio = Enable, then uncheck ALL of its recipient checkboxes.
- Action: Click "Save".
- Verify: the save is rejected with a per-event error "Select recipient(s)" shown against the New-Ticket event (keyed on `ticket_alert_active`); the tab re-renders with the entered values (BS-032.5). No success banner; other tabs unaffected.
- Status: [x]

### AC-7: The settings PUT rejects an enabled alert event with no recipients (BS-032.5). [API-ONLY]
- Setup: `POST http://localhost:3701/api/dev/reset-config`; obtain an admin session cookie.
- Action: PUT http://localhost:3701/api/staff/admin/settings with body `{ "tab": "alerts", "values": { "ticket_alert_active": 1 } }` (the New-Ticket event enabled with every `ticket_alert_*` recipient key omitted → 0) (Cookie: admin session).
- Expect: 422; `error.fields.ticket_alert_active` present with "Select recipient(s)" (FS-032.2 / BS-032.5).
- Request: GET http://localhost:3701/api/staff/admin/settings → confirm no alert keys were written.
- Status: [x]

### AC-8: The Knowledgebase tab toggles a KB setting and it persists after reload. [BROWSER]
- Navigate: click the "Knowledgebase" tab.
- Verify: the tab renders KB toggles — "Enable Knowledgebase" (`enable_kb`) and "Enable Canned Responses" (`enable_premade`).
- Action: toggle "Enable Knowledgebase" (`enable_kb`); Click "Save".
- Verify: success banner (e.g. "Knowledgebase settings updated").
- Action: reload; re-open the Knowledgebase tab.
- Verify: the toggle retains its new state (persisted 0/1 by presence, BS-032.4).
- Status: [x]

## Checklist (children)

- [ ] TS-M4-A3 — Frontend: Email/Pages/Autoresponder/Alerts/Attachments tabs UI
- [ ] (shared) TS-M4-A1 — Backend settings endpoints (owned by US-M4-A1)

## Test Infrastructure

- Admin account via `cargo run -p tools --bin seed` (`admin`/`Admin123!`, verified); config baseline via
  `POST /api/dev/reset-config` (verified). Reuses the settings endpoints from TS-M4-A1.
- Requires seeded `email_account`/`template_group` (TS-M4-PREP-D — seeded by the standard seed).
- **AC-2 (Pages bind)**: unblocked by `POST /api/dev/seed-page` (TS-M4-A1 dev endpoint) which creates a
  `landing` page — no EPIC-M4-F dependency for this AC.
- **AC-3 (Attachments)** exercises the **separate** `/staff/admin/settings/attachments` screen, not a tab.

## Dependencies

- **US-M4-A1**: admin shell + backend settings endpoints (TS-M4-A0/A1).
- **EPIC-M4-PREP**: email_account/template_group rows; page table.
- **EPIC-M4-F** (Integration AC-4): the real Pages admin; AC-2 no longer blocks on it thanks to the
  `POST /api/dev/seed-page` dev endpoint (TS-M4-A1).
