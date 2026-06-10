# Re-crawl Coverage Report — P8 (verification round)

Partition: **P8** — `include/ajax.*.php` + upgrader glue (`include/upgrader/*.inc.php`) + stream tasks (`include/upgrader/streams/core/*.task.php`)
Mode: READ-ONLY adversarial re-crawl. Did not trust existing `@implements` tags or prior reports.
Date: 2026-06-10

## Summary counts

| Metric | Count |
|--------|------:|
| Files in partition | 17 |
| Behavioral units walked (functions / screen scripts / migration tasks) | 47 |
| COVERED (tag present AND cited id verified) | 47 |
| UNCOVERED | 0 |
| MIS-TAGGED | 0 |
| TRIVIA (pure boilerplate, no behavior to spec) | 0 material |

**Result: DRY** — zero UNCOVERED / MIS-TAGGED findings.

## Spot-check performed (>30% of distinct cited ids)

~25 distinct ids verified against spec body text (not just existence). All matched the
behavior of the tagged unit:

- FS-033.8 / FS-033.9 / FS-033.10 (ajax.content, ajax.config) — verbatim sections in FS-033 (lines 226/246/276).
- FS-020.7 / FS-020.8 / FS-020.12 / FS-020.13, BS-020.2 / BS-020.15 / BS-020.16 (ajax.tickets, ajax.users, ajax.reports) — all present in FS-020; BS-020.16 three-query-shape rule (line 383) matches the assignee/staffId/status branching in `TicketsAjaxAPI::search()` exactly.
- FS-021.18 / FS-021.23a, BS-021.3 (ajax.tickets lock + preview) — FS-021 lines 244/316/350; FS-021.23a (line 324) correctly covers the permission-gated "More" action menu emitted by `previewTicket()`.
- FS-061.5 / 9 / 11 / 12 / 13 / 14 / 16, BS-061-03 (ajax.upgrader + all 5 upgrader/*.inc.php screens + AttachmentMigrater) — all present in FS-061; BS-061-03 admin-only gate matches the `!$thisstaff->isAdmin()` die guard on every screen and the 403 in `UpgraderAjaxAPI::upgrade()`.
- FS-022.12 / FS-022.14 (AttachmentMigrater next/queue + ajax.kbase cannedResp) — FS-022 lines 255/307.
- FS-050.10 (ajax.kbase faq) — FS-050 line 134.
- FS-043.1 (APIKeyMigrater) — FS-043 line 42; uppercase md5(ip.md5(key)) one-active-key-per-IP matches.
- FS-002.14 (MigrateDbSession) — FS-002 line 175.
- FS-003.1 (CryptoMigrater) — FS-003 line 29; two-key reversible encryption re-wrap matches.
- BS-031-021 (MigrateGroupDeptAccess) — FS-031 line 273; group↔dept CSV explode→join-row matches.
- FS-011.7 / FS-010.7 (ConfigAjaxAPI::client cross-refs) — FS-011 line 104 / FS-010 line 104.

## Findings (UNCOVERED / MIS-TAGGED)

None.

## Non-blocking observations (NOT findings — recorded for completeness)

These are correct-family informational cross-refs on co-located helper calls, not the
primary `@implements` binding of the unit. No id is wrong; they are merely less granular
than an available sub-rule. No action required under the MIS-TAGGED bar.

| File:line | Unit | Observation |
|-----------|------|-------------|
| `include/ajax.reports.php:162` | `OverviewReportAjaxAPI::downloadTabularData` | Cites bare `FS-090` for the `Http::download` delegation; more precise candidates exist (`FS-090.26` Report Tabular CSV Export Helper, `FS-090.24` File-Download Response Helper). Primary tag `FS-020.13` is correct. |
| `include/ajax.content.php:41` | `ContentAjaxAPI::ticket_variables` | Cites bare `FS-040` as the substitution authority the card documents; `FS-040.11` (Variable Substitution Grammar) is the precise rule. Primary tag `FS-033.9` is correct. |

## Coverage notes

- Every behavioral block in all 17 files carries an `@implements` tag; there are no
  untagged functions, screen scripts, or migration `run()`/`do_batch()`/`next()` bodies.
- The 5 upgrader `*.inc.php` files are presentation screens; each correctly carries the
  FS-061.9 screen-selection tag + its specific screen rule (12/13/14/16) + the BS-061-03
  admin gate. No view-only boilerplate was left dangling.
- The 5 stream `*.task.php` migrators each bind FS-061.5 (resumable task framework) PLUS
  the domain rule for what they migrate (attachments→FS-022.12, group-dept→BS-031-021,
  crypto→FS-003.1, api-key→FS-043.1, db-session→FS-002.14). All domain bindings verified.
