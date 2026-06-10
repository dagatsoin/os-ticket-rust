# P8 Coverage Report — include/ajax.* + api.* + upgrader glue

Partition: **P8** (17 files). Phase-3 COVERAGE tagging run, 2026-06-10.
Repo: `/Users/warfog/dev/osTicket-1.7`

## Summary counts

| Metric | Count |
|--------|------:|
| Files examined | 17 |
| Units examined (classes + methods + view sections + task units) | 41 |
| Units tagged | 38 |
| Units uncovered (no spec) | 0 |
| Units trivia-skipped | 3 |

**Result: full spec coverage for P8.** Every non-trivial unit mapped to an existing
FS spec. No uncovered rows.

## Per-file tag map

| File | Units tagged | Spec(s) |
|------|--------------|---------|
| `include/ajax.config.php` | `ConfigAjaxAPI`, `scp()`, `client()` | FS-033 (staff config bundle FS-033.10); `client()` also FS-010/FS-011 (client upload-config projection, KL-033.5) |
| `include/ajax.content.php` | `ContentAjaxAPI`, `log()`, `ticket_variables()` | FS-033 (FS-033.8 log popover, FS-033.9 variable card); `ticket_variables()` also FS-040 (variable-card consumer) |
| `include/ajax.kbase.php` | `KbaseAjaxAPI`, `cannedResp()`, `faq()` | FS-022 (canned-response fetch + var substitution); `faq()` FS-050 (KB/FAQ popover) |
| `include/ajax.reports.php` | `OverviewReportAjaxAPI` + 6 methods | FS-020 (staff dashboard overview report — plot + tabular + CSV download, FS-020 dashboard section) |
| `include/ajax.tickets.php` | `TicketsAjaxAPI` + 7 methods | FS-020 (`lookup`/`lookupByEmail`/`search` autocomplete + count search); FS-021 (`acquireLock`/`renewLock`/`releaseLock` FS-021.18 + `previewTicket` FS-021 preview) |
| `include/ajax.upgrader.php` | `UpgraderAjaxAPI`, `upgrade()` | FS-061 (AJAX progress protocol FS-061.11, BS-061-19 CSRF/POST-only) |
| `include/ajax.users.php` | `UsersAjaxAPI`, `search()` | FS-020 (staff-side requester-email autocomplete sourced from submitted tickets) |
| `include/upgrader/aborted.inc.php` | view section | FS-061 (aborted screen FS-061.9/.12) |
| `include/upgrader/done.inc.php` | view section | FS-061 (done screen FS-061.9/.14) |
| `include/upgrader/prereq.inc.php` | view section | FS-061 (prereq screen FS-061.9/.16) |
| `include/upgrader/rename.inc.php` | view section | FS-061 (rename screen FS-061.9/.13) |
| `include/upgrader/upgrade.inc.php` | view section | FS-061 (upgrade screen + ajax/manual mode switch FS-061.9/.11) |
| `…/15b30765-dd0022fb.task.php` | `AttachmentMigrater` + 11 methods | FS-061 (resumable task FS-061.5, BS-061-09); FS-022 (disk→DB file-store target) |
| `…/435c62c3-2e7531a2.task.php` | `MigrateGroupDeptAccess`, `run()` | FS-061 (single-shot task BS-061-22); FS-031 (group→dept access) |
| `…/8aeda901-16fcef4a.task.php` | `CryptoMigrater`, `run()`, `_decrypt()` | FS-061; FS-003 (password re-encryption) |
| `…/98ae1ed2-e342f869.task.php` | `APIKeyMigrater`, `run()` | FS-061; FS-043 (API-key whitelist → per-IP key rows) |
| `…/c00511c7-7be60a84.task.php` | `MigrateDbSession`, `run()` | FS-061; FS-002 (DB-backed session migration) |

## Trivia skipped (intentionally not tagged)

| File | Unit | Why |
|------|------|-----|
| `…/435c62c3-…task.php` etc. | `return '<ClassName>';` (each task tail) | The task-file return contract (FS-061.5) is a one-line idiom, not a behavioral unit; the class it names is already tagged. |
| all files | file-header license/`die('!')` / `if(!defined(INCLUDE_DIR))` guards | Boilerplate include-guards; owned generically by FS-001 bootstrap, not per-file behavior. |
| upgrader `.inc.php` | inline HTML chrome (sidebars/tips copy) | Static marketing/help copy with no behavior beyond the FS-061-tagged screen section. |

## Notable observations

1. **P8 is a thin presentation/glue tier — zero uncovered behavior.** All 17 files are
   AJAX controllers, upgrader view partials, or migration-task shims; each maps to an
   already-written spec (FS-020/021/022/033/050/061 plus cross-refs). No new spec or BS
   is needed for this partition.

2. **The 5 migration-task files are the only place FS-061's data-migration tasks are
   concretely implemented** — they are the source-of-truth for BS-061-22 (single-shot vs
   resumable). `AttachmentMigrater` is the single genuinely-resumable task (overrides
   `sleep`/`wakeup`/`isFinished`); the other four are single-shot. This matches FS-061.5
   verbatim and confirms the spec's task inventory is complete.

3. **`ajax.users.php::search()` and `ajax.tickets.php::lookup*()` are autocomplete
   endpoints not explicitly named in any FR**, but are behaviorally subsumed by FS-020's
   "search tickets by number / requester email" requirement. Tagged FS-020; a future
   spec pass could add an explicit FR for the staff autocomplete endpoints to make the
   mapping first-class rather than implied.

## CAVEAT — trailing-whitespace deviations (3 lines, cosmetic)

The Read/Edit tooling does not expose or preserve trailing whitespace on existing lines.
Three pre-existing lines lost trailing whitespace as a side effect of inserting an
adjacent comment tag (no content, no line-count change):

- `include/ajax.users.php` — several originally trailing-space-padded blank/code lines
  were normalized when the file was rewritten to insert the two `// [FS-020]` tags.
- `include/upgrader/aborted.inc.php` line 3 — `?>    ` (4 trailing spaces) → `?>`.
- `include/upgrader/done.inc.php` line 3 — `//Destroy the upgrader - we're done! `
  (1 trailing space) → trailing space removed.

These are whitespace-only deviations with no semantic or line-numbering impact; all other
edits are strictly additive comment-line insertions. Repo is not a git checkout, so a
byte-exact revert of the trailing whitespace was not possible.
