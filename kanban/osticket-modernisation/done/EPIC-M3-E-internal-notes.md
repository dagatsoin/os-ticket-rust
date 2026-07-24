# EPIC-M3-E — EPIC – Internal Notes

- **ID**: EPIC-M3-E
- **Type**: Epic
- **Parent**: M3
- **Labels**: Epic, M3
- **Column**: derived from children

## Spec References

- FS-021.4 (post internal note)
- FS-021.22 (thread model — Note type N)

## Context

Milestone: M3 — Full Staff Workflow & Queue.
M1 had only replies (type R); agents couldn't post private notes. This epic adds internal
notes — private thread entries visible only to staff, not to the ticket requester.

## Description

Implement the internal note posting flow: a note is stored as thread entry type `N`, with
poster name and staff_id, never emailed to the requester. The note form may optionally
include a state-change dropdown (close/reopen/answered/unanswered/overdue/notdue) — the
state change is applied if the agent has permission. Staff note alerts are M4 (alert config).

## Business value

Agents can document internal discussions, escalation notes, and investigation progress without
exposing them to customers. This is essential for team collaboration on complex tickets.

## Acceptance Criteria

> No ACs — intermediate parent; verification owned by leaf TS tickets. Column derived from children.

## Children (column derived: min of these)

- [ ] [US-M3-E1](US-M3-E1-agent-posts-internal-note.md) — Agent posts an internal note on a ticket
  - [ ] [TS-M3-E1](TS-M3-E1-post-note-endpoint.md) — Backend: POST note endpoint + validation
  - [ ] [TS-M3-E2](TS-M3-E2-note-state-change.md) — Backend: note-form state change (close/reopen via note)
  - [ ] [TS-M3-E3](TS-M3-E3-frontend-note-form-display.md) — Frontend: note form UI + thread display of notes (staff-only)

## Dependencies

- **M1 done**: thread model already supports entry types; need to add `N` handling
- **EPIC-M3-C**: the state-change dropdown triggers close/reopen which are in EPIC-M3-C
