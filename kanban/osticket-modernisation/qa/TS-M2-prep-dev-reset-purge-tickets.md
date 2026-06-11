# TS-M2-prep — update(tooling): add a `--reset` purge flag to the seed binary

- **ID**: TS-M2-prep-dev-reset
- **Type**: Technical Story (test infrastructure)
- **Parent**: M2 (attached directly to the milestone as test infra; no User Story)
- **Labels**: Technical Story, M2
- **Scope**: small

## Context

QA observation from the M1 E2E sweep (root 5/5, playbook 11/11 green, 2026-06-11): the seed binary
(`cargo run -p tools --bin seed`, TS-M1-A3) restores dept/group/`agent` but **never purges tickets**,
so the Open queue accumulates across repeated sweeps. M2 adds far more state (attachments, blobs,
canned bindings) and runs longer E2E journeys, so deterministic resets are now required before each
sweep — the M2 root E2E and every US AC pre-flight calls `cargo run -p tools --bin seed -- --reset`.

**Mechanism decided (consolidation):** a **`--reset` flag on the seed binary** (NOT a new HTTP
endpoint). It truncates ticket-scoped data before re-seeding the dept/group/staff baseline. Chosen
over a `/api/dev/reset` route because the E2E pre-flights already shell out to the seed binary, so a
flag keeps one reset mechanism and needs no env-gated route surface.

## Impact

- update(tooling): `cargo run -p tools --bin seed -- --reset` truncates (in FK-safe order) `ticket_attachment`, `ticket_thread`, `ticket`, and `canned_attachment`-derived ticket bindings, then re-seeds the dept/group/`agent` baseline (and the M2 seed config keys + seeded canned responses).
- update(tooling): prune orphaned `attachment_file` rows + their blobs under **`BLOB_ROOT`** (ROADMAP M2 Decisions §1) that are no longer referenced after the purge (keeps the blob dir clean across sweeps); the seeded `policy.txt` canned blob is preserved (still referenced by the re-seeded canned response). **Staging note:** the table-truncation half lands as soon as the schema exists (after A1); the **blob-reclamation half is staged after A1 + D1** (it needs the blob store and the seeded canned blob to know what to preserve).
- The plain `cargo run -p tools --bin seed` (no flag) keeps its M1 idempotent, non-destructive behaviour.

## Regressions

- Must NOT delete the seeded department / group / `agent` account or the seeded canned responses — only ticket-scoped data + truly-orphaned blobs.
- The M1 idempotency contract (plain `seed` writes only `osticket_dev`, destroys nothing) is preserved.
- Production safety: the binary is a dev tool; it must refuse to run against a non-dev database (guard on the target DB name / an explicit dev env).

## Acceptance Tests

> Setup (all): backend on :3701; fixtures in `/tmp/qa-fixtures`; staff cookie jar from
> `POST /api/staff/login` (`agent`/`Agent123!`). Run the binary as `cargo run -p tools --bin seed -- --reset`.
> The tool must refuse a non-dev target DB (guard on the DB name / dev env).

### AC-1: `--reset` purges all tickets + thread entries + ticket attachments so the Open queue is empty. [API-ONLY]
- Setup: create a few tickets, some with a `.pdf` — `curl -X POST .../api/tickets -F name=A -F email=a@x.com -F subject=s -F message=m -F attachment=@/tmp/qa-fixtures/invoice.pdf` (repeat for a couple).
- Request: run `cargo run -p tools --bin seed -- --reset`, then `curl -s -b /tmp/qa-staff.jar 'http://localhost:3701/api/staff/tickets?status=open'`.
- Verify: the Open queue is empty; `docker exec backend-db-1 psql -U postgres -d osticket_dev -c "select count(*) from ticket; select count(*) from ticket_attachment;"` → both 0.
- Status: [x]

### AC-2: `--reset` preserves the seeded department, group, `agent` account, and seeded canned responses. [API-ONLY]
- Request: after the reset, `curl -i -X POST .../api/staff/login` with `agent`/`Agent123!`; `psql -c "select count(*) from canned_response;"`.
- Verify: staff login succeeds (200); the two seeded canned responses still resolve (count = 2), and the seeded department/group rows remain.
- Status: [x]

### AC-3: `--reset` reclaims orphaned blobs but keeps referenced ones. [API-ONLY]
- Setup: note `find "${BLOB_ROOT:-var/blobs}" -type f | wc -l` after AC-1's ticket-with-attachment creates (ticket blob + seeded `policy.txt` present).
- Request: run `cargo run -p tools --bin seed -- --reset`, then re-run the `find ... | wc -l`.
- Verify: the ticket-only blob is gone; the seeded `policy.txt` canned blob remains (still referenced by the re-seeded canned response).
- Status: [x]

### AC-4: plain `seed` (no flag) remains non-destructive (M1 contract). [API-ONLY]
- Setup: with one or more tickets present, note the ticket count.
- Request: run `cargo run -p tools --bin seed` (no flag).
- Verify: `psql -c "select count(*) from ticket;"` is unchanged (tickets untouched); the dept/group/`agent` baseline is reseeded idempotently (no duplicate rows).
- Status: [x]

## Test Infrastructure

- Enables clean, repeatable M2 E2E sweeps; called once per journey pre-flight.

## Dependencies

- TS-M1-A3 (seed binary), TS-M2-A1 (blob store, for orphan-blob reclamation), TS-M2-D1 (seeded canned responses to preserve).
