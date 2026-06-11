# TS-M2-D1 — add(domain): FS-022.14 canned_response + canned_attachment schema + seed

- **ID**: TS-M2-D1
- **Type**: Technical Story
- **Parent**: EPIC-M2-D
- **Labels**: Technical Story, M2
- **Scope**: small

## Context

The canned-response data model + the two seeded samples that stand in for the M4 CRUD UI. One sample
carries a `%{...}` variable (exercises Epic C); one carries a seeded `.txt` attachment (exercises
Epic A binding + Epic B download).

## Impact

- add(migration): `canned_response` (`id`, `title` unique, `body`, `notes`, `dept_id` 0=all, `isenabled`, `created`, `updated`).
- add(migration): `canned_attachment` (`canned_id`, `file_id` → attachment_file).
- update(seed): seed **"Acknowledge receipt"** — enabled, dept 0 (all), body contains `%{ticket.number}` (and `%{url}`), carrying a seeded `policy.txt` (stored via BlobStore → `attachment_file` + `canned_attachment`).
- update(seed): seed **"Closed — disabled sample"** — **disabled** (negative case for BS-022.2), no attachment.

## Regressions

- New tables + seed rows; idempotent seed (TS-M1-A3 contract preserved). The seeded `policy.txt` blob reuses the A1 store.

## Acceptance Tests

### AC-1: schema — a canned_response with title uniqueness + dept scope + enabled flag persists. [API-ONLY]
- Insert two responses; duplicate title rejected; dept_id 0 + isenabled stored.
- Status: [ ]

### AC-2: seed — "Acknowledge receipt" exists enabled, dept 0, body holds %{ticket.number}, bound to policy.txt. [API-ONLY]
- Query after seed → the response + a `canned_attachment` row → `attachment_file` for `policy.txt` (one blob on disk).
- Status: [ ]

### AC-3: seed — a disabled sample response exists (for the enabled-only filter test). [API-ONLY]
- Query after seed → a response with isenabled = false.
- Status: [ ]

## Dependencies

- TS-M2-A1 (blob store + attachment_file), TS-M1-A3 (seed harness).
