# US-M1-2 — Portal – Client opens a ticket via the web form

- **ID**: US-M1-2
- **Type**: User Story
- **Parent**: EPIC-M1-B
- **Labels**: User Story, M1
- **Scope**: medium

## Spec References

- FS-011 (public ticket submission web form)
- BS-011.* (required fields, help-topic routing — routing simplified to the single seeded dept for M1)
- FS-091 (ticket + thread tables)

## Context

Epic: EPIC-M1-B — Ticket Round-Trip.
The entry point of the whole product: a visitor with no account submits a support request.
This story builds the **shared ticket create-and-append core** (create ticket + first thread
message) that US-M1-3's staff reply will reuse.

## Description

As a visitor, I can open a new support ticket by filling in my name, email, subject, and
message, and I receive a ticket number confirming it was created.

## Impact

- Frontend (public "Open a New Ticket" form + confirmation page)
- Backend (ticket service core: create ticket + first thread message; create-ticket route)
- Database (ticket, ticket_thread)
- Browser (desktop)

## Business Rules

- BS-011: name, email, subject, and message are required; email must be valid.
- BS-011: a successful submission creates a ticket assigned to the seeded department with status Open, and a thread message holding the body.
- BS-011: each ticket gets a unique, user-visible ticket number.
- (Out of M1 scope, deferred to M2+: CAPTCHA, banlist, open-ticket throttle, attachments, autoresponse/alert email — the stub mailer records, but does not send, any intended autoresponse.)

## Regressions

- None (greenfield). Establishes the shared core; changes here later ripple to US-M1-3 reply.

## Acceptance Criteria

### AC-1: The public site shows an "Open a New Ticket" link/page with name, email, subject, and message fields. [BROWSER]
- Setup: reseed dev DB to a known state — `cargo run -p tools --bin seed` (idempotent; only writes `osticket_dev`).
- Navigate: http://localhost:3702/open
- Verify: the page renders an "Open a New Ticket" heading and four fields — Name, Email, Subject, Message (no help-topic / CAPTCHA / attachment fields in M1).
- Verify: the public landing at http://localhost:3702/ exposes a link/route to /open.
- Status: [x]

### AC-2: Submitting with a missing required field or invalid email shows an inline validation error and does not create a ticket. [BROWSER]
- Navigate: http://localhost:3702/open
- Action: leave Email empty (or type an invalid email like "not-an-email"), fill Name "Jane Doe", Subject "Printer broken", Message "It won't print.".
- Action: attempt to submit.
- Verify: an inline validation error appears on the offending field and the submit is blocked / returns a 422 mapped to the field; no confirmation page is shown.
- Verify (persistence): no new ticket appears in the staff queue — log in at /staff/login (agent / Agent123!), open the Open-tickets queue, confirm the invalid attempt produced no ticket. (Alternatively assert ticket count unchanged via GET /api/staff/tickets?status=open after a staff login.)
- Status: [x]

### AC-3: Submitting a valid form shows a confirmation page displaying a generated ticket number. [BROWSER]
- Navigate: http://localhost:3702/open
- Action: fill Name "Jane Doe", Email "jane@example.com", Subject "Printer broken", Message "My printer won't print.".
- Action: submit the form.
- Verify: a confirmation page renders a 6-digit ticket number (100000–999999). Record this number + email for downstream US-M1-3/US-M1-4 flows.
- Status: [x]

### AC-4: The created ticket persists with status Open, the seeded department, and a thread message containing the submitted body. [API-ONLY]
- Setup: create a ticket via the public route — `curl -s -X POST http://localhost:3701/api/tickets -H 'Content-Type: application/json' -d '{"name":"Jane Doe","email":"jane@example.com","subject":"Printer broken","message":"My printer won't print."}'` → capture the returned ticket number.
- Request: authenticate as staff then read the ticket detail — `curl -s -c /tmp/c.txt -X POST http://localhost:3701/api/staff/login -H 'Content-Type: application/json' -d '{"username":"agent","password":"Agent123!"}'` then `curl -s -b /tmp/c.txt http://localhost:3701/api/staff/tickets?status=open` to find the ticket id, then `curl -s -b /tmp/c.txt http://localhost:3701/api/staff/tickets/{id}`.
- Expect: 201 on create; the detail shows status `open`, the seeded "Support" department, and a first thread entry of type `M` containing the submitted message body.
- Status: [x]

## Checklist (children)

- [ ] TS-M1-B1 — Ticket service core (create ticket + append thread entry)
- [ ] TS-M1-B2 — Public create-ticket route + validation
- [ ] TS-M1-B3 — Public "Open a New Ticket" form + confirmation UI

## Test Infrastructure

- A dev endpoint or the staff queue (US-M1-3) is used to confirm AC-4 persistence in-browser.
- No CAPTCHA/throttle in M1, so the form is directly submittable in automated browser runs.

## Dependencies

- EPIC-M1-A (foundation, validation, DB schema).
