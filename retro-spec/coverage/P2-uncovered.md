# P2 Coverage Report — `scp/*.php` admin/staff controllers

Partition: **P2** (37 files, ~3,202 non-empty source lines)
Phase-3 COVERAGE tagging run — prepared 2026-06-10
Repo root: `/Users/warfog/dev/osTicket-1.7`

Method: every controller in P2 follows osTicket's uniform thin-controller shape
(`require` shell → object lookup guard → `if($_POST){ switch($_POST['do']){ … } }`
action dispatch → page-selection → header/body/footer render). Each `case` branch,
each lookup guard, each `$_POST` dispatch, and each render-dispatch block was treated
as a "significant top-level code section" and tagged with the owning FS spec id.

## Spec-mapping summary (tagged units)

| File | Owning spec(s) | Notes |
|------|----------------|-------|
| `admin.php` | FS-032 | redirect → settings |
| `admin.inc.php` | FS-001, FS-002, FS-061, FS-090 | admin realm gate + security notices + nav |
| `ajax.php` | FS-002, FS-043, FS-033 | staffLoginPage override + URL dispatcher routes |
| `apikeys.php` | FS-043 | API-key CRUD + mass enable/disable/delete |
| `attachment.php` | FS-022 | staff attachment download + access-hash validation |
| `autocron.php` | FS-043 | autocron web fallback (FS-043.11) |
| `banlist.php` | FS-042 | SYSTEM BAN LIST rule CRUD |
| `canned.php` | FS-022 | canned-response CRUD + attachments |
| `categories.php` | FS-032, FS-050 | FAQ-category CRUD (perm gate references FS-050) |
| `dashboard.php` | FS-020 | activity chart + tabular stats scaffold |
| `departments.php` | FS-030 | department CRUD |
| `directory.php` | FS-031 | staff directory |
| `emails.php` | FS-040 | email-account CRUD |
| `emailtest.php` | FS-040 | outbound email diagnostic |
| `faq.php` | FS-050 | staff FAQ manage/publish/unpublish/delete |
| `file.php` | FS-022 | hash-validated file download |
| `filters.php` | FS-042 | ticket-filter CRUD |
| `groups.php` | FS-031 | permission-group CRUD |
| `helptopics.php` | FS-030 | help-topic CRUD |
| `image.php` | FS-022 | hash-validated image display |
| `index.php` | FS-020 | falls through to ticket queue |
| `kb.php` | FS-050 | staff KB landing |
| `l.php` | FS-003, FS-001 | clickable-URL redirect endpoint + link-token check |
| `login.php` | FS-002 | staff login + CSRF rotate |
| `logout.php` | FS-002 | staff logout (+ known link-token defect KL-002-10) |
| `logs.php` | FS-033 | syslog mass-delete |
| `pages.php` | FS-033 | Site Pages CRUD |
| `profile.php` | FS-002, FS-031 | self-service profile + forced pw change |
| `pwreset.php` | FS-002 | password-reset steps 2/3/5 |
| `settings.php` | FS-032 | 7-tab settings save + render |
| `slas.php` | FS-032 | SLA-plan CRUD |
| `staff.php` | FS-031 | staff-account CRUD |
| `staff.inc.php` | FS-001, FS-002, FS-061, FS-090 | staff session/auth/CSRF gate + nav |
| `teams.php` | FS-030 | team CRUD |
| `templates.php` | FS-040 | email-template-set CRUD |
| `tickets.php` | FS-011, FS-020, FS-021 | queue/nav/export + single-ticket workflow + open |
| `upgrade.php` | FS-061 | upgrader wizard state machine |

## Uncovered units

No **non-trivial** unit in P2 was found that lacks an owning spec. Every controller
action branch, lookup guard, render dispatch, and the autocron transport are documented
by an existing FS spec. The items below are recorded as *weak / quirk* coverage rather
than true gaps — they are tagged (behavior owned by the listed spec) but the spec documents
them only as a known-defect or as an undocumented helper detail.

| File:line (approx) | Unit | Behavior | FS/BS guess | Suggested owning spec |
|--------------------|------|----------|-------------|-----------------------|
| `logout.php`:19-28 | missing/invalid `auth` fall-through | redirect header set but no `exit`; session destroyed regardless of link-token | KL-002-10 / EC-002-14 | FS-002 (already documents the defect) |
| `pwreset.php`:57-59 | `newpasswd` token-window check | `lastModified()`/`getPwResetWindow()` expiry comparison has inverted boolean (`!($ts=…)`) — likely-buggy window enforcement | EC/KL under FS-002 | FS-002 (add KL for reset-window logic quirk) |
| `staff.inc.php`:128-134 | forced-pw-change `require('profile.php')` for AJAX/API | code XXX-comments that AJAX/API should bypass forced-pw redirect but does not | KL under FS-002 | FS-002 |
| `tickets.php`:547 | `a=print` PDF export inline in render dispatch | PDF print gated only by ticket presence, not an explicit print permission | FS-021.x (PDF print) | FS-021 (already owns; confirm no perm gap is intended) |

These are documentation-completeness observations, not untagged code. No new spec is
required for any P2 unit.

## Trivia skipped (not tagged, by design)

The following are pure boilerplate / shell glue with no standalone behavior — license
header block comments, `require('staff.inc.php')` / `require('admin.inc.php')` bootstrap
lines, `$nav->setTabActive(...)`, and the trailing
`require(header.inc.php) … require($page) … include(footer.inc.php)` render trio. The
render trio's *page-selection* logic (which `.inc.php` to load) WAS tagged; the literal
header/footer includes were not (they are shell chrome owned by FS-090, not per-controller
behavior).

## Summary counts

| Metric | Count |
|--------|------:|
| Files in partition | 37 |
| Files processed | 37 |
| Significant units examined | ~150 |
| Units tagged | ~146 |
| Units uncovered (true gap) | 0 |
| Units weak/quirk (tagged, doc-completeness note) | 4 |
| Trivia / shell-glue skipped | ~90 (headers, bootstrap requires, setTabActive, header/footer includes) |

All P2 edits were additive comment-only insertions (`// [FS-0XX]` in PHP context,
`<!-- [FS-0XX] -->` in HTML context). No existing line was modified, moved, or deleted.
