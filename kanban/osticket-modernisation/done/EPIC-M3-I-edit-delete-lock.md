# EPIC-M3-I — EPIC – Edit + Delete + Lock

- **ID**: EPIC-M3-I
- **Type**: Epic
- **Parent**: M3
- **Labels**: Epic, M3
- **Column**: derived from children

## Spec References

- FS-021.15 (edit ticket properties)
- FS-021.19 (delete ticket)
- FS-021.18 (collaborative edit locking)

## Context

Milestone: M3 — Full Staff Workflow & Queue.
M1 had no edit, delete, or locking. This epic adds the ability to edit ticket properties
(subject, priority, SLA, due date, etc.), permanently delete tickets, and collaboratively
lock tickets so two agents don't reply simultaneously.

## Description

Implement edit: change requester details, routing (dept/topic), priority, SLA, source, and
explicit due date (future only, clears overdue). A reason note is required. Implement delete:
permanently remove ticket + thread + attachments (cascading). Implement locking: auto-acquire
a lock when viewing a ticket, renew via AJAX, block reply when locked by another staff.

## Business value

- **Edit**: agents can correct mistakes (wrong department, priority) or update ticket metadata.
- **Delete**: admins can purge spam or test tickets.
- **Lock**: prevents two agents from double-replying to the same customer — a common support
  team frustration.

## Acceptance Criteria

> No ACs — intermediate parent; verification owned by leaf TS tickets. Column derived from children.

## Children (column derived: min of these)

- [ ] [US-M3-I1](US-M3-I1-agent-edits-ticket-properties.md) — Agent edits ticket properties
  - [ ] [TS-M3-I1](TS-M3-I1-edit-update-endpoint.md) — Backend: edit/update endpoint + validation
  - [ ] [TS-M3-I4](TS-M3-I4-edit-form-ui.md) — Frontend: edit form UI
- [ ] [US-M3-I2](US-M3-I2-collaborative-locking.md) — Collaborative locking blocks conflicting reply
  - [ ] [TS-M3-I3](TS-M3-I3-lock-acquire-renew-release.md) — Backend: lock acquire/renew/release endpoints
  - [ ] [TS-M3-I6](TS-M3-I6-lock-ui-warning-polling.md) — Frontend: lock UI (warning banner + auto-renew polling)
- [ ] [TS-M3-I2](TS-M3-I2-delete-endpoint-cascade.md) — Backend: delete endpoint + cascade
- [ ] [TS-M3-I5](TS-M3-I5-delete-confirmation-dialog.md) — Frontend: delete confirmation dialog

## Dependencies

- **M1 done**: ticket_lock table exists in M1 schema
- **EPIC-M3-D**: edit can change SLA, due date — the SLA infrastructure should be in place
- **TS-M3-prep**: `ticket_lock_time` config key for lock duration
