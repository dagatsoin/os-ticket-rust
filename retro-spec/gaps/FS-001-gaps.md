# FS-001 Gap Report — App Bootstrap & Shared Request Lifecycle

Phase-2 GAP-CLOSE run. Source slice: `main.inc.php`, `client.inc.php`, `scp/staff.inc.php`,
`scp/admin.inc.php`, `include/class.osticket.php`, `include/class.config.php`,
`include/class.nav.php`, `include/class.csrf.php`, `offline.php`, `index.php`.

Scope guard honored: spec-only edits; no source file modified.

| ID | Type | Severity | What the code does | What the spec said | Fix applied |
|----|------|----------|--------------------|--------------------|-------------|
| G1 | MISSING | Medium | `osTicket` exposes request/env utilities: `is_https()` (server flag `on` OR `X-Forwarded-Proto: https`), `is_cli()` (sapi `cli` OR no method+host), `get_path_info()` (`PATH_INFO`→`ORIG_PATH_INFO`→null), `get_var()`/`get_db_input()` (typed safe accessor), `isFileTypeAllowed()` (`.*` wildcard / lowercased 3-4 char ext match), `getDBSignature()`. | Overview claimed ownership of "request/environment utilities" but never enumerated them; none had a FR. | Added **FS-001.16** documenting all six utility groups. Added **EC-018** (HTTPS via proxy) and **EC-019** (CLI execution). |
| G2 | MISSING | Medium | `OsticketConfig` constructor: when `core`-namespace query returns 0 rows, falls back to loading single config row id=1 and wrapping each column as a value. `getSchemaSignature` is 3-tier (namespaced key → id=1 column → `md5(getDBVersion())`). | No mention of legacy single-row hydration fallback or tiered schema-signature lookup. | Added **BS-016**, extended **FS-001.5** and **FS-001.7**, added **EC-017**. |
| G3 | MISSING | Medium | Config constructor lazily derives & session-persists `tz_offset` from `Timezone::getOffsetById(default_timezone_id)`, or `0` when no timezone id configured; the seeded session `TZ_OFFSET` reads through this. | Spec said session TZ is seeded from `getTZoffset()` but did not document that `tz_offset` itself is lazily computed/cached. | Added **BS-015**, added derivation bullet to **FS-001.5**. |
| G4 | IMPRECISE | Medium | Failed CSRF check calls `logWarning(..., false)` → admin alert explicitly SUPPRESSED; log title `Invalid CSRF Token <name>`; POST field checked before header. | Spec said failure is "recorded as a warning" (implying default alert=true via logWarning). | Refined **FS-001.11**: noted alert suppression, exact title, and field-before-header order. |
| G5 | IMPRECISE | Low-Med | StaffNav: tab key is `kbase` (not `knowledgebase`); My-Tickets href `tickets.php?status=assigned`; New-Ticket href `tickets.php?a=open`; always-present "Tickets" drop-only entry; Dashboard submenu = Dashboard/Staff Directory/My Profile; FAQs entry also matches `faq.php`. UserNav uses `getNumTickets()`/`getTicketID()`, exact section-key match, ignores unknown keys. | Spec gave only generic gated-entry descriptions and `tickets.php` hrefs; tab key, query-string targets, drop-only entries, and Dashboard submenu were absent. | Rewrote both nav bullets in **FS-001.13** with exact keys/hrefs/submenus. |
| G6 | IMPRECISE | Low-Med | Admin writable-config warning requires `is_writable(CONFIG_FILE)` first, then `clearstatcache()` + `fileperms()` bit check `0x0002` (world) / `0x0010` (group); register-globals warning only when no prior warning set, message verbatim. | Spec said "group- or world-writable" without the `is_writable` pre-gate, bit values, or cache-clear; register-globals message not quoted. | Refined **FS-001.10** clause (c)/(d); added **KL-010** for the writability-probe pre-gate suppression case. |
| G7 | IMPRECISE | Low | `alertAdmin` sends via alert email account → default email account → system mailer (fixed `"osTicket Alerts"` from-name, from = recipient); appends THISPAGE; when asked to log, logs at LOG_CRIT WITHOUT re-alerting (loop guard). | Spec covered recipient/account fallback but omitted from-name detail and the no-re-alert loop guard. | Refined **FS-001.12** admin-alerting bullet. |
| G8 | INCORRECT (bug) | Low | `getError()` reads `$this->system['err']`; `setError()` writes `$this->system['error']` — single system error can never be round-tripped through the pair. | Spec did not document the error accessor; latent getter/setter key mismatch undocumented. | Added **KL-009** and **EC-020** documenting the latent bug. |
| G9 | IMPRECISE | Low | Two distinct password-reset config keys: `pw_reset_window` (minutes→seconds on read) and legacy `passwd_reset_period` (separate accessor, not converted). | Data Requirements listed only `pw_reset_window`. | Annotated the Password-reset row in Data Requirements. |

## Summary

- **Gaps found / fixed: 9** — MISSING ×3 (G1, G2, G3), IMPRECISE ×5 (G4, G5, G6, G7, G9), INCORRECT ×1 (G8).
- **New spec artifacts:** 1 FR (FS-001.16), 2 BS (BS-015, BS-016), 4 EC (EC-017, EC-018, EC-019, EC-020), 2 KL (KL-009, KL-010). Plus in-place refinements to FS-001.5, FS-001.7, FS-001.10, FS-001.11, FS-001.12, FS-001.13 and the Data Requirements table.
- No source files modified.
