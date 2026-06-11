# EPIC-M2-C — EPIC – Variable substitution engine

- **ID**: EPIC-M2-C
- **Type**: Epic
- **Parent**: M2
- **Labels**: Epic, M2
- **Column**: derived from children (`min(children.columns)`)

## Spec References

- FS-040 (FS-040.11 variable-substitution grammar; BS-040.14–.19 dot-paths, accessors, bare-object,
  unknown-preserved, array/string, `%{url}` always present)

## Context

Milestone: M2. Standalone — depends on nothing in M2, but both Epic D (canned bodies) and Epic E
(email templates) consume it. Built early, in parallel, behind a clean unit-tested interface.

## Description

Implements the **`%{token}` substitution engine** in the shared `ost_core` crate, **TDD**
(FS-040.11, BS-040.14–.19): dot-path traversal, **unknown tokens preserved verbatim** (not blanked),
and **`%{url}` always present**. Ships a token catalog sufficient for M2:
`%{ticket.number|name|subject|email|status|create_date}`, `%{ticket.dept.name}`, and `%{url}`.
No UI — this is a pure library with a comprehensive unit-test suite.

Business value: a single, well-tested substitution primitive that both canned responses and email
templates reuse, so personalised text ("Hello, your ticket #123…") works identically everywhere and
authoring mistakes surface as a visible literal `%{...}` rather than silently-dropped content. It is
the shared engine that makes M2 root AC-3 (substituted canned body) and AC-4 (substituted email)
possible.

## Acceptance Criteria

- (Epic — no ACs. Validated through TS-M2-C1's unit tests and, end to end, via the M2 root E2E
  AC-3 / AC-4 where the substituted output is visible.)

## Checklist (children)

- [ ] TS-M2-C1 — `%{token}` substitution engine in ost_core (TDD) + M2 token catalog

## Dependencies

- M1 (ost_core crate, ticket model). No other M2 epic.
