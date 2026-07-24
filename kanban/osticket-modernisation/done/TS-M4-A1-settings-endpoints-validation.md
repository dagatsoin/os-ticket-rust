# TS-M4-A1 — add(route): FS-032.2 settings GET/PUT endpoints + per-tab validation + persistence

- **ID**: TS-M4-A1
- **Type**: Technical Story
- **Parent**: US-M4-A1
- **Labels**: Technical Story, M4
- **Scope**: medium

## Context

Backend for the settings panel: read all config keys, and write them a tab at a time with per-tab
validation and presence-based persistence. Shared by US-M4-A1 (System/Ticket tabs) and US-M4-A2
(Email/Pages/Knowledgebase/Alerts tabs + the separate Attachments screen). This TS also owns the
**admin/capability bootstrap**: loading `staff.isadmin` + the four M4 capability flags and exposing
them on `GET /api/staff/me`, plus the shared `require_admin` per-route gate helper — consumed by the
frontend guards in TS-M4-A0.

## Impact

- add(route): `GET /api/staff/admin/settings` — admin-gated; returns the config keys grouped by tab
  (`system`, `ticket`, `email`, `pages`, `kb`, `autoresponder`, `alerts`, `attachments`; the `kb`
  group holds `enable_kb` + `enable_premade`; `attachments` is served to the separate Attachments
  screen). The payload also carries the **option lists** the selects need — departments, SLA plans,
  help topics, priorities, email_accounts, template_groups, timezones — inline.
- add(route): `PUT /api/staff/admin/settings` — admin-gated; accepts `{ tab, values }`, validates only
  that tab's fields (FS-032.2), writes only that tab's keys (FS-032.7), checkbox keys by presence (BS-032.4).
- add(service): per-tab validators (numeric ranges, required selects, url/email shape) returning the
  shared 422 field-error envelope.
- update(backend): the staff session bootstrap (`GET /api/staff/me`) exposes `isadmin` and the four M4
  capability flags — `can_manage_faq`, `can_manage_premade`, `can_ban_emails`, `can_view_staff_stats` —
  loaded from `staff.isadmin` + the staff group; consumed by the TS-M4-A0 frontend guards.
- add(service): a shared `require_admin` per-route gate helper (401 no session / 403 non-admin) used by
  the settings routes; the delegated capability gate is a sibling helper for EPIC-M4-E/H.
- add(route, dev-only): `POST /api/dev/seed-page` — seeds a `landing` page so the Pages/Landing settings
  AC (US-M4-A2 / TS-M4-A3 AC-2) is testable without EPIC-M4-F.
- KL-032.3 modernised: `send_sys_errors` is read and written as its real stored value.

## Regressions

- Runtime consumers of these keys (pagination, auth timeouts, attachment limits) keep reading the
  same key names; only the write path is new.

## Acceptance Tests

### AC-1: FS-032.2 — GET returns config grouped by tab (admin only). [API-ONLY]
- Setup: `cargo run -p tools --bin seed`; `POST http://localhost:3701/api/dev/reset-config`; obtain an admin session cookie.
- Request: GET http://localhost:3701/api/staff/admin/settings (Cookie: admin session)
- Expect: 200; a JSON object keyed by tab — `system`, `ticket`, `email`, `pages`, `kb`, `autoresponder`, `alerts`, `attachments` — each holding its config keys (the `kb` group holds `enable_kb` + `enable_premade`).
- Request: GET http://localhost:3701/api/staff/admin/settings (Cookie: agent session) → 403 (admin gate).
- Request: GET http://localhost:3701/api/staff/admin/settings (no session) → 401.
- Status: [x]

### AC-2: FS-032.7 — PUT writes only the submitted tab's keys. [API-ONLY]
- Setup: `POST /api/dev/reset-config`; admin session; GET settings and record the system-tab `helpdesk_name`.
- Action: PUT http://localhost:3701/api/staff/admin/settings `{ "tab": "ticket", "values": { "default_priority_id": <valid id> } }`.
- Expect: 200.
- Request: GET http://localhost:3701/api/staff/admin/settings → the ticket `default_priority_id` changed; the system `helpdesk_name` unchanged.
- Status: [x]

### AC-3: FS-032.2 — invalid tab value returns 422 with a field error and writes nothing. [API-ONLY]
- Action: PUT http://localhost:3701/api/staff/admin/settings `{ "tab": "ticket", "values": { "max_open_tickets": "abc" } }` (admin session).
- Expect: 422; `error.fields.max_open_tickets` present.
- Request: GET → the ticket keys are unchanged (rejected write is atomic).
- Status: [x]

