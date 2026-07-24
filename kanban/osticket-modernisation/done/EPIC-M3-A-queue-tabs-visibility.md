# EPIC-M3-A — EPIC – Queue Tabs + Visibility

- **ID**: EPIC-M3-A
- **Type**: Epic
- **Parent**: M3
- **Labels**: Epic, M3
- **Column**: derived from children

## Spec References

- FS-020.1 (queue entry & default landing)
- FS-020.2 (predefined queues / status tabs)
- FS-020.3 (ticket listing table & columns)
- FS-020.4 (visibility scoping: department + assignment)
- FS-020.11 (quick ticket stats for tab counts)

## Context

Milestone: M3 — Full Staff Workflow & Queue.
M1 delivered a minimal open-tickets list. This epic adds the full queue tab structure (Open,
Answered, My Tickets, Overdue, Closed) with real-time counts, and enforces department-based
visibility so agents only see tickets they should access.

## Description

Implement the predefined queue tabs driven by the `status` parameter, department+assignment
visibility scoping, the quick-stats counts for each tab, and the proper listing columns with
status-appropriate rightmost column (Assigned To / Closed By / Department per BS-020.5).

## Business value

Agents can quickly navigate between ticket states (open, answered, assigned to them, overdue,
closed) and see accurate counts. The visibility scoping ensures data security — agents only
see tickets in their departments or assigned to them, preventing unauthorized access.

## Acceptance Criteria

> No ACs — intermediate parent; verification owned by leaf TS tickets. Column derived from children.

## Children (column derived: min of these)

- [ ] [US-M3-A1](US-M3-A1-agent-views-queue-tabs-filters.md) — Agent views queue tabs with counts and filters by status
  - [ ] [TS-M3-A1](TS-M3-A1-status-param-quickstats.md) — Backend: status param handling + quick-stats endpoint
  - [ ] [TS-M3-A2](TS-M3-A2-visibility-scoping.md) — Backend: visibility scoping (dept + assignment predicate)
  - [ ] [TS-M3-A3](TS-M3-A3-listing-columns-per-queue.md) — Backend: listing columns per queue (rightmost column logic)
  - [ ] [TS-M3-A4](TS-M3-A4-frontend-queue-tabs.md) — Frontend: queue tabs UI with counts + status routing

## Dependencies

- **M1 done**: provides the baseline queue route (`GET /api/staff/tickets`) and staff auth
- **TS-M3-prep**: the second department (Sales) and group-dept access are needed for visibility testing
