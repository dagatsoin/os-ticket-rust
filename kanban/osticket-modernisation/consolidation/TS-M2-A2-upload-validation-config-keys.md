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
- update(migration/seed): persist the three keys into the config store seeded by TS-M1-A3.

## Regressions

- None — new validation helper + new config keys. Config seed must remain idempotent.

## Acceptance Tests

### AC-1: BS-022.13 — a permitted extension passes; a disallowed extension is rejected. [API-ONLY]
- Test: `invoice.pdf` passes against `.pdf,.png,.jpg,.txt,.doc`; `evil.exe` returns "Invalid file type".
- Status: [ ]

### AC-2: BS-022.14 — an empty allow-list rejects everything; `.*` allows all. [API-ONLY]
- Test: with `allowed_filetypes` empty, every file is rejected (default-deny); with `.*`, any extension passes.
- Status: [ ]

### AC-3: FS-022.13 — a file larger than max_file_size is rejected with a too-big error. [API-ONLY]
- Test: a 1 MB + 1 byte file against `max_file_size=1048576` returns the "too big" error; a sub-max file passes.
- Status: [ ]

### AC-4: when allow_attachments is disabled, validation reports attachments not permitted. [API-ONLY]
- Test: with `allow_attachments=false`, any upload is refused (master switch).
- Status: [ ]

## Dependencies

- TS-M1-A3 (config seed harness), TS-M1-A4a (validation/field-error conventions).
