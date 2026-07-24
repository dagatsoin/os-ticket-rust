# EPIC-M3-C — EPIC – Close/Reopen/Assign Workflow

- **ID**: EPIC-M3-C
- **Type**: Epic
- **Parent**: M3
- **Labels**: Epic, M3
- **Column**: derived from children

## Spec References

- FS-021.7 (assign / reassign ticket)
- FS-021.8 (claim ticket — self-assign)
- FS-021.9 (release / unassign ticket)
- FS-021.10 (transfer between departments)
- FS-021.11 (close ticket)
- FS-021.12 (reopen ticket)

## Context

Milestone: M3 — Full Staff Workflow & Queue.
M1 delivered only reply; tickets couldn't be closed, assigned, or transferred. This epic adds
the core workflow actions that let agents manage ticket lifecycle: assign/claim/release/transfer
and close/reopen.

## Description

Implement the single-ticket workflow actions: assign to staff/team (with required comments),
claim (self-assign with auto-generated note), release/unassign, transfer to another department
(with SLA re-selection), close (clears overdue, credits closer), and reopen (annuls prior close
event). Each action logs an internal note and may fire alert emails (alert config is M4).

## Business value

Agents can fully manage ticket routing and lifecycle. A ticket can be picked up (claimed),
handed off to another agent/team (assign), moved to a different department (transfer), and
resolved (close) or brought back for more work (reopen). This is the core of daily helpdesk
operations.

## Acceptance Criteria

> No ACs — intermediate parent; verification owned by leaf tickets. Column derived from children.

## Children (column derived: min of these)

### User Stories

- [ ] [US-M3-C1](US-M3-C1-agent-claims-assigns-transfers.md) — Agent claims, assigns, and transfers a ticket
- [ ] [US-M3-C2](US-M3-C2-agent-closes-reopens.md) — Agent closes and reopens a ticket

### Technical Stories

- [ ] [TS-M3-C1](TS-M3-C1-backend-assign-claim.md) — add(route): BS-021.8 assign/claim routes + validation + internal notes
- [ ] [TS-M3-C2](TS-M3-C2-backend-transfer.md) — add(route): FS-021.10 transfer route + dept change + SLA re-select
- [ ] [TS-M3-C3](TS-M3-C3-backend-close-reopen.md) — add(route): FS-021.11/12 close/reopen routes + lifecycle events
- [ ] [TS-M3-C4](TS-M3-C4-frontend-workflow-ui.md) — add(ui): EPIC-M3-C workflow action buttons + dialogs

## Scope Decisions (M3)

1. **Release (unassign) deferred** — FS-021.9 release/unassign requires department-manager status
   (the ticket's department manager). M3 does not implement department-manager determination;
   release is deferred to M4 when the manager flag lands.

2. **Assignment alerts deferred** — FS-021.16 specifies assignment alert e-mails to the assignee/team.
   Alert infrastructure is M4; M3 logs the internal note but does not fire email alerts.

3. **Transfer alerts deferred** — same as above; transfer logs a note but no email alerts in M3.

4. **Lifecycle event table** — ticket_event with columns (event_id, ticket_id, staff_id, team_id,
   dept_id, topic_id, timestamp, state, username, annulled). States include: created, closed,
   reopened, assigned, transferred, overdue. The annulled flag supports BS-021.14 (reopen annuls
   prior close).

## Dependencies

- **EPIC-M3-A**: visibility scoping affects which tickets the agent can act on
- **EPIC-M3-D**: close clears overdue; transfer re-selects SLA — the SLA infrastructure should be in place
- **TS-M3-prep**: teams (for team assignment), second department (for transfer testing), permission flags
