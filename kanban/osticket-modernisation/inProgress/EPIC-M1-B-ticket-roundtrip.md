# EPIC-M1-B — EPIC – Ticket Round-Trip

- **ID**: EPIC-M1-B
- **Type**: Epic
- **Parent**: M1
- **Labels**: Epic, M1
- **Column**: derived from children

## Spec References

- FS-011 (public ticket submission web form)
- FS-010 (client portal lookup / view / reply)
- FS-021 (staff single-ticket workflow — reply + status)
- FS-020 (staff queue — minimal open-tickets list)
- FS-002 (staff login / session / one permission gate)

## Context

Milestone: M1 — First Ticket Round-Trip.
This epic delivers the actual user-facing loop on top of EPIC-M1-A's foundation: a customer
opens a ticket, an agent logs in, finds it, and replies, and the customer reads the reply.
It is anchored on the **shared ticket create-and-append core** (the architecture note: web
form and staff UI are thin adapters over one service).

## Description

Implement the three user-facing stories of the slice. The ticket service core (create ticket
+ append thread entry) is built once inside US-M1-2 and reused by the staff reply path in
US-M1-3.

## Business value

Delivers the first demonstrable end-to-end helpdesk interaction — the product's reason to
exist — and exercises the shared core that every future channel (email, API) will adapt to.

## Acceptance Criteria

> No ACs — intermediate parent; verification owned by the M1 root milestone E2E journey (the full public-create → staff-reply → client-view round-trip). Column derived from children.

## Children (column derived: min of these)

- [ ] US-M1-2 — Client opens a ticket via the web form
- [ ] US-M1-3 — Staff logs in, sees the queue, and replies
- [ ] US-M1-4 — Client views the staff reply

## Dependencies

- All children depend on EPIC-M1-A (foundation). US-M1-3 and US-M1-4 depend on US-M1-2
  (a ticket must exist), though their UI can be built in parallel against seeded/mock tickets.
