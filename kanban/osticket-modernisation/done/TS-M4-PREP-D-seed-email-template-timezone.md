# TS-M4-PREP-D — add(seed): DEVIATION M4-D1 minimal email_account + template_group + timezone

- **ID**: TS-M4-PREP-D
- **Type**: Technical Story
- **Parent**: EPIC-M4-PREP
- **Labels**: Technical Story, M4
- **Scope**: small

## Context

**DEVIATION M4-D1 (RISK-1).** The department Email/Template required selects (FS-030.4, BS-030-02/03)
and the FS-032 Emails tab must resolve against **real rows**, not system-default stubs. This TS seeds a
minimal `email_account` + `template_group` model and the `timezone` reference set. M5/FS-040 extends
the email/template model with fetch/SMTP/template-body detail.

## Impact

- add(seed): one `email_account` row — `support@osticket.local`, name "Support", active
  (the existing M2 SMTP env transport is unchanged; this row exists so the selects have an option).
- add(seed): one `template_group` row — "osTicket Default", isactive — value that BS-030-03 accepts
  (numeric id; `0` remains the "system default template" sentinel per BS-030-03).
- add(seed): the `timezone` reference rows (a practical subset of offsets + DST flags) so the
  profile Time Zone select (FS-031.10/.11) and `default_timezone_id` resolve.
- Idempotent + honored by `--reset`.

## Regressions

- The M2 env-driven SMTP transport (DEVIATION D3) is authoritative for actually sending mail — this
  row is **model/selection only**, it does not change how mail is transported.

## Acceptance Tests

### AC-1: M4-D1 — at least one active email_account exists. [API-ONLY]
- Setup: `cargo run -p tools --bin seed -- --reset`
- Request: `psql -U postgres -d osticket_dev -c "SELECT email, name, active FROM email_account WHERE active"`
- Expect: >= 1 row; `support@osticket.local`, name "Support", `active=t`.
- Status: [x]

### AC-2: M4-D1 — at least one active template_group exists. [API-ONLY]
- Request: `psql -U postgres -d osticket_dev -c "SELECT name, isactive FROM template_group WHERE isactive"`
- Expect: >= 1 row, name "osTicket Default", `isactive=t`.
- Status: [x]

### AC-3: FS-031.11 — the timezone reference set is seeded. [API-ONLY]
- Request: `psql -U postgres -d osticket_dev -c "SELECT count(*) FROM timezone"`
- Expect: count > 0 (a practical offset subset).
- Verify (shape): `psql -U postgres -d osticket_dev -c "SELECT column_name FROM information_schema.columns WHERE table_name='timezone'"` shows the offset + label + dst columns (note: `offset` is a PG reserved word — if used as a column name it must be quoted `"offset"` in queries, or named e.g. `gmt_offset`), and a UTC/GMT+0 row is present.
- Status: [x]

### AC-4: default_email_id / default_template_id point at seeded rows. [API-ONLY]
- Request: `psql -U postgres -d osticket_dev -c "SELECT c.key, c.value FROM config c WHERE c.key IN ('default_email_id','default_template_id') ORDER BY c.key"`
- Expect: both keys present; `default_email_id` equals the seeded `email_account.id` and `default_template_id` equals the seeded `template_group.id` (or the `0` system-default sentinel per BS-030-03, if that is the chosen default).
- Verify (referential): `psql -U postgres -d osticket_dev -c "SELECT (SELECT value::int FROM config WHERE key='default_email_id') IN (SELECT id FROM email_account)"` returns `t`.
- Status: [x]

### AC-5: Idempotent — re-seed does not duplicate reference rows. [API-ONLY]
- Setup: run `cargo run -p tools --bin seed -- --reset` twice
- Request: `psql -U postgres -d osticket_dev -c "SELECT (SELECT count(*) FROM email_account WHERE email='support@osticket.local'), (SELECT count(*) FROM template_group WHERE name='osTicket Default')"`
- Expect: each count is exactly 1 (upsert, no duplication) — honors the `--reset` contract.
- Status: [x]

## Test Infrastructure

- Uses `cargo run -p tools --bin seed -- --reset`. No dev endpoint required.
- **Provides for downstream**: the seeded `email_account`/`template_group` rows are the options the
  M4-C department Email/Template `[BROWSER]` selects render (FS-030.4, BS-030-02/03), and the
  `timezone` rows back the M4-B/profile Time Zone select (FS-031.10/.11). Coordinate the id-valued
  `default_email_id`/`default_template_id` values with TS-M4-PREP-C (which owns those config keys).

## Dependencies

- **TS-M4-PREP-B**: `email_account`/`template_group`/`timezone` tables.
- Pairs with **TS-M4-PREP-C** (which points the default_* keys at these rows).
