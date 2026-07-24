# TS-M4-A3 — add(frontend): FS-032.5/.6 Email/Pages/Autoresponder/Alerts/Attachments tabs UI

- **ID**: TS-M4-A3
- **Type**: Technical Story
- **Parent**: US-M4-A2
- **Labels**: Technical Story, M4
- **Scope**: medium

## Context

The remaining settings tab bodies inside the admin shell, wired to the shared settings endpoints
(TS-M4-A1). The Email tab binds `email_account`/`template_group`; the Pages tab binds `page` rows.

## Impact

- add(frontend): Email tab (default email account + template selects), Pages tab (landing/offline/
  thank-you page selects populated from `GET /api/staff/admin/pages`), Knowledgebase tab
  (`enable_kb` / `enable_premade` toggles), Autoresponder + Alerts & Notices tabs (checkbox groups),
  and a **separate** Attachments settings screen at `/staff/admin/settings/attachments` (standalone
  sub-route, **not** a 7th tab per FS-032.1; master switch gating the type/size fields, BS-032.6).
- add(frontend): each tab / the Attachments screen posts `{ tab, values }` and surfaces success/422 like TS-M4-A2.

## Regressions

- The Attachments tab must not weaken server-side upload validation (backend remains authoritative).

## Acceptance Tests

### AC-1: FS-032.5 — Email tab shows the seeded account/template and saves. [BROWSER]
- Setup: `cargo run -p tools --bin seed`; `POST http://localhost:3701/api/dev/reset-config`.
- Navigate: http://localhost:3702/staff/login → log in as `admin` / `Admin123!`.
- Navigate: http://localhost:3702/staff/admin → click the "Email" tab.
- Verify: the "Default Email" select lists `support@osticket.local`; the "Default Template" select lists "osTicket Default".
- Action: choose both; Click "Save".
- Verify: success banner; reload → both selects retain the chosen values.
- Status: [x]

### AC-2: FS-032.6 — Pages tab binds a landing page. [BROWSER]
- Setup: seed a `landing` page via `POST http://localhost:3701/api/dev/seed-page` (TS-M4-A1 dev endpoint) — testable without EPIC-M4-F.
- Navigate: click the "Pages" tab.
- Action: select the page in the "Landing Page" select; Click "Save".
- Verify: success banner; reload → the binding persists.
- Status: [x]

### AC-3: BS-032.6 — Attachments master switch gates the sub-fields. [BROWSER]
- Navigate: open the **separate** Attachments settings screen at http://localhost:3702/staff/admin/settings/attachments (standalone screen, **not** one of the seven tabs — FS-032.1).
- Action: toggle "Allow Attachments" OFF → Verify the "Allowed File Types" and "Max File Size" fields become disabled.
- Action: toggle "Allow Attachments" ON → Verify the sub-fields re-enable; change "Max File Size"; Click "Save".
- Verify: success banner; reload → the switch ON and new max size persist.
- Status: [x]

### AC-4: FS-032.6 — an Alerts checkbox persists by presence. [BROWSER]
- Navigate: click the "Alerts & Notices" tab.
- Action: toggle a notice recipient checkbox (event kept valid with ≥1 recipient); Click "Save".
- Verify: success banner; reload → the checkbox state is retained.
- Status: [x]

### AC-5: FS-032.6 — Autoresponder tab edits and saves a toggle. [BROWSER]
- Navigate: click the "Autoresponder" tab.
- Verify: the tab renders autoresponder toggles (New Ticket / New Message autoresponse, overlimit notice).
- Action: toggle "New Ticket Autoresponse"; Click "Save".
- Verify: success banner; reload → state retained.
- Status: [x]

### AC-6: BS-032.5 — an enabled alert event with no recipients shows an inline error. [BROWSER]
- Navigate: click the "Alerts & Notices" tab.
- Action: set "New Ticket Alert" active = Enable; uncheck ALL its recipient checkboxes; Click "Save".
- Verify: an inline "Select recipient(s)" error renders against the New-Ticket event; the tab re-renders with values; no success banner (BS-032.5).
- Status: [x]

### AC-7: FS-032.6 — Knowledgebase tab toggles a KB setting and it persists. [BROWSER]
- Navigate: click the "Knowledgebase" tab.
- Verify: the tab renders KB toggles — "Enable Knowledgebase" (`enable_kb`) and "Enable Canned Responses" (`enable_premade`).
- Action: toggle "Enable Knowledgebase"; Click "Save".
- Verify: success banner; reload → the toggle state is retained.
- Status: [x]

## Test Infrastructure

- Component tests via Vitest + RTL + MSW mocking settings + pages endpoints.
- Requires seeded email_account/template_group (TS-M4-PREP-D). AC-2 seeds a `landing` page via
  `POST /api/dev/seed-page` (TS-M4-A1) — no EPIC-M4-F dependency.

## Dependencies

- **TS-M4-A0/A1**: shell + settings endpoints + `POST /api/dev/seed-page`.
- **EPIC-M4-F** (later): the real Pages admin + `GET /api/staff/admin/pages`; AC-2 is unblocked in the
  interim by the `POST /api/dev/seed-page` dev endpoint (TS-M4-A1).
