# US-M4-C3 — Admin manages help topics

- **ID**: US-M4-C3
- **Type**: User Story
- **Parent**: EPIC-M4-C
- **Labels**: User Story, M4
- **Scope**: medium

## Spec References

- FS-030.13 (help topic list), FS-030.14 (add/edit form), FS-030.15 (create & update)
- FS-030.16 (deletion & dependent cleanup), FS-030.17 (bulk actions)
- BS-030-18 (topic required/min/unique within parent), BS-030-19 (dept + priority required)
- BS-030-20 (one-level nesting — PRESERVED), BS-030-21 (topic routing/defaults)
- BS-030-22 (auto-assign staff-OR-team exclusive), BS-030-23 (SLA + thank-you page optional overrides)
- BS-030-26 (deletion promotes children + clears references)

## Context

Epic: EPIC-M4-C — Departments, Teams & Help Topics. Help topics drive ticket routing. This story
delivers the topic list and add/edit form: department + priority (required), optional SLA override,
optional auto-assign (staff OR team), optional thank-you page (from EPIC-M4-F), and one-level parent
nesting. Deleting a topic promotes its children to top-level.

## Description

As an administrator, I can list help topics and create or edit one: topic text, active/public,
required department + priority, an optional SLA override, an optional single parent (from top-level
topics only), optional auto-assignment to a staff member OR a team (never both), an optional
thank-you page, and auto-response override. Deleting a topic promotes its children and clears
references on tickets.

## Impact

- Frontend (topic list + add/edit form + bulk actions)
- Backend (topic CRUD + routing fields + one-level nesting + delete promote)
- Database (`help_topic`, `department`, `priority`, `sla_plan`, `staff`, `team`, `page`, `ticket`)
- Browser (desktop)

## Business Rules

- BS-030-18: topic required, >= 5 chars, unique among siblings.
- BS-030-19: department and priority required.
- BS-030-20: one-level nesting — the parent picker lists only top-level topics (`topic_pid=0`), and the server also rejects write-side any `parentId` pointing at a non-top-level topic (PRESERVED, KL-030-02).
- BS-030-22: auto-assign encodes staff OR team with an `s`/`t` prefix; selecting one clears the other.
- BS-030-23: SLA `0` = no topic SLA; empty thank-you page = system default (web tickets only).
- BS-030-26: delete promotes children to top-level and clears topic references.

## Regressions

- The M3 seeded "General"/"Billing" topics keep routing; SLA precedence stays owned by FS-021.13.

## Acceptance Criteria

### AC-1: The help-topic list renders (with Parent / Child display for nested topics). [BROWSER]
- Setup: `cargo run -p tools --bin seed` then `POST /api/dev/reset-help-topics` for clean state (retains the seeded "General"/"Billing" topics); ensure at least one child topic exists (create one in AC-2, or seed a parent+child).
- Setup: log in as admin (`admin` / `Admin123!`) at http://localhost:3702/staff/login.
- Navigate: http://localhost:3702/staff/admin/help-topics
- Verify: a paginated table of topics renders; a nested (child) topic displays as "Parent / Child".
- Status: [x]

### AC-2: Admin creates a topic with required dept + priority and an SLA override. [BROWSER]
- Setup: admin on /staff/admin/help-topics (from AC-1).
- Action: click "Add Help Topic"; type "Refund Request" (>=5 chars) into the topic text.
- Action: select a department (e.g. Support) and a priority (e.g. Normal); select an SLA override (e.g. "Default SLA").
- Action: click Save.
- Verify: success banner; "Refund Request" appears in the topic table (BS-030-18/19).
- Status: [x]

### AC-3: Auto-assign is mutually exclusive staff-or-team. [BROWSER]
- Setup: admin editing a topic (e.g. "Refund Request").
- Action: in the combined auto-assign control, choose a staff auto-assignee → the team selection clears; then choose a team → the staff selection clears.
- Action: click Save.
- Verify: only the last-chosen assignee (one of staff OR team) is retained on reopen (BS-030-22).
- Status: [x]

### AC-4: A thank-you page (from Site Pages) is selectable on the topic. [BROWSER]
- Setup: a `thank-you` page exists (create via `POST /api/dev/seed-page` or the EPIC-M4-F Pages screen).
- Action: admin edits a topic (e.g. "Refund Request"); select that thank-you page in the thank-you page select; click Save.
- Verify: the topic saves; on reopen the thank-you page remains bound (Integration AC-5).
- Status: [x]

### AC-5: One-level nesting — the parent picker lists only top-level topics. [BROWSER]
- Setup: admin on /staff/admin/help-topics with at least one existing child topic.
- Action: open the topic add/edit form; inspect the Parent select options.
- Verify: only topics with no parent are offered; a child topic is NOT selectable as a parent (PRESERVED KL-030-02 / BS-030-20).
- Status: [x]

### AC-6: Deleting a topic promotes its children to top-level. [API-ONLY]
- Setup: admin session cookie; a parent topic with one child (seed a parent then a child via the CRUD API or `seed-topic`).
- Request: `DELETE /api/staff/admin/help-topics/:parentId`.
- Headers: admin session cookie.
- Expect: 200; the former child now has a null parent (promoted to top-level) and any referencing tickets have the topic cleared (BS-030-26).
- Status: [x]

## Checklist (children)

- [ ] TS-M4-C5 — Backend: help-topic CRUD + routing fields + one-level nesting + delete promote
- [ ] TS-M4-C6 — Frontend: help-topic list + add/edit form UI

## Test Infrastructure

- Admin account (`admin`/`Admin123!`). Requires SLA plans (EPIC-M4-D), staff/teams (EPIC-M4-B), a thank-you page (EPIC-M4-F — `seed-page` exists). `seed-topic` dev endpoint already exists (minimal create for setup).
- Dev **`POST /api/dev/reset-help-topics`** — declared as a deliverable of TS-M4-C5 (`add(dev): reset-help-topics`); provided by the dev iteration, no consolidation gap.

## Dependencies

- **EPIC-M4-PREP**, **EPIC-M4-A (shell)**, **EPIC-M4-B**, **EPIC-M4-D (SLA)**, **EPIC-M4-F (thank-you page)**.
