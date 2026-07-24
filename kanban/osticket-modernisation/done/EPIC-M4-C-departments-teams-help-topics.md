# EPIC-M4-C — EPIC – Departments, Teams & Help Topics

- **ID**: EPIC-M4-C
- **Type**: Epic
- **Parent**: M4
- **Labels**: Epic, M4
- **Column**: derived from children

## Spec References

- FS-030.3–.7 (department list, add/edit form, create/update, deletion & re-homing, bulk actions)
- FS-030.8–.12 (team list, add/edit form, create/update/member-removal, deletion, bulk actions)
- FS-030.13–.17 (help topic list, add/edit form, create/update, deletion & cleanup, bulk actions)
- BS-030-01..11 (department rules), BS-030-12..17 (team rules), BS-030-18..27 (topic rules)
- BS-030-04/05/06/07 (default-dept protection & delete re-home), BS-030-20 (one-level nesting)
- BS-030-21/23 (topic routing + SLA override, precedence owned by FS-021.13)

## Context

Milestone: M4 — Admin Configuration. The **most cross-linked** epic: it consumes EPIC-M4-A
(settings/email), EPIC-M4-B (managers, team leads, topic auto-assignees, dept-access groups),
EPIC-M4-D (SLA select on dept/topic) and EPIC-M4-F (thank-you page on topic). Built **last among
routing objects** so its selects resolve against real rows (Integration ACs 1, 2, 3, 5, 8).

## Description

- **Departments**: list; add/edit with required Email + Template selects (M4-D1), manager,
  SLA, priority, group-access matrix (full-replace sync), auto-response overrides, public/private;
  delete re-homes dependents to the default department; default department is protected
  (not private, not deletable, disabled checkbox). No-change update guard modernised (KL-030-11 fixed).
- **Teams**: list; add/edit with lead (must be a member) + member-removal only; delete releases
  associations. "Last Updated" sort modernised (KL-030-12 fixed).
- **Help Topics**: list; add/edit routing (dept + priority required, optional SLA override, optional
  auto-assign staff-OR-team, optional thank-you page, one-level parent nesting — KL-030-02
  preserved); delete promotes children to top-level and clears references.

## Business value

Routing and ownership become operator-configurable: where tickets land, who owns them, which SLA and
which post-submission page apply. This is the functional heart of the admin panel.

## Acceptance Criteria

> No ACs — intermediate parent; verification owned by leaf tickets. Column derived from children.

## Children (column derived: min of these)

- [ ] [US-M4-C1](US-M4-C1-admin-manages-departments.md) — Admin manages departments
  - [ ] [TS-M4-C1](TS-M4-C1-department-crud-endpoints.md) — Backend: department CRUD + delete re-home + default protection + group-access sync
  - [ ] [TS-M4-C2](TS-M4-C2-department-list-form-ui.md) — Frontend: department list + add/edit form UI
- [ ] [US-M4-C2](US-M4-C2-admin-manages-teams.md) — Admin manages teams
  - [ ] [TS-M4-C3](TS-M4-C3-team-crud-endpoints.md) — Backend: team CRUD + lead/member-removal + delete release
  - [ ] [TS-M4-C4](TS-M4-C4-team-list-form-ui.md) — Frontend: team list + add/edit form UI
- [ ] [US-M4-C3](US-M4-C3-admin-manages-help-topics.md) — Admin manages help topics
  - [ ] [TS-M4-C5](TS-M4-C5-help-topic-crud-endpoints.md) — Backend: help-topic CRUD + routing fields + one-level nesting + delete promote
  - [ ] [TS-M4-C6](TS-M4-C6-help-topic-list-form-ui.md) — Frontend: help-topic list + add/edit form UI

## Dependencies

- **EPIC-M4-PREP**: department additive columns, email_account/template_group rows.
- **EPIC-M4-A**: admin shell + settings (default bindings).
- **EPIC-M4-B**: staff (manager/lead/auto-assign) + groups (dept-access matrix).
- **EPIC-M4-D**: SLA plans (dept/topic SLA select).
- **EPIC-M4-F**: pages (thank-you page on topic).
