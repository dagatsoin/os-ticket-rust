# EPIC-M3-B — EPIC – Sorting + Pagination

- **ID**: EPIC-M3-B
- **Type**: Epic
- **Parent**: M3
- **Labels**: Epic, M3
- **Column**: derived from children

## Spec References

- FS-020.5 (sortable columns & sticky sort)
- FS-020.6 (pagination & page size)
- FS-090.19 (page size resolution: personal -> system default -> DEFAULT_PAGE_LIMIT)

## Context

Milestone: M3 — Full Staff Workflow & Queue.
M1's queue had no sorting or pagination. This epic adds sortable column headers with per-queue
sticky sort memory, and proper pagination controls with a configurable page size.

## Description

Implement sortable columns (Date, ID, Priority, Subject, Name, Assignee, Department) with
clickable headers that toggle sort direction, session-based sticky sort per queue, and
pagination with page-size override. The default sort varies by queue (FS-020.5): Overdue
queue sorts by urgency+duedate, Closed by close-date, others by priority+effective-date.

## Business value

Agents working large queues can quickly find relevant tickets by sorting (newest first, highest
priority, etc.) and page through results without loading everything at once. The sticky sort
means their preferred ordering persists as they navigate tabs.

## Acceptance Criteria

> No ACs — intermediate parent; verification owned by leaf TS tickets. Column derived from children.

## Children (column derived: min of these)

- [ ] [US-M3-B1](US-M3-B1-agent-sorts-queue-paginate.md) — Agent sorts the queue and paginates through results
  - [ ] [TS-M3-B1](TS-M3-B1-sort-param-default-sorts.md) — Backend: sort param handling + per-queue default sorts
  - [ ] [TS-M3-B2](TS-M3-B2-sticky-sort-session.md) — Backend: sticky sort (session-based per queue)
  - [ ] [TS-M3-B3](TS-M3-B3-pagination-limit-count.md) — Backend: pagination with limit param + total count
  - [ ] [TS-M3-B4](TS-M3-B4-frontend-sort-pagination-ui.md) — Frontend: sortable column headers + pagination controls

## Dependencies

- **EPIC-M3-A**: the queue tabs/visibility must exist before sorting/pagination adds value
- **TS-M3-prep**: `default_page_size` config key for page-size resolution
