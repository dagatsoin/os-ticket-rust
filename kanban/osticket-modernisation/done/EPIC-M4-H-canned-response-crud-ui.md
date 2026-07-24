# EPIC-M4-H — EPIC – Canned Response CRUD UI

- **ID**: EPIC-M4-H
- **Type**: Epic
- **Parent**: M4
- **Labels**: Epic, M4
- **Column**: derived from children

## Spec References

- FS-022 (canned responses — the admin CRUD surface deferred out of M2)
- BS-031-020 (`can_manage_premade` flag gates this surface)

## Context

Milestone: M4 — Admin Configuration. M2 built the `canned_response` / `canned_attachment` model and
the consumption path (dropdown in the reply composer) but **deferred the CRUD UI to M4**. This epic
delivers that admin surface, gated by the `can_manage_premade` group flag (a delegated, non-admin
capability like FAQ categories).

## Description

Canned-response CRUD reachable by any staff member whose group has `can_manage_premade`: list;
create/edit (title, department scope optional, body with `%{token}` variables, attachments,
active flag); disable/delete. Reuses the M2 model and the M2 variable-token catalog; the two M2
seeded responses become editable rows.

## Business value

Agents' pre-written replies are curated from the browser instead of seed SQL, closing the M2
deferral and completing the canned-response feature end to end.

## Acceptance Criteria

> No ACs — intermediate parent; verification owned by leaf tickets. Column derived from children.

## Children (column derived: min of these)

- [ ] [US-M4-H1](US-M4-H1-manager-manages-canned-responses.md) — Premade manager manages canned responses
  - [ ] [TS-M4-H1](TS-M4-H1-canned-crud-endpoints.md) — Backend: canned-response CRUD + `can_manage_premade` gate
  - [ ] [TS-M4-H2](TS-M4-H2-canned-list-form-ui.md) — Frontend: canned-response list + create/edit form UI

## Dependencies

- **M2 (done)**: `canned_response` / `canned_attachment` model + `%{token}` engine + token catalog.
- **EPIC-M4-B**: `can_manage_premade` flag + a group carrying it.
- **EPIC-M4-A (shell)**: reuses the panel shell with the premade (not admin) gate.
