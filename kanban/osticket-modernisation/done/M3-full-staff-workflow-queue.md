# M3 — MILESTONE – Full Staff Workflow & Queue

- **ID**: M3
- **Type**: Milestone (root ticket)
- **Parent**: — (top of hierarchy)
- **Labels**: Milestone
- **Column**: derived from children (`min(children.columns)`)

## Spec References

- FS-020 (staff ticket queue: tabs, visibility scoping, sort/paginate, mass actions, CSV export, basic/advanced search, dashboard)
- FS-021 (single-ticket workflow: assign/claim/release/transfer, close/reopen, internal notes, overdue/SLA, edit/delete, locking)

## Description

Promotes M1's minimal staff view into the complete agent workspace. This milestone covers the
**full queue experience** (status tabs with visibility scoping, sortable/paginated columns,
search, mass actions) and the **complete single-ticket workflow** (assignment, transfer, internal
notes, close/reopen, overdue/SLA handling, edit/delete, collaborative locking).

Business value: transforms the proof-of-concept staff panel into a production-ready agent
workspace. Agents can efficiently manage their queue (filter by status, sort, search, bulk
close/reopen), work individual tickets (assign/transfer/note/close), and handle SLA-driven
priorities. This is the milestone that makes the staff panel genuinely usable for daily support
operations.

## Scope

**In scope (M3)**:
- Queue tabs: Open/Answered/My Tickets/Overdue/Closed with quick-stat counts
- Department-based visibility scoping (you see tickets in your departments + your assignments)
- Sortable columns with sticky per-queue sort memory
- Pagination with configurable page size
- Basic keyword search (number/email/free-text) and advanced search dialog
- Mass close/reopen/delete actions
- Close and reopen individual tickets
- Assign to staff/team, claim, release, transfer between departments
- Internal notes (with optional state change)
- SLA plans: seed, due-date computation, mark/clear overdue, overdue queue filter
- Edit ticket properties (subject, dept, priority, SLA, due date)
- Delete ticket
- Collaborative edit locking (auto-lock on view, lock renewal, block conflicting replies)

**Explicitly deferred**:
- Staff-initiated (phone) ticket creation (FS-021.20) -> M4 or later
- PDF print (FS-021.17) -> M4 or later
- Ban/unban requester email (FS-021.14) -> M4 or later (needs banlist infra)
- Dashboard activity chart and statistics (FS-020.12-13) -> M4
- CSV export (FS-020.10) -> M4

## Acceptance Criteria (root E2E — browser-only journey, no [API-ONLY])

> One continuous browser journey covering the M3 workflow expansion. Validated by qa-criterion-tester
> against the running frontend once all children reach `qa`. Run reseed (`cargo run -p tools --bin seed --reset`)
> ONCE before AC-1; the journey then flows without re-seeding — state created in AC-1 is used by AC-2+.

### AC-1 (US-M3-A1): Agent sees queue tabs with counts (My Tickets appears only after a claim) and can filter by status. [BROWSER]
> Per FS-020.2 / BS-020.14 (legacy scp/tickets.php:536-544 `if($stats['assigned'])`): with the standard queue
> config (`show_answered_tickets=0`), Open / Overdue / Closed always render with count badges; "Answered" is
> hidden while `show_answered_tickets=0`; and "My Tickets" (status=assigned) renders ONLY once the agent has
> ≥1 assigned ticket (it is not gated on `show_assigned_tickets` and shows no count-0 state).
- Setup: ONE-TIME pre-flight — reseed dev DB with M3 seed data (2 depts, SLA plans, teams): `cargo run -p tools --bin seed --reset`.
- Navigate: http://localhost:3702/staff/tickets
- Action: login as `agent` / `Agent123!` if not already logged in.
- Verify: the queue shows Open, Overdue and Closed tabs with count badges.
- Verify: the "Answered" tab is hidden (`show_answered_tickets=0`).
- Verify: the "My Tickets" tab is absent initially (agent has no assigned tickets yet).
- Action: open an unassigned open ticket and click "Claim Ticket", then return to the queue.
- Verify: the "My Tickets" tab now appears with its count.
- Action: click "Closed" tab.
- Verify: the queue updates to show only closed tickets (or empty state if none).
- Action: click "Open" tab to return.
- Status: [x]

