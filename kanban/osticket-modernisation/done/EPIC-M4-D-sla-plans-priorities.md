# EPIC-M4-D — EPIC – SLA Plans & Priorities

- **ID**: EPIC-M4-D
- **Type**: Epic
- **Parent**: M4
- **Labels**: Epic, M4
- **Column**: derived from children

## Spec References

- FS-032.8 (ticket priority reference set)
- FS-032.9 (SLA plan listing & mass actions)
- FS-032.10 (SLA plan create / edit form)
- FS-032.11 (SLA overdue computation & behavioral hooks — referenced; math owned by M3)
- FS-032.12 (SLA plan deletion constraints & reassignment)
- BS-032.13 (priorities are a fixed, urgency-ranked set)

## Context

Milestone: M4 — Admin Configuration. M3 seeded SLA plans directly; this epic gives them an admin CRUD
surface and exposes the read-only priority reference set. Its SLA rows are consumed by EPIC-M4-C
(department/topic SLA select) and protected against deletion when bound (Integration AC-1).

## Description

- **SLA plans**: list with mass activate/disable/delete; create/edit (name, grace period hours,
  transient/active flags, overdue-alert options); deletion constraints + reassignment of dependents
  to the default SLA. The default SLA is delete-protected. KL-032.4 (copy-paste error text) and
  KL-032.10 ("Date Added" not sortable) modernised.
- **Priorities**: a **read-only** reference panel over the fixed, urgency-ranked set (KL-032.1
  preserved — no priority CRUD).

## Business value

Operators define service-level commitments and see the priority scale that drives urgency, without
editing seed SQL. SLA rows become selectable throughout routing.

## Acceptance Criteria

> No ACs — intermediate parent; verification owned by leaf tickets. Column derived from children.

## Children (column derived: min of these)

- [ ] [US-M4-D1](US-M4-D1-admin-manages-sla-plans.md) — Admin manages SLA plans
  - [ ] [TS-M4-D1](TS-M4-D1-sla-crud-endpoints.md) — Backend: SLA plan CRUD + deletion constraints/reassignment + priority read endpoint
  - [ ] [TS-M4-D2](TS-M4-D2-sla-list-form-ui.md) — Frontend: SLA plan list + create/edit form UI
- [ ] [US-M4-D2](US-M4-D2-admin-views-priority-set.md) — Admin views the read-only priority reference set
  - [ ] [TS-M4-D3](TS-M4-D3-priorities-readonly-ui.md) — Frontend: read-only priorities reference panel

## Dependencies

- **EPIC-M4-PREP**: sla_plan transient-trump key, default_sla_id.
- **EPIC-M4-A (shell)**: admin screens mount inside TS-M4-A0.
- Blocks: EPIC-M4-C (dept/topic SLA select).
