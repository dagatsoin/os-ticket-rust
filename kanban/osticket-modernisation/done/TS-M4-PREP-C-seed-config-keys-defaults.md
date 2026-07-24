# TS-M4-PREP-C — add(seed): FS-032 ~110 core config keys + default_dept/sla/*_page_id bindings

- **ID**: TS-M4-PREP-C
- **Type**: Technical Story
- **Parent**: EPIC-M4-PREP
- **Labels**: Technical Story, M4
- **Scope**: small

## Context

The FS-032 settings tabs read/write ~110 config keys. M1/M2/M3 seeded only the handful the runtime
needed. This TS seeds the full set to their FS-032 defaults so every tab renders real values, plus
the default-object bindings the admin panel and routing rely on.

## Impact

- add(seed): ~110 `config` keys grouped by tab and set to FS-032 defaults, including at minimum:
  - System tab: `helpdesk_title`, `helpdesk_url`, `default_timezone_id`, `enable_daylight_saving`,
    `default_locale`, `max_page_size`, `staff_ip_binding`, `passwd_reset_period`,
    `staff_session_timeout`, `client_session_timeout`, `isonline`/`offline_reason`.
  - Ticket tab: `default_ticket_status`, `default_priority_id`, `default_sla_id`, `default_dept_id`,
    `default_help_topic`, `ticket_alert_*`, `max_open_tickets`, `enable_captcha`, `ticket_lock_time`,
    `showanswered`, `showassigned`, `random_ticket_ids`, `log_level`, `log_graceperiod`.
  - Email tab: `default_email_id`, `default_template_id`, `admin_email`, `alert_email_id`,
    `strip_quoted_reply`, `use_email_priority`.
  - Pages tab: `landing_page_id`, `offline_page_id`, `thank-you_page_id`.
  - Autoresponder + Alerts tabs: the `*_autoresp`, `*_alert`, `*_notice` toggles.
  - Attachments: `allow_attachments`, `allowed_filetypes`, `max_file_size` (already seeded in M2 —
    reconciled here, not duplicated).
- add(seed): `default_dept_id` → Support, `default_sla_id` → the default SLA, `*_page_id` → 0/unbound
  initially (bound later in EPIC-M4-A/F).
- add(seed): **the admin staff account** `admin` / `Admin123!` with `isadmin=true` — the login every
  M4 admin epic (M4-A settings, M4-B staff/groups, …) authenticates the admin panel with. The M1 seed
  only creates `agent`/`agent2` (both `isadmin=false`), so without this NO admin login exists. Reuses
  the M1 argon2id hashing util; joins the seeded group + Support dept; idempotent upsert on `username`.
- KL-032.3 modernised: `send_sys_errors` seeded and stored as its real value (not forced-0 display).
- The seed is **idempotent** and honored by `--reset`.

## Regressions

- M2/M3 keys (`allow_attachments`, `max_page_size`, `ticket_lock_time`, `show_answered_tickets`, …)
  must not be duplicated or clobbered — reconcile to a single source of truth.
- **config upsert semantics (`lib.rs` `seed_config`)**: the current upsert *preserves an existing
  value* ("an existing DB keeps whatever an admin has since set"). That is correct for admin-tunable
  runtime keys, but it means `seed --reset` will NOT restore a key an M4 settings-tab E2E has since
  changed. Either the ~110 defaults must be force-restored by `--reset`, or a dev reset-config path is
  added (see Test Infrastructure) — otherwise the M4-A settings E2Es cannot return to a known state.

## Acceptance Tests

### AC-1: FS-032 — the config table carries at least ~110 keys after seed. [API-ONLY]
- Setup: `cargo run -p tools --bin seed -- --reset`
- Request: `psql -U postgres -d osticket_dev -c "SELECT count(*) FROM config"`
- Expect: count >= 110.
- Verify (spot-check tab coverage): `psql -U postgres -d osticket_dev -c "SELECT count(*) FROM config WHERE key IN ('helpdesk_title','helpdesk_url','default_timezone_id','default_ticket_status','default_priority_id','default_help_topic','admin_email','strip_quoted_reply','landing_page_id','offline_page_id','thank-you_page_id','log_level','staff_session_timeout')"` returns 13 (every named key present).
- Status: [x]

### AC-2: FS-032 — the default-object bindings are seeded and resolve to real ids. [API-ONLY]
- Request: `psql -U postgres -d osticket_dev -c "SELECT key, value FROM config WHERE key IN ('default_dept_id','default_sla_id','max_page_size','passwd_reset_period') ORDER BY key"`
- Expect: all four present, values non-empty; `default_dept_id` equals the Support `dept_id` and `default_sla_id` equals the default SLA `id`.
- Verify (referential): `psql -U postgres -d osticket_dev -c "SELECT (SELECT value::int FROM config WHERE key='default_dept_id') = (SELECT dept_id FROM department WHERE dept_name='Support')"` returns `t`.
- Status: [x]

### AC-3: KL-032.3 — send_sys_errors is stored as its real value. [API-ONLY]
- Request: `psql -U postgres -d osticket_dev -c "SELECT value FROM config WHERE key='send_sys_errors'"`
- Expect: exactly one row; value reflects the seeded FS-032 default (a stored real value, not forced `0` for display).
- Status: [x]

### AC-4: No duplicate keys — config.key is unique. [API-ONLY]
- Request: `psql -U postgres -d osticket_dev -c "SELECT key, count(*) FROM config GROUP BY key HAVING count(*)>1"`
- Expect: zero rows (the `config_key_key` UNIQUE constraint holds; M2/M3 keys reconciled, not duplicated).
- Status: [x]

### AC-5: Admin account is seeded — `admin`/`Admin123!` with isadmin=true (infra for all M4 admin epics). [API-ONLY]
- Setup: `cargo run -p tools --bin seed -- --reset`
- Request: `psql -U postgres -d osticket_dev -c "SELECT username, isadmin, isactive FROM staff WHERE username='admin'"`
- Expect: one row, `isadmin=t`, `isactive=t`.
- Verify (login works): `curl -s -X POST http://localhost:3701/api/staff/login -H 'Content-Type: application/json' -d '{"username":"admin","password":"Admin123!"}' -i` returns `200`, body `{"ok":true,...}`, and a `Set-Cookie` staff session (the argon2id hash verifies).
- Verify (idempotent): re-running `seed --reset` leaves exactly one `admin` row (upsert on username, no duplicate).
- Status: [x]

## Test Infrastructure

- Uses `cargo run -p tools --bin seed -- --reset`. No dev endpoint required for these ACs.
- **Provides for downstream M4 epics**: the `admin`/`Admin123!` login (AC-5) is the credential every
  M4 admin-panel `[BROWSER]` epic logs in with — document it in the epics' Setup lines.
- **Test-infra gap raised for downstream (not blocking PREP-C)**: because `seed_config` preserves
  existing values, a settings-tab E2E that writes a key cannot be reset by `--reset`. Downstream M4-A
  should either land a `--reset`-forces-config-defaults behaviour or a dev endpoint
  (`POST /api/dev/reset-config`). Flagged in the epic report; not an AC here.

## Dependencies

- **TS-M4-PREP-B**: `email_account`/`template_group`/`page` tables (for the id-valued default keys).
- **TS-M4-PREP-D**: seeded email_account/template_group rows to point `default_email_id`/`default_template_id` at.
