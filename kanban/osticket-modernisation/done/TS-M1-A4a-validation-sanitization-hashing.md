# TS-M1-A4a — add(core): BS-003/BS-002 validation, sanitization & password hashing utils

- **ID**: TS-M1-A4a
- **Type**: Technical Story
- **Parent**: EPIC-M1-A
- **Labels**: Technical Story, M1
- **Scope**: small

## Context

The first half of the cross-cutting backend layer, split out from the original TS-M1-A4
(scope too big). This ticket is the pure-function tier: input validation, HTML sanitization,
and password hashing. No I/O, no DB, no middleware — all functions are deterministic and
TDD-driven. Reimplemented from the FS-003/FS-002 infrastructure specs (only the parts M1
exercises). Unblocks B1/B2 (which need validation/sanitize) and A3 (which needs hashing to
generate the seeded staff hash).

## Impact

- add(core): BS-003 input validation helpers — email validity (FS-003.7/.8/.9), required-field
  and length checks (FS-011.8).
- add(core): BS-003 HTML sanitization of free-text input via the **`ammonia` crate** (FS-003.10) —
  strip/escape disallowed HTML before persist.
- add(core): BS-002 password hashing + verify — **argon2id only**. No phpass/MD5/bcrypt legacy
  fallback; M1 is greenfield with no legacy accounts to migrate (see ROADMAP.md → Decisions →
  Hashing).

## Regressions

- None (greenfield). Shared pure-function code path: every validating route (B2) and the
  auth/seed flows (A3, C1) call these helpers.

## Acceptance Tests

> Pure functions — verified by `cargo test` over the `core` crate unit tests. No running server,
> no DB, no browser. All [API-ONLY] (command-line test execution).

### AC-1: BS-003 — it should reject an invalid email and accept a valid one (FS-003.7/.8/.9). [API-ONLY]
- Request: `cargo test -p core validate_email` (or run the full suite `cargo test -p core`).
- Expect: tests assert valid addresses pass and malformed ones (no @, no domain, spaces) are rejected.
- Status: [x]

### AC-2: BS-003 — it should reject a missing required field and enforce length bounds (FS-011.8). [API-ONLY]
- Request: `cargo test -p core required_fields` (covered by the core suite).
- Expect: empty/missing required fields rejected; over-length input rejected at the configured bounds.
- Status: [x]

### AC-3: BS-003 — it should strip/escape disallowed HTML from free-text via ammonia (FS-003.10). [API-ONLY]
- Request: `cargo test -p core sanitize` (covered by the core suite).
- Expect: a payload like `<script>alert(1)</script>hello` has the disallowed HTML stripped/escaped while safe text is preserved.
- Status: [x]

### AC-4: BS-002 — it should hash a password with argon2id and verify the correct plaintext, rejecting a wrong one. [API-ONLY]
- Request: `cargo test -p core hashing` (covered by the core suite).
- Expect: `hash("Agent123!")` verifies against "Agent123!" → true and against "wrong" → false.
- Status: [x]

### AC-5: BS-002 — it should produce argon2id hashes only (no legacy algorithm path exists). [API-ONLY]
- Request: `cargo test -p core hashing` plus inspect the emitted hash string prefix (`$argon2id$`).
- Expect: hashes are argon2id; no phpass/MD5/bcrypt code path is reachable.
- Status: [x]

## Test Infrastructure

- Pure functions — unit-testable with no fixtures. No dev endpoint required.

## Dependencies

- TS-M1-A1 (workspace).
- Unblocks: TS-M1-A3 (hashing for the seeded password), TS-M1-B1/TS-M1-B2 (validation + sanitize).
