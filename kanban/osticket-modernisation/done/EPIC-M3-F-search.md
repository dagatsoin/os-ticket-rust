# EPIC-M3-F — EPIC – Search

- **ID**: EPIC-M3-F
- **Type**: Epic
- **Parent**: M3
- **Labels**: Epic, M3
- **Column**: derived from children

## Spec References

- FS-020.7 (basic keyword search)
- FS-020.8 (advanced search dialog)

## Context

Milestone: M3 — Full Staff Workflow & Queue.
M1 had no search. This epic adds basic keyword search (by ticket number, email, or free-text)
and an advanced search dialog with filters for status, department, assignee, date range, etc.

## Description

Implement basic search: a keyword >= 3 chars triggers search by ticket number (numeric prefix),
requester email (exact match), or deep content search (subject/name/thread body LIKE). Implement
advanced search: a dialog with optional filters for status, department (bounded by access),
assignee, help topic, and date range. Search results show "(Search Results)" label and may
span all statuses (Status column replaces Priority when no status filter).

## Business value

Agents can quickly find specific tickets by number, requester email, or content keywords. The
advanced search lets them narrow down to specific departments, assignees, or date ranges for
reporting or investigation.

## Acceptance Criteria

> No ACs — intermediate parent; verification owned by leaf TS tickets. Column derived from children.

## Children (column derived: min of these)

- [ ] [US-M3-F1](US-M3-F1-agent-searches-by-keyword.md) — Agent searches by keyword and sees results
  - [ ] [TS-M3-F1](TS-M3-F1-basic-search-endpoint.md) — Backend: basic search (number/email/deep-text)
  - [ ] [TS-M3-F2](TS-M3-F2-advanced-search-endpoint.md) — Backend: advanced search (multi-criteria filtering)
  - [ ] [TS-M3-F3](TS-M3-F3-frontend-search-dialog.md) — Frontend: search box + advanced search dialog UI

## Dependencies

- **EPIC-M3-A**: search operates on the same listing with visibility scoping
- **EPIC-M3-B**: search results use the same sort/pagination infrastructure
