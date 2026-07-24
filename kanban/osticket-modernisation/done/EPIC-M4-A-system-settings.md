# EPIC-M4-A — EPIC – System Settings (7 Tabs + Attachments)

- **ID**: EPIC-M4-A
- **Type**: Epic
- **Parent**: M4
- **Labels**: Epic, M4
- **Column**: derived from children

## Spec References

- FS-032.1 (settings panel entry & tab routing)
- FS-032.2 (settings save dispatch & per-tab validation)
- FS-032.3 (System settings tab fields)
- FS-032.4 (Ticket settings & options tab fields)
- FS-032.5 (Email settings tab fields)
- FS-032.6 (Site Pages, Logos, Autoresponder, Knowledge-Base & Alerts tabs)
- FS-032.7 (settings persistence semantics)
- BS-032.4 (checkbox settings persist as 0/1 by presence)
- BS-032.6 (attachment validation conditional on master switch)

## Context

Milestone: M4 — Admin Configuration. First user-facing admin epic; it also lands the **shared admin
panel FE shell** (routing, admin gate, nav) consumed by every other M4 screen. The ~110 config keys
were seeded by EPIC-M4-PREP; this epic exposes them as editable tabs and persists edits.

## Description

Deliver the multi-tab System Settings screen: seven `t`-routed tabs — System, Ticket Settings & Options,
Email, Site Pages, Knowledgebase, Autoresponder, Alerts & Notices — plus a **separate** Attachments
settings screen (standalone sub-route, not one of the seven tabs, per FS-032.1). Each tab reads its keys, validates per FS-032.2, and persists by
presence semantics (BS-032.4). The Pages and Email tabs bind against the `page` and `email_account`
rows (M4-D1). Cosmetic KLs are modernised: `send_sys_errors` renders as stored (KL-032.3 fixed).

## Business value

The operator tunes global helpdesk behaviour — page size, login window, password aging, attachment
limits, default department/SLA/pages — from one screen. These keys feed the whole app (pagination,
auth, routing), making M4-A the widest-reaching epic by downstream effect (Integration AC-7).

## Acceptance Criteria

> No ACs — intermediate parent; verification owned by leaf tickets. Column derived from children.

## Deviation record

- **DEV-M4-A1 (FS-032.1)**: the **Attachments** settings screen is delivered as a **separate screen /
  sub-route** (`/staff/admin/settings/attachments`), not as a 7th tab — matching the legacy out-of-band
  `admin.php?t=attach` partial. All **seven** `t`-routed tabs are delivered, **including Knowledgebase**
  (System, Ticket Settings & Options, Email, Site Pages, Knowledgebase, Autoresponder, Alerts & Notices).

## Children (column derived: min of these)

- [ ] [US-M4-A1](US-M4-A1-admin-edits-system-ticket-settings.md) — Admin edits System & Ticket settings tabs
  - [ ] [TS-M4-A0](TS-M4-A0-admin-shell-nav-gate.md) — Frontend: admin panel shell + nav + admin-gate routing
  - [ ] [TS-M4-A1](TS-M4-A1-settings-endpoints-validation.md) — Backend: settings GET/PUT endpoints + per-tab validation + persistence
  - [ ] [TS-M4-A2](TS-M4-A2-settings-system-ticket-ui.md) — Frontend: System + Ticket settings tabs UI
- [ ] [US-M4-A2](US-M4-A2-admin-edits-email-pages-alerts-attachments.md) — Admin edits Email, Pages, Knowledgebase, Alerts tabs & the Attachments screen
  - [ ] [TS-M4-A3](TS-M4-A3-settings-email-pages-alerts-attachments-ui.md) — Frontend: Email/Pages/Knowledgebase/Autoresponder/Alerts tabs + Attachments screen UI

## Dependencies

- **EPIC-M4-PREP**: config keys, `page` and `email_account` tables/rows.
- Blocks (FE shell): EPIC-M4-B/C/D/E/F/G/H all mount inside TS-M4-A0's admin shell.
