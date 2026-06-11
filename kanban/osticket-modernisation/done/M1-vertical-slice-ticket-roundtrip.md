# M1 — MILESTONE – First Ticket Round-Trip (Vertical Slice)

- **ID**: M1
- **Type**: Milestone (root ticket)
- **Parent**: — (top of hierarchy)
- **Labels**: Milestone
- **Column**: derived from children (`min(children.columns)`)

## Spec References

- FS-091 (data model — ticket/thread/staff/group/dept/session/config subset)
- FS-003 (validation / crypto / formatting infrastructure)
- FS-001 (bootstrap / realm gates / CSRF)
- FS-002 (staff auth / sessions / one permission gate)
- FS-010 (client portal lookup / view / reply)
- FS-011 (public ticket submission web form)
- FS-021 (staff single-ticket workflow — reply + status)
- FS-020 (staff queue — minimal open-tickets list)
- FS-040 (outbound mail — stubbed no-op log mailer behind a port)

## Description

The first end-to-end vertical slice of the modernised osTicket. It proves the new stack
(Rust/Axum + React/TS + PostgreSQL) can carry the **shared ticket create-and-append core**
through its two earliest channels — the public web form and the staff UI — plus the client
portal read/reply path.

Business value: demonstrates a working, demonstrable helpdesk loop to stakeholders. A
customer submits a request, an agent picks it up and answers, the customer reads the answer —
the irreducible heart of a support-ticket product. Everything in later milestones (attachments,
real email, admin config, API, KB) hangs off this core.

Scope deliberately excludes: CAPTCHA, banlist, throttling, autoresponse emails, attachments,
canned responses, assignment/transfer, SLA, search, admin CRUD, real email send/fetch. The
single dept + group + staff account are seeded by migration/fixture (no admin UI yet).

## Acceptance Criteria (root E2E — browser-only journey, no [API-ONLY])

> One continuous browser journey covering every descendant US. Validated by qa-criterion-tester
> against the running frontend once all children reach `qa`. The journey runs WITHOUT re-seeding
> between steps — state created in AC-1 flows through to AC-5. Reseed (`cargo run -p tools --bin
> seed`) is the ONLY pre-flight setup, run once before AC-1; the carried-over ticket number +
> email created in AC-1 are the fixture for AC-3/AC-4/AC-5. The journey is the test.

### AC-1 (US-M1-2): Visitor opens "Open a New Ticket", submits, sees a ticket number. [BROWSER]
- Setup: ONE-TIME pre-flight only — reseed dev DB to a known state: `cargo run -p tools --bin seed`. Use a clean browser session.
- Navigate: http://localhost:3702/open
- Action: fill Name "Jane Doe", Email "jane@example.com", Subject "Printer broken", Message "My printer won't print.".
- Action: submit the form.
- Verify: a confirmation page shows a generated 6-digit ticket number. CARRY this {ticketNumber, email="jane@example.com"} forward to AC-3/AC-4/AC-5 — do not reseed.
- Status: [x]
  - Carried fixture: ticket number 822960, email jane@example.com (subject "Printer broken", message "My printer won't print.")

### AC-2 (US-M1-3): Agent logs in with seeded credentials and lands on the staff panel. [BROWSER]
- Navigate: http://localhost:3702/staff/login (same browser; staff realm uses the distinct ost_staff_sess cookie).
- Action: type Username "agent", Password "Agent123!", submit.
- Verify: the agent lands on the staff control panel (e.g. /staff/tickets).
- Status: [x]

### AC-3 (US-M1-3): Agent sees the just-created ticket in the Open-tickets queue. [BROWSER]
- Navigate: http://localhost:3702/staff/tickets
- Verify: the Open-tickets queue lists the AC-1 ticket — the carried-over ticket number and subject "Printer broken" are visible (newest first).
- Status: [x]

### AC-4 (US-M1-3): Agent opens the ticket, reads the message, posts a reply that appears. [BROWSER]
- Action: click the AC-1 ticket row in the queue.
- Verify: the detail thread shows the customer's original `M` message "My printer won't print.".
- Action: type "We are looking into your printer issue." in the reply box and post.
- Verify: the reply appears in the thread (as an agent `R` response) immediately after the refetch.
- Status: [x]

### AC-5 (US-M1-4): In a fresh session, the customer logs into the client portal and sees the reply. [BROWSER]
- Setup: open a FRESH browser session (no staff cookie) to keep the client realm separate.
- Navigate: http://localhost:3702/tickets (client portal login).
- Action: enter the AC-1 ticket number + email "jane@example.com", submit.
- Verify: login succeeds and the read-only thread shows the customer's original `M` message followed by the agent's `R` reply "We are looking into your printer issue.", in order, with no internal note shown.
- Status: [x]

## Children (column derived: min of these)

- [ ] EPIC-M1-A — Foundation & Workspace Setup
- [ ] EPIC-M1-B — Ticket Round-Trip (client open → staff reply → client view)

## Dependencies

- None external. EPIC-M1-A (foundation) must be in place before EPIC-M1-B leaf work can integrate,
  but B's UI/contract design can proceed in parallel against mocked backend responses.

## Decisions to confirm

- **Database: PostgreSQL** (legacy was MySQL). Flagged as a decision point — see ROADMAP.md.
