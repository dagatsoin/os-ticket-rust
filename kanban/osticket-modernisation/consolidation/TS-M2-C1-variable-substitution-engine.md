# TS-M2-C1 — add(domain): FS-040.11 %{token} substitution engine (TDD) + M2 token catalog

- **ID**: TS-M2-C1
- **Type**: Technical Story
- **Parent**: EPIC-M2-C
- **Labels**: Technical Story, M2
- **Scope**: small

## Context

A pure, unit-tested `%{token}` substitution engine in `ost_core`, reused by canned bodies (D) and
email templates (E). **TDD** — the BS-040.14–.19 rules are the test list. No UI, no DB beyond reading
a ticket.

## Impact

- add(domain): `VariableReplacer` in `ost_core` — `render(text, context) -> String` resolving `%{name}` and `%{a.b.c}` dot-paths against a context of objects/scalars.
- add(domain): an M2 token catalog: `%{ticket.number}`, `%{ticket.name}`, `%{ticket.subject}`, `%{ticket.email}`, `%{ticket.status}`, `%{ticket.create_date}`, `%{ticket.dept.name}`, and `%{url}` (always present). The engine takes the base URL as an **input argument**; the wiring (D2/E3) feeds it the seeded `helpdesk_url` config key (= `http://localhost:3702`, ROADMAP M2 Decisions §6).
- Unknown tokens are left **verbatim** (`%{...}` preserved), not blanked.

## Regressions

- None — new library, no existing caller until D/E.

## Acceptance Tests

### AC-1: BS-040.14 — dot-path traversal resolves nested tokens. [API-ONLY]
- `render("#%{ticket.number} — %{ticket.dept.name}", ctx)` → number + dept name substituted.
- Status: [ ]

### AC-2: BS-040.17 — an unknown token is left literally in the output. [API-ONLY]
- `render("%{ticket.bogus}", ctx)` → output still contains the literal `%{ticket.bogus}`.
- Status: [ ]

### AC-3: BS-040.19 — %{url} is always present without being supplied in the ticket context. [API-ONLY]
- `render("%{url}", ctx)` → the configured base URL (FQDN).
- Status: [ ]

### AC-4: BS-040.14 — a single-character token name is NOT recognized (left untouched). [API-ONLY]
- `render("%{x}", ctx)` → literal `%{x}` preserved (the grammar requires ≥1 trailing char).
- Status: [ ]

### AC-5: BS-040.18 — substitution applies across an array (subject + body pair). [API-ONLY]
- `render(["%{ticket.subject}", "Ticket %{ticket.number}"], ctx)` → both substituted.
- Status: [ ]

### AC-6: the M2 catalog tokens all resolve against a real seeded ticket. [API-ONLY]
- For a seeded ticket, each of number/name/subject/email/status/create_date/dept.name resolves to the ticket's value.
- Status: [ ]

## Dependencies

- M1 (ost_core, ticket model, config base-URL key).
