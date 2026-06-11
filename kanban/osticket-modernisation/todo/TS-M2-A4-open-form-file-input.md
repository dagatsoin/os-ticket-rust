# TS-M2-A4 — add(ui): FS-011.7 /open file input + inline validation + confirmation chip

- **ID**: TS-M2-A4
- **Type**: Technical Story
- **Parent**: US-M2-1
- **Labels**: Technical Story, M2
- **Scope**: small

## Context

The frontend half of US-M2-1: a file input on the M1 open-ticket form, client- and server-side
validation surfaced inline, and an attachment chip on the confirmation page.

## Impact

- update(ui): add an `<input type=file>` to the `/open` form (TS-M1-B3), shown only when `allow_attachments` is on, with helper text naming allowed types + the 1 MB cap.
- update(ui): submit the form as `multipart/form-data` via the FormData-aware apiClient (TS-M2-A0); map a 422 `attachment` field error to an **inline error** under the input.
- add(ui): a reusable **AttachmentChip** component (filename + icon); render it on the confirmation page for the uploaded file — the chip label comes from the **locally-selected `File.name`** (no API echo needed) (ROADMAP M2 Decisions §11). The AttachmentChip is the **shared, consumer-agnostic presentational shell** (`{label, icon, onClick?, readOnlyMarker?}` — render-only, no fetch) OWNED here and reused by B2 + D3 (ROADMAP M2 Decisions §14).
- add(ui): a **client-side** allow-list / 1 MB pre-check that hard-codes the seeded allow-list + cap as a UX convenience (ROADMAP M2 Decisions §11). **KL:** the backend (A3 + A2) remains authoritative; this front-end check is a pre-flight only and may drift from config until M4 exposes it. Extract this pre-check as the **shared `validateAttachment(file)` helper** OWNED here and reused by D3 for its own-file input (ROADMAP M2 Decisions §14).

## Regressions

- The form must still submit with no file selected (M1 path). Verify TS-M1-B3 ACs.

## Acceptance Tests

### AC-1: the /open form renders a file input with helper text when attachments are enabled. [BROWSER]
- Setup: `cargo run -p tools --bin seed -- --reset`; fixtures present (see Test Infrastructure).
- Navigate: http://localhost:3702/open
- Verify: a file input is present alongside Name/Email/Subject/Message, with helper text naming the allowed types and the 1 MB cap.
- Status: [ ]

### AC-2: a valid submission with a file shows the confirmation page with an AttachmentChip. [BROWSER]
- Navigate: http://localhost:3702/open
- Action: fill valid Name/Email/Subject/Message; choose `/tmp/qa-fixtures/invoice.pdf`; submit.
- Verify: the confirmation page shows the 6-digit ticket number AND an AttachmentChip labelled `invoice.pdf`.
- Status: [ ]

### AC-3: a disallowed type shows an inline error on the file field and blocks submission. [BROWSER]
- Navigate: http://localhost:3702/open
- Action: fill valid fields; choose `/tmp/qa-fixtures/evil.exe`; attempt submit.
- Verify: an inline "invalid file type" error renders under the file input; no confirmation page.
- Status: [ ]

### AC-4: an oversized file shows an inline "too big" error. [BROWSER]
- Navigate: http://localhost:3702/open
- Action: fill valid fields; choose `/tmp/qa-fixtures/big.pdf` (permitted type, > 1 MB); attempt submit.
- Verify: an inline "too big" error renders under the file input; no confirmation page.
- Status: [ ]

## Test Infrastructure

- Fixtures (create once; full block in US-M2-1 Test Infrastructure): `/tmp/qa-fixtures/invoice.pdf` (< 1 MB), `/tmp/qa-fixtures/evil.exe` (bad type), `/tmp/qa-fixtures/big.pdf` (> 1 MB permitted type). Reused by US-M2-1.

## Dependencies

- **TS-M2-A0 (apiClient FormData/multipart support — blocker)**, TS-M2-A3 (multipart create route + 422 field errors), TS-M1-B3 (open form), TS-M1-A5 (apiClient).
