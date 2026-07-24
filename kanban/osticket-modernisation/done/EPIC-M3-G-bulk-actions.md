# EPIC-M3-G — EPIC – Bulk Actions

- **ID**: EPIC-M3-G
- **Type**: Epic
- **Parent**: M3
- **Labels**: Epic, M3
- **Column**: derived from children

## Spec References

- FS-020.9 (mass / bulk actions from the queue)
- FS-021.21 (mass ticket actions handler)

## Context

Milestone: M3 — Full Staff Workflow & Queue.
M1 had no bulk operations. This epic adds the ability to select multiple tickets and apply
a single action (close, reopen, delete) to all of them at once.

## Description

Implement bulk actions: per-row checkboxes (shown only when staff canManageTickets), select-all
helper, and a bulk action bar with queue-appropriate actions (Closed queue -> Reopen; Open ->
Close; any queue with delete permission -> Delete). Each action is permission-checked per-item
and reports partial success if some items fail.

## Business value

Agents can efficiently process backlogs — bulk-close resolved tickets, bulk-reopen mistakenly
closed ones, or bulk-delete spam. This dramatically speeds up queue maintenance compared to
one-by-one processing.

## Acceptance Criteria

> No ACs — intermediate parent; verification owned by leaf TS tickets. Column derived from children.

## Children (column derived: min of these)

- [ ] [US-M3-G1](US-M3-G1-agent-bulk-closes-tickets.md) — Agent bulk-closes, reopens, or deletes multiple tickets
  - [ ] [TS-M3-G1](TS-M3-G1-mass-process-endpoint.md) — Backend: mass_process endpoint + per-action permission
  - [ ] [TS-M3-G2](TS-M3-G2-bulk-action-bar-ui.md) — Frontend: checkboxes + bulk action bar UI

## Dependencies

- **EPIC-M3-A**: the listing provides the ticket rows to select
- **EPIC-M3-C**: the close/reopen actions are reused from single-ticket workflow
- **TS-M3-prep**: the agent's group needs `can_close_tickets` and `can_delete_tickets` flags for bulk action visibility