### AC-2 (US-M3-A1): Agent sorts the queue and the sort persists on tab return. [BROWSER]
- Action: click the "Date" column header to sort by date ascending.
- Verify: the sort indicator shows ascending, tickets reorder by date.
- Action: click "Closed" tab then click "Open" tab again.
- Verify: the Open queue remembers the Date-ascending sort (sticky sort per queue).
- Status: [x]

### AC-3 (US-M3-B1): Agent searches by keyword and sees results. [BROWSER]
- Setup: ensure at least one ticket exists with a known subject (from prior milestones or seed).
- Action: type a keyword from the subject in the search box, press Enter.
- Verify: the queue filters to matching tickets with "(Search Results)" label.
- Action: clear the search to return to the normal Open queue.
- Status: [x]

### AC-4 (US-M3-C1): Agent assigns an unassigned ticket to themselves (claim). [BROWSER]
- Setup: create a new ticket via /open form (client) if no unassigned ticket exists.
- Action: open an unassigned open ticket from the queue.
- Action: click "Claim Ticket" button.
- Verify: success message "Ticket is now assigned to you!", the ticket shows as assigned to the agent.
- Status: [x]

### AC-5 (US-M3-C1): Agent transfers a ticket to another department. [BROWSER]
- Action: on the same ticket, click "Transfer" or open transfer dialog.
- Action: select the "Sales" department (seeded second dept), enter comment >= 5 chars, submit.
- Verify: success message "Ticket transferred successfully", department shows "Sales".
- Note: if agent loses access to Sales dept, they return to the queue (expected behavior).
- Status: [x]

### AC-6 (US-M3-E1): Agent posts an internal note on a ticket. [BROWSER]
- Setup: navigate to an accessible open ticket.
- Action: click "Post Internal Note" or open note form.
- Action: type note body "Internal investigation started", submit.
- Verify: the note appears in the thread marked as internal (not visible to client).
- Status: [x]

### AC-7 (US-M3-C2): Agent closes and reopens a ticket. [BROWSER]
- Setup: navigate to an open ticket.
- Action: click "Close Ticket", enter optional comment, confirm.
- Verify: success message "Ticket status set to CLOSED", agent returns to queue, ticket no longer in Open tab.
- Action: navigate to Closed tab, open the just-closed ticket.
- Action: click "Reopen Ticket", enter optional comment, confirm.
- Verify: success message "Ticket REOPENED", ticket returns to open status.
- Status: [x]

### AC-8 (US-M3-D1): An overdue ticket appears in the Overdue tab. [BROWSER]
- Setup: one of the seeded SLA plans (e.g., "Urgent 4h") has a short grace period for testing, OR mark a ticket overdue manually via the note-form state dropdown (manager required).
- Verify: the Overdue tab shows tickets past their SLA due date.
- Note: if manual mark is used, navigate to a ticket, open note form, select state "Mark Overdue" (requires dept manager), post the note.
- Status: [x]

### AC-9 (US-M3-G1): Agent bulk-closes multiple tickets. [BROWSER]
- Setup: ensure at least 2 open tickets exist.
- Action: on the Open queue, check the checkboxes for 2 tickets.
- Action: click "Close" bulk action, confirm.
- Verify: success message "Selected tickets closed", the tickets move to Closed state.
- Status: [x]

### AC-10 (US-M3-I1): Agent edits ticket properties. [BROWSER]
- Setup: navigate to an open ticket.
- Action: click "Edit" button to open edit view.
- Action: change the priority (e.g., from "Normal" to "High"), enter a reason note, save.
- Verify: success message "Ticket updated successfully", priority change reflects on the ticket.
- Status: [x]

