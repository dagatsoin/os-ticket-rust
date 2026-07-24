# Retag Report — Partition P1 (root entry scripts + top-level includes)

Date: 2026-06-10
Format applied: requirement-level `@implements FS-XXX.N` / `BS-XXX` / `KL-NNN` annotations,
one id per line, note required (em-dash + what the unit does), replacing the Phase-3
bracketed `// [FS-0XX]` tags. Comment-only edits; no source lines touched.

## Per-file breakdown

| File | Old `[FS-]` tags | New `@implements` lines | FS.N-level | FS-level fallback | BS | KL | Ambiguous |
|------|-----------------:|------------------------:|-----------:|------------------:|---:|---:|----------:|
| index.php | 3 | 7 | 5 | 0 | 1 | 0 | 0 |
| login.php | 2 | 5 | 4 | 0 | 1 | 0 | 0 |
| logout.php | 1 | 2 | 2 | 0 | 0 | 0 | 0 |
| logo.php | 2 | 4 | 3 | 0 | 1 | 0 | 0 |
| l.php | 1 | 2 | 2 | 0 | 0 | 0 | 0 |
| offline.php | 2 | 4 | 4 | 0 | 0 | 0 | 0 |
| open.php | 2 | 7 | 6 | 0 | 1 | 0 | 0 |
| tickets.php | 3 | 8 | 6 | 0 | 2 | 0 | 0 |
| view.php | 1 | 2 | 1 | 0 | 1 | 0 | 0 |
| attachment.php | 1 | 3 | 2 | 0 | 1 | 0 | 0 |
| captcha.php | 1 | 2 | 2 | 0 | 0 | 0 | 0 |
| ajax.php | 2 | 3 | 2 | 1 | 0 | 0 | 1 |
| main.inc.php | 10 | 23 | 9 | 0 | 9 | 5 | 0 |
| client.inc.php | 3 | 9 | 6 | 0 | 3 | 0 | 0 |
| secure.inc.php | 2 | 3 | 3 | 0 | 0 | 0 | 0 |
| pages/index.php | 1 | 3 | 2 | 1 | 0 | 0 | 1 |
| kb/faq.php | 1 | 4 | 3 | 0 | 1 | 0 | 0 |
| kb/file.php | 1 | 3 | 2 | 0 | 1 | 0 | 0 |
| kb/index.php | 1 | 2 | 2 | 0 | 0 | 0 | 0 |
| kb/kb.inc.php | 1 | 2 | 1 | 0 | 1 | 0 | 0 |
| api/api.inc.php | 1 | 3 | 2 | 0 | 1 | 0 | 0 |
| api/cron.php | 1 | 2 | 2 | 0 | 0 | 0 | 0 |
| api/http.php | 1 | 5 | 4 | 0 | 1 | 0 | 0 |
| api/index.php | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| api/pipe.php | 1 | 3 | 2 | 0 | 1 | 0 | 0 |
| include/api.cron.php | 6 | 8 | 6 | 0 | 2 | 0 | 0 |
| include/api.tickets.php | 9 | 18 | 13 | 0 | 5 | 0 | 0 |
| include/ost-sampleconfig.php | 2 | 5 | 4 | 0 | 1 | 0 | 0 |
| include/index.php | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| include/mysql.php | 28 | 30 | 28 | 0 | 2 | 0 | 0 |
| include/mysqli.php | 29 | 31 | 29 | 0 | 2 | 0 | 0 |
| include/class.misc.php | 11 | 11 | 10 | 0 | 1 | 0 | 0 |
| include/class.timezone.php | 9 | 11 | 10 | 0 | 0 | 1 | 0 |
| include/class.passwd.php | 3 | 5 | 3 | 0 | 2 | 0 | 0 |
| **Total** | **145** | **234** | **180** | **3** | **49** | **7** | **2** |

(api/index.php and include/index.php carried no tags — 3-line guard stubs — and were left
untouched.)

## Notable mapping decisions

- **DB driver files (mysql.php / mysqli.php)**: the `// [FS-003]` blanket tags resolved to
  precise FS-003 sub-requirements — FS-003.26 (db_input/db_real_escape), FS-003.27
  (db_query/db_squery/db_count), FS-003.28 (all result-set accessors + error/metadata),
  FS-003.29 (connection lifecycle + session variables). `db_connect`/`db_query` keep their
  bootstrap FS-001.5 co-tag (the spec explicitly dual-owns them). BS-026/BS-027/BS-028
  attached first-class. `db_version`/`db_create_database` re-homed to FS-060.6/FS-060.7
  (installer), correcting the original blanket `[FS-060]`.
- **main.inc.php**: each ordered bootstrap step mapped to its specific FS-001.N + the
  governing BS rule (BS-001..BS-006) and the relevant KL limitations (KL-001 forced
  error-display, KL-005 unconditional X-Forwarded-For trust, KL-006 dead BANLIST_TABLE).
  The installer-redirect line re-homed from a generic tag to BS-060-02.
- **class.timezone.php**: getName/getDesc tagged FS-003.25 + first-class **KL-013** (reads
  the never-populated `$info` slot — documented latent bug).
- **api.tickets.php / api.cron.php**: split FS-043 (HTTP/API ticket-create, key auth, cron)
  from FS-041 (email pipe intake, threading, MTA exit-code mapping) per method. PipeApiController
  response/exit-code mapping tagged BS-041.2; soft attachment validation BS-437.

## Ambiguous cases logged (2)

1. **ajax.php** (client AJAX `/config/client` route) — the dispatcher mechanics map cleanly
   to FS-043.13, but the `ConfigAjaxAPI::client()` bundle it routes to (client upload config:
   file types/size/max-uploads) has **no dedicated FS-033.N**. FS-033.10 covers only the
   sibling `/config/scp` staff bundle. Used **spec-level FS-033** with an explicit note that
   the client bundle has no dedicated requirement id.
2. **pages/index.php** (public custom-page servlet) — FS-033 owns Site Pages, but every
   enumerated requirement (FS-033.11–.16) is the **admin management** surface; there is no
   requirement for the public-facing servlet that resolves a slug to an active `'other'`-type
   page and prints its body. Used **spec-level FS-033** (note flags the missing public-render
   .N) alongside the precise FS-003.15 (slugify) and FS-010.9 (client shell boot).
