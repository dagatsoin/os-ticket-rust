# TS-M2-A2 — add(domain): FS-022.13 upload validation (type + size) + attachment config keys

- **ID**: TS-M2-A2
- **Type**: Technical Story
- **Parent**: EPIC-M2-A
- **Labels**: Technical Story, M2
- **Scope**: small

## Context

The shared validation gate every upload path (A3 create, A5 reply, D4 canned own-file) runs before
a blob is stored. Extension-based allow-list + max-size, matching the legacy behaviour and its
default-deny quirk.

## Impact

- add(domain): `validate_upload(name, size, mime)` in `ost_core` — extension allow-list check (BS-022.13) + max-size check (FS-022.13); returns a field error on failure.
- add(config): seed config keys — `allow_attachments` (enabled), `allowed_filetypes` (`.pdf,.png,.jpg,.txt,.doc`), `max_file_size` (`1048576` = 1 MB).
- add(config): seed `helpdesk_url` = `http://localhost:3702` (ROADMAP M2 Decisions §6) — the base URL the substitution engine (C1) and email wiring (E3) read for `%{url}`.
- update(migration/seed): persist the four keys into the config store seeded by TS-M1-A3.

## Regressions

- None — new validation helper + new config keys. Config seed must remain idempotent.

## Acceptance Tests

> Pure `validate_upload` unit tests (no DB). Run: `cargo test -p ost_core validate_upload`.
> Config-seed cases are DB-backed (`TEST_DATABASE_URL`, skip-pass when unset): `cargo test -p tools config_seed`.

### AC-1: BS-022.13 — a permitted extension passes; a disallowed extension is rejected. [API-ONLY]
- Run: unit test `validate_upload("invoice.pdf", 100, ..)` against allow-list `.pdf,.png,.jpg,.txt,.doc`, and `validate_upload("evil.exe", 100, ..)`.
- Verify: `invoice.pdf` returns Ok; `evil.exe` returns a field error "Invalid file type".
- Status: [x]

### AC-2: BS-022.14 — an empty allow-list rejects everything; `.*` allows all. [API-ONLY]
- Run: unit test with `allowed_filetypes=""` then `allowed_filetypes=".*"`.
- Verify: with the empty list every file is rejected (default-deny); with `.*` any extension passes.
- Status: [x]

### AC-3: FS-022.13 — a file larger than max_file_size is rejected with a too-big error. [API-ONLY]
- Run: unit test `validate_upload(name, 1048577, ..)` and `validate_upload(name, 1048576, ..)` against `max_file_size=1048576`.
- Verify: the 1 MB + 1 byte file returns the "too big" error; the at-cap file passes.
- Status: [x]

### AC-4: when allow_attachments is disabled, validation reports attachments not permitted. [API-ONLY]
- Run: unit test with `allow_attachments=false`.
- Verify: any upload is refused (master switch) regardless of type/size.
- Status: [x]

### AC-5: the four config keys seed idempotently with the pinned defaults. [API-ONLY]
- Setup: `TEST_DATABASE_URL` set.
- Run: run the seed twice (`cargo test -p tools config_seed::idempotent`), then `docker exec backend-db-1 psql -U postgres -d osticket_test -c "select key,value from config where key in ('allow_attachments','allowed_filetypes','max_file_size','helpdesk_url') order by key;"`.
- Verify: `allow_attachments`=on, `allowed_filetypes`=`.pdf,.png,.jpg,.txt,.doc`, `max_file_size`=`1048576`, `helpdesk_url`=`http://localhost:3702`; a second seed run leaves exactly one row per key (idempotent).
- Status: [x]

## Dependencies

- TS-M1-A3 (config seed harness), TS-M1-A4a (validation/field-error conventions).
