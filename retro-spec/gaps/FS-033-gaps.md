# FS-033 Gap Report — Admin: System Logs, Site Pages & Content

Source slice re-read: `scp/logs.php`, `scp/pages.php`, `include/staff/syslogs.inc.php`,
`include/staff/pages.inc.php`, `include/staff/page.inc.php`, `include/class.log.php`,
`include/class.page.php`, `include/ajax.config.php`, `include/ajax.content.php`,
`include/class.osticket.php` (`purgeLogs`), `include/class.config.php` (page/log getters),
`pages/index.php`, `index.php`, `offline.php`, `open.php`.

| id | type | severity | what the code does | what the spec said | fix applied |
|----|------|----------|--------------------|--------------------|-------------|
| G-01 | INCORRECT | CRITICAL | A full **Site Pages** CMS exists: `scp/pages.php`, `class.page.php` (`Page` entity over `ost_page`), `pages.inc.php` (list) + `page.inc.php` (add/edit). Pages back the landing page (`index.php`), offline banner (`offline.php`), thank-you page (`open.php`), and public `other` pages by slug (`pages/index.php`). | KL-033.1 + two scope-boundary notes asserted "there is no `scp/pages.php`, no `class.page.php`, no `page(s).inc.php`, and no `ost_page` content-management feature... no editable site pages exist in osTicket 1.7." | Retitled spec; added FS-033.11–.16, BS-033.8/.9/.10, EC-033.11–.15, Site Pages data store, page flows (Flow 6/7), 6 new dependency rows; inverted KL-033.1 to document real Pages constraints with a correction note; fixed Future Considerations. |
| G-02 | MISSING | HIGH | Page create/edit form: required Name (unique, tag-stripped) + Type (closed enum) + body (HTML-safened) + active radio + admin notes; messages "Page added/updated successfully", "Name already exists", "Type required/Invalid selection", "Page body is required", "Internal error. Try again". | No coverage. | Added FS-033.13 (create/edit) + FS-033.14 (validation/uniqueness). |
| G-03 | MISSING | HIGH | Bulk enable/disable/delete with in-use protection: default pages cannot be disabled/deleted (only enabled); per-page in-use guard skips; messages "Selected pages enabled/disabled/deleted...", "N of M...", "Unable to...", "...is in-use and CANNOT be disabled/deleted.", "You must select at least one page."; delete resets referencing `topic.page_id=0`. | No coverage. | Added FS-033.15 (bulk) + FS-033.16 (entity guards) + BS-033.9 (default/in-use protection). |
| G-04 | MISSING | MEDIUM | Page types `landing`/`offline`/`thank-you`/`other`; only `thank-you` supports ticket variables; active `other` pages served publicly at `pages/<slug>`; inactive/non-other → 404. | No coverage. | Added BS-033.8 (type set) + BS-033.10 (public slug serving) + EC-033.15. |
| G-05 | MISSING | MEDIUM | Pages list nav tab is `manage`; default sort `name` ASC; sort keys name/status/created/updated; status shows Active / **Disabled** + `(in-use)`. | No coverage; spec only documented logs (nav `dashboard`, default id DESC). | Added FS-033.11 (gate/`manage` tab) + FS-033.12 (table/sort/pagination). |
| G-06 | MISSING | LOW | Pages POST unknown-command/action messages: "Unknown action/command" / "Unknown action - get technical help."; unknown page id → "Unknown or invalid page" / update "Invalid or unknown page". | No coverage. | Added EC-033.13 + EC-033.14. |
| G-07 | IMPRECISE | LOW | Log type filter injects the **raw request casing** into `log_type=...`; the dropdown re-highlight compares lowercase `$type` against capitalized labels (so it may not re-select). | FS-033.2 implied a clean normalized `log_type` equality; casing nuance undocumented. | Added KL-033.6. |
| G-08 | IMPRECISE | LOW | Bulk page **disable** partial message interpolates unset `$num` (loop counts into `$i`), so the count may render blank/wrong. | No coverage. | Added KL-033.7. |
| G-09 | IMPRECISE | LOW | `purgeLogs()` no-ops when grace period is falsy — that includes `0`/"Never" (the FS-032 select offers `0`=Never), not only empty/non-numeric. | EC-033.10 / BS-033.6 said only "empty/non-numeric". | Clarified EC-033.10 to include `0`/"Never". |

## Summary
- Gaps found / fixed: **9** (1 INCORRECT, 5 MISSING, 3 IMPRECISE).
- By severity: 1 CRITICAL, 2 HIGH, 3 MEDIUM, 3 LOW.
- New spec content: **+6 FR** (FS-033.11–.16), **+3 BS** (BS-033.8/.9/.10),
  **+5 EC** (EC-033.11–.15), **+2 KL** (KL-033.6/.7), 1 KL inverted (KL-033.1),
  + Site Pages data store, 2 user flows, 6 dependency rows. No existing ids renumbered.

## Most significant
1. **G-01 (CRITICAL)** — the spec flatly denied the existence of a feature that is fully
   implemented (Site Pages / `ost_page`). The original FS-033 covered roughly half its
   intended domain. This was the single largest correction.
2. **G-03 (HIGH)** — the in-use protection semantics (default pages enable-only; in-use
   pages refuse disable/delete; delete cascades `topic.page_id=0`) are non-obvious business
   rules with real data-integrity consequences, now captured in BS-033.9 + FS-033.15/.16.
3. **G-07 (LOW but subtle)** — the log type filter's raw-casing SQL match is a latent
   case-sensitivity bug that only manifests via hand-built URLs / collation choice; worth
   documenting so implementers don't "fix" the UI casing and break the dropdown path.
