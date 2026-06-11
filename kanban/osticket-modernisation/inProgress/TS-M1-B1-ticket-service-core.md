# TS-M1-B1 — add(core): BS-011/BS-021 ticket service core (create + append thread entry)

- **ID**: TS-M1-B1
- **Type**: Technical Story
- **Parent**: US-M1-2
- **Labels**: Technical Story, M1
- **Scope**: medium

## Context

The architectural heart of the system (per the spec-writer note: web form, staff UI, email,
and API are all thin adapters over one core). Built once here, reused by US-M1-3's staff reply.
Only the create + append-thread-entry operations are in M1 scope.

## Impact

- add(core): `create_ticket(input) -> Ticket` — allocates the external ticket number, sets dept +
  `open` status, writes the first thread message. **Create = the ticket row + its first thread
  entry in ONE transaction** (never a partial ticket).
- add(core): `append_thread_entry(ticket, entry)` — adds a thread entry to an existing ticket.
  **Thread entry types per FS-091.4**: `M` = first client message, `R` = staff response,
  `N` = internal note (`N` is **unused in M1** but the type is modelled).
- add(core): ticket-number generation per **FS-091.2** — a **random 6-digit number in
  100000–999999**, made **collision-safe via the UNIQUE constraint + retry-on-conflict** (insert,
  and on unique-violation regenerate and retry — **no SELECT-then-INSERT** race).
- (Channel adapters live in TS-M1-B2 web route and TS-M1-C* staff route — this TS is transport-agnostic.)

## Regressions

- Shared code path. Flows affected by future changes: US-M1-2 (open ticket), US-M1-3 (staff reply),
  US-M1-4 (client view). List these in the Epic QA checklist for regression when this core changes.

## Acceptance Tests

> Service-core logic verified by `cargo test` integration tests against `osticket_dev` (schema +
> seed applied). No browser. All [API-ONLY].

### AC-1: BS-011 — create_ticket persists a ticket with a unique 6-digit number (100000–999999), the seeded dept, status open, and a first M thread entry, all in one transaction. [API-ONLY]
- Setup: migrations + seed applied to `osticket_dev`.
- Request: `cargo test -p core create_ticket` — calls create_ticket with a sample input.
- Expect: a persisted ticket with ticketID in 100000–999999, the seeded department, status `open`, and exactly one thread entry of type `M` holding the body; assert atomicity (a forced failure leaves NO partial ticket row).
- Status: [ ]

### AC-2: BS-021 — append_thread_entry adds an R response entry visible in the thread, preserving order. [API-ONLY]
- Request: `cargo test -p core append_thread_entry` — create a ticket, then append an `R` entry.
- Expect: the thread returns `M` then `R` in chronological order.
- Status: [ ]

### AC-3: FS-091.2 — distinct ticket numbers across consecutive creates; recover from a number collision via retry-on-conflict (no SELECT-then-INSERT). [API-ONLY]
- Request: `cargo test -p core ticket_number` — create several tickets and assert distinct numbers; a test forces a unique-violation (e.g. by pre-seeding a colliding number) and asserts the core regenerates + retries the INSERT (no SELECT-then-INSERT race).
- Expect: all numbers distinct; collision path recovers without error.
- Status: [ ]

## Dependencies

- EPIC-M1-A (schema, validation).