### AC-11 (US-M3-I2): Collaborative locking blocks a conflicting reply from a DIFFERENT staff member. [BROWSER]
> Per FS-021.18/FS-021.20, BS-021.3 (legacy class.ticket.php:349-369, class.lock.php, staff/ticket-view.inc.php:36):
> the lock is keyed by STAFF (ticket_id + staff_id), so a genuine conflict requires TWO DIFFERENT staff accounts.
> A same-staff second session merely renews the owner's own lock and is NOT blocked.
- Setup: staff A `agent` / `Agent123!` and staff B `agent2` / `Agent123!` (second seeded account, TS-M3-prep) both exist.
- Action: in Browser 1, log in as staff A `agent` and open a ticket (staff A holds the lock).
- Action: in a SECOND browser session (incognito or different browser), log in as staff B `agent2` (a DIFFERENT staff member).
- Action: navigate to the same ticket.
- Verify: staff B's session sees the warning "This ticket is currently locked by [staff A's name]" and cannot post a reply until staff A's lock expires or is released.
- Verify: separately, staff A opening the same ticket in a SECOND same-staff session is NOT blocked and sees no spurious lock warning (idempotent acquire).
- Status: [x]

## Children (column derived: min of these)

- [ ] EPIC-M3-A — Queue Tabs + Visibility
- [ ] EPIC-M3-B — Sorting + Pagination
- [ ] EPIC-M3-C — Close/Reopen/Assign Workflow
- [ ] EPIC-M3-D — SLA + Overdue
- [ ] EPIC-M3-E — Internal Notes
- [ ] EPIC-M3-F — Search
- [ ] EPIC-M3-G — Bulk Actions
- [ ] EPIC-M3-I — Edit + Delete + Lock

## Seed Data Requirements (TS-M3-prep)

M3 requires expanded seed data beyond M1/M2:
- Second department: "Sales" (in addition to existing "Support")
- Team: "Tier 2" with the agent as a member
- SLA plans: 2 plans — "Standard" (24h grace) and "Urgent" (4h grace)
- Help topics: 2 topics — "General" (default) and "Billing"
- Group-dept access: the agent's group has access to both Support and Sales
- Config keys: `show_assigned_tickets`, `show_answered_tickets`, `ticket_lock_time`, `max_page_size`
- Second staff account: `agent2` / `Agent123!` (in addition to `agent`) — required for the two-DIFFERENT-staff
  collaborative-lock conflict in AC-11 / US-M3-I2 (the lock is keyed by staff_id, so a genuine conflict needs
  two distinct staff accounts).

## Dependencies

- **M1 (done)**: provides the baseline ticket/thread/staff/session/auth infrastructure
- **M2 (in qa)**: provides attachments in thread display (thread entries may have attachments)
- **EPIC-M3-A** (tabs/visibility) should land before search/bulk (F/G) for coherent queue UX
- **EPIC-M3-D** (SLA) should land early — other epics reference due-date/overdue state
- **EPIC-M3-I** (locking) is standalone and can proceed in parallel

## Decisions (M3)

1. **Lock time default** — seed `ticket_lock_time = 2` (minutes). The lock renewal AJAX is owned by
   TS-M3-I3; the lock table was seeded in M1 schema (FS-091).

2. **Visibility model** — department-based (FS-020.4). The agent sees tickets in their accessible
   departments + tickets directly assigned to them + tickets assigned to their teams (for open tickets).
   This is simpler than the full "assigned-only" toggle — the `showAssignedOnly` staff flag is M4.

3. **Manager gate for overdue/answered/release** — M3 does NOT implement department-manager status;
   those state-change actions (FS-021.13) are hidden or gated. The agent can close/reopen (their group
   has `can_close_tickets`), but manual mark-overdue / mark-answered / release require the M4 manager flag.
