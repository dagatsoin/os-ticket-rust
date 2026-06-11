# M4 — MILESTONE – Admin Configuration

- **ID**: M4
- **Type**: Milestone (root, stub)
- **Status**: Backlog stub — not yet broken down.

## Spec References

- FS-030 (departments, teams, help topics)
- FS-031 (staff accounts, permission groups, staff directory, profile editing)
- FS-032 (system settings ~110 keys, SLA plans, priorities, FAQ categories)
- FS-033 (system-log viewer + content/config AJAX endpoints)

## Description

Replaces M1's seed-only configuration with a full admin panel: CRUD for departments, teams,
help topics, staff accounts and permission groups; the seven-tab system settings; SLA plans
and priorities; and the system-log viewer. This removes the need for migration-seeded config
and lets admins shape routing, authorization, and SLA behaviour.

## Acceptance Criteria (to be defined as a browser-only E2E during consolidation)

- Status: [ ] (placeholder) Admin creates a department, a permission group, and a staff account, adjusts a system setting, and the new staff can log in and is routed correctly.

## Notes

- Not refined. Requires spec-writer dependency-graph confirmation before consolidation.
- Supersedes the M1 seed fixture (TS-M1-A3) as the source of dept/group/staff once shipped.
