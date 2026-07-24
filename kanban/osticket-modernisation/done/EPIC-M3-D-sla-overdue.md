# EPIC-M3-D — EPIC – SLA + Overdue

- **ID**: EPIC-M3-D
- **Type**: Epic
- **Parent**: M3
- **Labels**: Epic, M3
- **Column**: derived from children

## Spec References

- FS-021.13 (due date, SLA selection & overdue marking)
- FS-032.11 (SLA grace period math — referenced, not owned)
- FS-020.2 (overdue queue tab)
- FS-091.6 (SLA model, sla_plan table)

## Context

Milestone: M3 — Full Staff Workflow & Queue.
M1/M2 had no SLA or overdue handling. This epic adds the SLA infrastructure: seeded SLA plans,
due-date computation based on SLA grace period, the overdue flag, and the Overdue queue tab.

## Description

Implement SLA plan support: seed two SLA plans (Standard 24h, Urgent 4h), add SLA selection
precedence logic (department > topic > system default per FS-021.13), compute effective due
date from created + grace_period, mark tickets overdue when past due, and surface them in the
Overdue queue tab. Clear overdue on close. The manual mark-overdue action (manager-only per
FS-021.13) is deferred until department-manager status is implemented in M4.

## Business value

Tickets are automatically tracked against service-level commitments. Overdue tickets surface
prominently so agents can prioritize urgent work. This is essential for any helpdesk with SLA
obligations to customers.

## Acceptance Criteria

> No ACs — intermediate parent; verification owned by leaf TS tickets. Column derived from children.

## Children (column derived: min of these)

- [ ] [US-M3-D1](US-M3-D1-overdue-tickets-appear-in-overdue-tab.md) — Overdue tickets appear in the Overdue tab
  - [ ] [TS-M3-D1](TS-M3-D1-sla-plan-schema.md) — Schema: sla_plan table
  - [ ] [TS-M3-D2](TS-M3-D2-sla-plan-seed.md) — Seed: two SLA plans (Standard 24h, Urgent 4h)
  - [ ] [TS-M3-D3](TS-M3-D3-sla-selection-duedate-computation.md) — Backend: SLA selection precedence + due-date computation
  - [ ] [TS-M3-D4](TS-M3-D4-overdue-detection-flag.md) — Backend: overdue detection + flag on listing
  - [ ] [TS-M3-D5](TS-M3-D5-frontend-overdue-tab-badge.md) — Frontend: Overdue tab + due-date/overdue badge display

## Dependencies

- **EPIC-M3-A**: the Overdue tab is one of the predefined queues
- **TS-M3-prep**: help topics (with SLA assignment) are seeded
- **Note**: the system overdue sweep (cron-driven checkOverdue, FS-043) is M6; M3 handles the flag + display but not the automated batch marking
