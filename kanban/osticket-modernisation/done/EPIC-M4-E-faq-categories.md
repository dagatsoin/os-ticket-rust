# EPIC-M4-E — EPIC – FAQ Categories

- **ID**: EPIC-M4-E
- **Type**: Epic
- **Parent**: M4
- **Labels**: Epic, M4
- **Column**: derived from children

## Spec References

- FS-032.13 (FAQ category listing & mass actions)
- FS-032.14 (FAQ category create / edit form)
- FS-032.15 (FAQ category access control — gated by `can_manage_faq`, NOT the admin gate)
- FS-032.16 (FAQ category deletion cascade)
- BS-031-020 (`can_manage_faq` flag)

## Context

Milestone: M4 — Admin Configuration. The **first delegated, non-admin** capability: unlike every
other M4 screen, FAQ categories are gated by the `can_manage_faq` group flag rather than the admin
gate. Built **after EPIC-M4-B** so the flag/group exists (Integration AC-6). The FAQ articles
themselves are M7; M4 delivers only the category taxonomy.

## Description

FAQ-category CRUD reachable by any staff member whose group has `can_manage_faq` (admins included):
list with mass actions; create/edit (name, type public/private, description); deletion cascades to
contained FAQs' associations (articles arrive in M7). KL-032.11 (`Category::lookup` not verifying
the loaded row) modernised — a missing category id yields a clean 404, not a hollow object.

## Business value

Knowledge-base structure can be curated by a delegated content manager who has no access to system
settings or other admin screens — proving the group-flag authorization model end to end.

## Acceptance Criteria

> No ACs — intermediate parent; verification owned by leaf tickets. Column derived from children.

## Children (column derived: min of these)

- [ ] [US-M4-E1](US-M4-E1-faq-manager-manages-categories.md) — FAQ manager manages FAQ categories
  - [ ] [TS-M4-E1](TS-M4-E1-faq-category-crud-endpoints.md) — Backend: faq_category CRUD + `can_manage_faq` gate + deletion cascade
  - [ ] [TS-M4-E2](TS-M4-E2-faq-category-ui.md) — Frontend: FAQ category list + form UI (reachable without settings access)

## Dependencies

- **EPIC-M4-PREP**: `faq_category` table.
- **EPIC-M4-B**: `can_manage_faq` flag + a non-admin group carrying it.
- **EPIC-M4-A (shell)**: reuses the panel shell but with the FAQ (not admin) gate.
