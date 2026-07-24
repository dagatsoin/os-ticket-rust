# TS-M4-C6 — add(frontend): FS-030.13/.14 help-topic list + add/edit form UI

- **ID**: TS-M4-C6
- **Type**: Technical Story
- **Parent**: US-M4-C3
- **Labels**: Technical Story, M4
- **Scope**: medium

## Context

The help-topic screen inside the admin shell, wired to TS-M4-C5. Routing selects draw from
departments, priorities, SLA plans (EPIC-M4-D), staff/teams (EPIC-M4-B) and pages (EPIC-M4-F).

## Impact

- add(frontend): `HelpTopicListPage` — **paginated** table; child rows show "Parent / Child".
- add(frontend): `HelpTopicFormDialog` — text, active/public, required dept + priority selects,
  optional SLA-override select, parent select (top-level only), **a single mutually-exclusive
  auto-assign staff-OR-team control** (picking a staff clears any team and vice versa; encoded as
  `s<id>`/`t<id>`), thank-you page select, auto-response override; inline 422 + success banner.
  All selects source from `GET /api/staff/admin/help-topics/form-options`
  (`priorities`/`departments`/`sla`/`pages`/`staff`/`teams`/`parent_topics`).
- add(store): **`TopicAdminStore`** (MobX, TDD) — paginated list/form-options/create/update/delete.

## Regressions

- None; additive.

## Acceptance Tests

### AC-1: FS-030.13 — list renders; child shows "Parent / Child". [BROWSER]
- Setup: `cargo run -p tools --bin seed`; ensure a parent+child topic exist; log in as admin (`admin`/`Admin123!`) at http://localhost:3702/staff/login.
- Navigate: http://localhost:3702/staff/admin/help-topics
- Verify: the paginated topic table renders; a nested topic displays as "Parent / Child".
- Status: [x]

### AC-2: FS-030.14 — create with dept + priority + SLA override saves. [BROWSER]
- Setup: admin on /staff/admin/help-topics.
- Action: click "Add Help Topic"; type a >=5-char text; select a department + priority; select an SLA override; click Save.
- Verify: success banner; the topic appears in the table.
- Status: [x]

### AC-3: BS-030-22 — auto-assign staff/team mutual exclusion in the UI. [BROWSER]
- Setup: admin editing a topic.
- Action: in the auto-assign control, choose a staff → the team selection clears; choose a team → the staff selection clears.
- Verify: only one auto-assignee remains selected before Save.
- Status: [x]

### AC-4: BS-030-20 — parent select lists top-level topics only. [BROWSER]
- Setup: admin on the topic add/edit form with at least one child topic existing.
- Action: open the Parent select.
- Verify: no child topic is offered — only topics with a null parent.
- Status: [x]

### AC-5: Integration AC-5 — a thank-you page is selectable. [BROWSER]
- Setup: a thank-you page present (`POST /api/dev/seed-page` or EPIC-M4-F Pages screen); admin editing a topic.
- Action: select that thank-you page on the topic; click Save.
- Verify: the topic saves; on reopen the thank-you page remains bound.
- Status: [x]

## Test Infrastructure

- Vitest + RTL + MSW mocking paginated topic CRUD + `GET /api/staff/admin/help-topics/form-options`. Admin account.

## Dependencies

- **TS-M4-A0**: shell. **TS-M4-C5**: topic endpoints + form-options. **EPIC-M4-D/B/F**: routing selects.
- **Router**: adds `/staff/admin/help-topics` to the shared admin `<Route>` block in `router.tsx` —
  coordinate the single coherent router edit with C2/C4 + E2 (all C/E screens touch the same block).
