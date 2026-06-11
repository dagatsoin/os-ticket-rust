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

> Setup (all): `cargo run -p tools --bin seed -- --reset` (seeds the two canned samples). DB queries via
> `docker exec backend-db-1 psql -U postgres -d osticket_dev`. Schema AC-1 is a `db` integration test
> (`TEST_DATABASE_URL`, skip-pass when unset): `cargo test -p db canned_schema`.

### AC-1: schema — a canned_response with title uniqueness + dept scope + enabled flag persists. [API-ONLY]
- Run: integration test inserts two responses, then a third reusing a title (`cargo test -p db canned_schema`).
- Verify: the duplicate-title insert errors (unique constraint); `dept_id=0` and `isenabled` persist as written.
- Status: [ ]

### AC-2: seed — "Acknowledge receipt" exists enabled, dept 0, body holds %{ticket.number}, bound to policy.txt. [API-ONLY]
- Run: `psql -c "select title,dept_id,isenabled,body from canned_response where title='Acknowledge receipt';"` and `psql -c "select af.name from canned_attachment ca join attachment_file af on af.id=ca.file_id join canned_response cr on cr.id=ca.canned_id where cr.title='Acknowledge receipt';"`.
- Verify: the response is `isenabled=true`, `dept_id=0`, its body contains `%{ticket.number}`; the joined `attachment_file.name` is `policy.txt`; `find "${BLOB_ROOT:-var/blobs}" -type f | wc -l` shows the one seeded blob.
- Status: [ ]

### AC-3: seed — a disabled sample response exists (for the enabled-only filter test). [API-ONLY]
- Run: `psql -c "select title,isenabled from canned_response where isenabled=false;"`.
- Verify: "Closed — disabled sample" is present with `isenabled=false`.
- Status: [ ]

## Dependencies

- TS-M2-A1 (blob store + attachment_file), TS-M1-A3 (seed harness).