### AC-4: BS-032.4 — checkbox persists 0/1 by presence. [API-ONLY]
- Action: PUT the `alerts` tab OMITTING a recipient checkbox key (its event left validly configured) → GET shows that key `0`.
- Action: PUT the `alerts` tab INCLUDING that key set on → GET shows `1`.
- Expect: GET reflects 0 then 1.
- Status: [x]

### AC-5: KL-032.3 — send_sys_errors round-trips its real value. [API-ONLY]
- Action: PUT http://localhost:3701/api/staff/admin/settings `{ "tab": "alerts", "values": { "send_sys_errors": 1 } }` (admin session).
- Request: GET http://localhost:3701/api/staff/admin/settings
- Expect: `send_sys_errors` = 1 (stored real value, not forced 0 as in the legacy defect).
- Status: [x]

### AC-6: BS-032.5 — an enabled alert event with no recipients returns 422. [API-ONLY]
- Action: PUT http://localhost:3701/api/staff/admin/settings `{ "tab": "alerts", "values": { "ticket_alert_active": 1 } }` (New-Ticket event enabled, every `ticket_alert_*` recipient key omitted → 0) (admin session).
- Expect: 422; `error.fields.ticket_alert_active` = "Select recipient(s)".
- Request: GET → no alert keys written.
- Status: [x]

### AC-7: BS-032.6 — attachment sub-field validation is conditional on the master switch. [API-ONLY]
- Action: PUT `{ "tab": "attachments", "values": { "allow_attachments": 0, "max_file_size": "-5" } }` → Expect 200 (validation skipped while the master switch is off).
- Action: PUT `{ "tab": "attachments", "values": { "allow_attachments": 1, "max_file_size": "-5" } }` → Expect 422; `error.fields.max_file_size` present.
- Status: [x]

### AC-8: FS-032.1 — GET /api/staff/me exposes isadmin + the four M4 capability flags. [API-ONLY]
- Setup: `cargo run -p tools --bin seed`; obtain an admin session cookie (log in as `admin`/`Admin123!`).
- Request: GET http://localhost:3701/api/staff/me (Cookie: admin session)
- Expect: 200; body includes `isadmin: true` and the four flags `can_manage_faq`, `can_manage_premade`, `can_ban_emails`, `can_view_staff_stats`.
- Request: GET http://localhost:3701/api/staff/me (Cookie: agent session — plain `agent`/`Agent123!`)
- Expect: 200; `isadmin: false` and the four capability flags all `false` for the plain agent group.
- Status: [x]

### AC-9: FS-032.1 — require_admin gates settings GET and PUT for non-admins (403). [API-ONLY]
- Setup: admin + agent sessions.
- Request: GET http://localhost:3701/api/staff/admin/settings (Cookie: agent session) → Expect 403.
- Action: PUT http://localhost:3701/api/staff/admin/settings `{ "tab": "system", "values": { "helpdesk_name": "x" } }` (Cookie: agent session) → Expect 403.
- Request: GET http://localhost:3701/api/staff/admin/settings (no session) → Expect 401.
- Request: GET http://localhost:3701/api/staff/admin/settings → confirm no keys were written by the rejected non-admin PUT.
- Status: [x]

### AC-10: FS-032.2 — settings GET returns the option lists the selects need, inline. [API-ONLY]
- Setup: `cargo run -p tools --bin seed`; admin session.
- Request: GET http://localhost:3701/api/staff/admin/settings (Cookie: admin session)
- Expect: 200; the payload carries option lists for `departments`, `sla_plans`, `help_topics`, `priorities`, `email_accounts`, `template_groups`, and `timezones` (each a non-empty list from the seed where applicable).
- Status: [x]

## Test Infrastructure

- Admin + agent accounts via seed. `SQLX_OFFLINE` query cache updated for the new queries.
- **`POST /api/dev/seed-page`** (dev-only) is provided by this TS so the Pages/Landing settings AC
  (US-M4-A2 / TS-M4-A3 AC-2) is testable without EPIC-M4-F.

## Dependencies

- **EPIC-M4-PREP**: seeded config keys; `staff.isadmin` + group capability-flag columns.
- **TS-M4-A0**: consumes this TS's `GET /api/staff/me` (isadmin + flags) — this TS provides the gate,
  A0 consumes it (no backward dependency on A0).
