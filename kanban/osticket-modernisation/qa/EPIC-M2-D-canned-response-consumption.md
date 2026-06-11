# EPIC-M2-D — EPIC – Canned response consumption

- **ID**: EPIC-M2-D
- **Type**: Epic
- **Parent**: M2
- **Labels**: Epic, M2
- **Column**: derived from children (`min(children.columns)`)

## Spec References

- FS-022 (FS-022.14 canned consumption; BS-022.1 dept scope, BS-022.2 enabled-only, BS-022.15
  marks unanswered)
- FS-040 (FS-040.11 variable substitution — via Epic C)
- FS-021 (FS-021.3 staff reply path the dropdown lives on)

## Context

Milestone: M2. Needs Epic A (so a canned response's attachment rides on the reply as a real bound
file) and Epic C (so the canned body is variable-substituted before insertion).

## Description

Adds the `canned_response` + `canned_attachment` tables and **SEEDS two sample responses** — one
carrying a `%{...}` variable, one carrying a seeded `.txt` attachment. The staff reply box gains a
**"canned response" dropdown** (only **enabled** + **dept-scoped** responses offered, BS-022.1 /
BS-022.2) that, on selection, **inserts the substituted body** (FS-022.14, via Epic C) and **carries
the response's attachments** onto the reply. A posted canned reply **marks the ticket unanswered**
(BS-022.15). **No CRUD UI** — managing canned responses is deferred to M4.

**Documented divergence from legacy**: BS-022.15 records the canned-reply poster as the literal
"SYSTEM (Canned Reply)". M2 keeps the **posting agent** as the author (the M1 reply model has no
SYSTEM actor yet); the unanswered-flag and substitution/attachment behaviours are preserved. This is
an intentional, pinned divergence.

Business value: agents reply faster and more consistently with a shared template library, and the
customer receives a fully personalised, attachment-bearing answer in one click. Makes M2 root AC-3
(reply via canned response) green.

## Acceptance Criteria

- (Epic — no ACs. Validated through its children's ACs and the M2 root E2E AC-3.)

## Checklist (children)

- [ ] TS-M2-D1 — canned_response / canned_attachment schema + seed two samples (one w/ variable, one w/ .txt)
- [ ] US-M2-2 — Staff replies with attachment + canned response (children: TS-M2-D2 canned-fetch route, TS-M2-D3 reply-box dropdown UI, TS-M2-D4 canned-reply post wiring)

## Dependencies

- EPIC-M2-A (attachment binding), EPIC-M2-C (substitution engine). M1 staff reply path.
