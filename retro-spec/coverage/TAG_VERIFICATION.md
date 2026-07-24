# TAG_VERIFICATION — Phase-3 [FS-XXX] Comment-Tag Insertion Audit

Date: 2026-06-10 · Branch: `develop` · Baseline: clean HEAD (see `work/backups/git-baseline.txt`)
Tooling: `work/coverage/verify_tags.py` (classifier/ws-repairer), `work/coverage/repair_violations.py` (violation repairs)

## Headline numbers

| Metric | Value |
|---|---|
| Modified tracked files | **224** |
| Added comment lines (final) | **1451** (1449 contain `[FS-` tags; 2 are continuation lines of one multi-line `//` block in `include/class.ticket.php` area) |
| Trailing-whitespace lines repaired (restored to HEAD bytes) | **80** across 23 files |
| Real violations found | **40** (all repaired, 0 remaining) |
| `php -l` | **224/224 pass** (php 5.6-cli via Docker; no local php) |
| Final plain `git diff` | `224 files changed, 1451 insertions(+)` — **zero deletions, zero modifications** |

## Method

Byte-level line diff (split on `\n`, CR preserved) per file vs `git show HEAD:<file>`, using
`difflib.SequenceMatcher` over trailing-whitespace-stripped lines (strip ` \t\r` — equivalent to
`git diff --ignore-space-at-eol`; note: this git build rejects `--ignore-trailing-space`, the
correct flag is `--ignore-space-at-eol`). Classification:

- stripped-equal but raw-bytes-different → trailing-whitespace-only → **restored HEAD bytes**
- pure insertion matching `^\s*(//|#|/\*|\*|<!--|<\?php\s*/\*)` → allowed comment addition
- any other insert / delete / replace → **real violation**

## Trailing-whitespace repairs (80 lines, 23 files)

`attachment.php` 36 (file was CRLF at HEAD; agent stripped every `\r` — all restored),
`include/ajax.users.php` 5, `include/class.ajax.php` 1, `include/class.attachment.php` 2,
`include/class.banlist.php` 3, `include/class.canned.php` 8, `include/class.category.php` 2,
`include/class.client.php` 1, `include/class.group.php` 1, `include/class.http.php` 1,
`include/class.knowledgebase.php` 1, `include/class.lock.php` 2, `include/class.pagenate.php` 1,
`include/class.priority.php` 1, `include/class.team.php` 2, `include/class.usersession.php` 3,
`include/staff/tickets.inc.php` 3, `include/upgrader/aborted.inc.php` 1,
`include/upgrader/done.inc.php` 1, `scp/attachment.php` 1, `scp/banlist.php` 1,
`setup/inc/install-done.inc.php` 2, `setup/inc/install.inc.php` 1.

## Real violations found and repaired (40 total — 0 remain; all `[FS-` tags preserved)

**A. Mutated existing `<?php` open-tag line (33 files).** Agents rewrote pre-existing line 1
`<?php` into `<?php /* [FS-xxx] */`. Semantically harmless but a line *modification*, not an
insertion. Repair: restored HEAD line 1 verbatim; moved the inline tag(s) to a newly inserted
`// [FS-xxx]` comment line 2. Files: `include/staff/{apikey,apikeys,attachment,banlist,banrule,
cannedresponse,cannedresponses,categories,category,department,departments,directory,email,emails,
faq-categories,faq-category,faq-view,faq,filter,filters,group,groups,header,helptopic,helptopics,
login.header,login.tpl,page,pages,profile,pwreset.login,pwreset,pwreset.sent}.inc.php` (header/
login files are `.php`). `attachment.inc.php` CRLF endings preserved.

**B. Comment appended to existing code line (2 files).** `setup/inc/install-done.inc.php` and
`setup/inc/subscribe.inc.php`: `// [FS-060] ...` appended to the existing
`<?php if(!defined('SETUPINC')) die('Kwaheri!');` line. Repair: restored HEAD line; comment moved
to its own inserted line 2 (still inside the open PHP block — valid).

**C. Blank line replaced (1).** `setup/inc/footer.inc.php`: original blank line 1 was consumed by
the inserted `<!-- [FS-060] ... -->`. Repair: blank line re-inserted after the comment.

**D. Blank line deleted (1).** `include/ajax.users.php`: pre-existing blank HEAD line 17 (between
the header-comment close and `if(!defined('INCLUDE_DIR'))`) was dropped. Repair: re-inserted.

**E. Syntax-breaking mid-PHP open tags (4 spots, `include/staff/tickets.inc.php`).** Agent
inserted `<?php /* [FS-020] ... */ ?>` + bare `<?php` pairs at former lines 58/108/203/268 —
*inside an already-open PHP block* (parse error: nested `<?php`). Repair: each pair collapsed to a
single pure `/* [FS-020] ... */` PHP comment line. (The identical pattern at the top of the file is
fine — there it precedes the original `<?php`, i.e. HTML mode.)

**F. (Found by php -l) `offline.php` line 28.** Inserted `<?php /* [FS-032] */` directly after an
existing `<?php` → parse error. Repair: converted to `// [FS-032]`. This passed the lexical
classifier (it matches the allowed `<?php /*` pattern) and was only caught by linting — counted
here in addition to the 40 lexical violations.

## php -l

No local `php` binary; ran `php -l` on all 224 modified files inside Docker `php:5.6-cli`
(era-appropriate for osTicket 1.7). First run: 1 failure (`offline.php`, fix F above).
After repair: **224/224 OK, 0 parse errors**.

## Final verification (post-repair)

- Classifier re-run: 0 whitespace deltas, 0 violations, 1451 comment insertions.
- `git diff -U0` (plain): **0 removed lines**; 1451 added lines, every one matching the comment
  regex `^\s*(//|#|/\*|\*|<!--|<\?php\s*/\*)`; 1449 contain `[FS-`.
- `git diff --stat`: `224 files changed, 1451 insertions(+)` — no deletion column at all.
- No `[FS-` tag removed anywhere (all repairs relocated tag text into pure comment lines).
- Nothing committed; working tree only.

## Verdict

**PASS.** The working tree now contains exclusively additive comment-line insertions over a
byte-identical HEAD baseline; all 224 modified PHP files lint clean.

---

# Round 2 — Re-verification after CLI/DB/timezone tagging pass (2026-06-10)

Scope: round-2 agents added ~98 `[FS-092]`/`[FS-003]`/`[FS-091]` comment tags in
`setup/cli/manage.php`, `setup/cli/package.php`, `setup/cli/modules/{class.module,deploy,unpack,import}.php`,
`include/mysql.php`, `include/mysqli.php`, `include/class.timezone.php`.

## Headline numbers

| Metric | Value |
|---|---|
| Modified tracked files | **229** (+5 vs round 1) |
| Added comment lines (final) | **1549** (+98 vs round 1; 1547 contain `[FS-`; same 2 untagged continuation lines as round 1) |
| Trailing-whitespace lines repaired (restored to HEAD bytes) | **3**, all in `include/class.timezone.php` |
| Real violations found | **3** (the 3 whitespace strips — the exact ones the agent disclosed; all repaired, 0 remaining) |
| `php -l` | **9/9 round-2-touched files pass** (php:5.6-cli via Docker), zero repairs needed |
| Final plain `git diff` | `229 files changed, 1549 insertions(+)` — **zero deletions, zero modifications** |

## Method

Same as round 1: plain `git diff` vs `git diff --ignore-space-at-eol` comparison plus
line-by-line classification of every `+` line against the comment regex
`^\s*(//|#|/\*|\*|<!--|<\?php\s*/\*)` / `[FS-` presence.

## Trailing-whitespace repairs (3 lines, 1 file)

`include/class.timezone.php` — agent had stripped trailing whitespace from:
- `    function getId() { ` (trailing space after `{`)
- the `        ` (8-space) blank line following that method's closing brace
- `        return $this->ht['offset'];    ` (4 trailing spaces)

All three restored verbatim from `git show HEAD:include/class.timezone.php`; all inserted
`[FS-003]`/`[FS-091]` comment lines in the file preserved. Pre-repair plain diff showed
`12 insertions / 3 deletions` for this file; post-repair `9 insertions / 0 deletions`.

## Round-2 file insertion counts (post-repair)

`include/mysql.php` 29, `include/mysqli.php` 29, `setup/cli/package.php` 11,
`setup/cli/modules/class.module.php` 10, `setup/cli/modules/import.php` 10,
`include/class.timezone.php` 9, `setup/cli/modules/unpack.php` 6,
`setup/cli/manage.php` 4, `setup/cli/modules/deploy.php` 3.

## php -l

Docker `php:5.6-cli`, all 9 round-2-touched files:
`No syntax errors detected` on every file. No tag caused a syntax issue; no repair needed.

## Final verification (post-repair)

- `diff <(git diff) <(git diff --ignore-space-at-eol)` → empty (no EOL-whitespace deltas anywhere).
- Plain `git diff`: 1549 added lines, 0 removed; every added line matches the comment regex;
  1547 contain `[FS-`.
- `git diff --stat`: `229 files changed, 1549 insertions(+)` — no deletion column.
- No `[FS-` tag removed; no code semantics changed; nothing committed.

## Verdict

**PASS.** Round-2 additions are exclusively comment-line insertions; the one disclosed
whitespace regression (3 lines) was repaired back to HEAD bytes; all touched files lint clean
under php 5.6.

# Round 3 — @implements conversion (2026-06-10)

Scope: a tag-format conversion rewrote ALL coverage tags from bracketed `// [FS-0XX]` to
requirement-level `@implements (FS|BS|EC|KL)-x.y: Title — detail` annotations across the
same 229 tracked PHP files. Verification + repair pass over the full working tree.

## Headline numbers

| Metric | Value |
|---|---|
| Modified tracked files | **229** (unchanged set) |
| Added lines (final) | **2293** — zero deletions, zero modifications |
| `@implements` tag lines | **2189** (FS 1709, BS 399, EC 44, KL 37) |
| Non-tag added lines | **104** = 52 bare `<?php` + 52 bare `?>` scaffolding lines (balanced, comment-only blocks in template files) |
| Bracket-tag residuals (`grep -rn '\[FS-'` excl. work/ + specs/) | **0** |
| Trailing-whitespace lines repaired (restored to HEAD bytes) | **39 lines across 14 files** |
| Real violations (code token changed/moved/deleted) | **0** |
| `php -l` (php:5.6-cli via Docker, all 229 files) | **229/229 pass, zero repairs** |
| Final plain `git diff` | `229 files changed, 2293 insertions(+)` |

## Method

Same as rounds 1–2: plain `git diff` vs `git diff --ignore-space-at-eol` comparison.
The `--ignore-space-at-eol` diff contained **0 removed lines**, proving every plain-diff
deletion (39) was an EOL-whitespace-only strip. Every added line was classified:
2103 pure comment lines, 86 one-liner `<?php /* @implements ... */ ?>` (HTML context),
52+52 bare `<?php`/`?>` tag lines. Each run containing bare tags was verified balanced
(open count == close count within the inserted run; 1513 insertion runs, 0 unbalanced) —
they wrap comment-only blocks or close/reopen an existing PHP block around a comment
group with no intervening output (newline after `?>` is consumed by PHP). `php -l`
confirms no parse damage.

## scp/categories.php (special attention)

Clean. Plain diff shows only `+ // @implements ...` lines; `$category=null;` sits at its
HEAD position (context line, unmoved); zero deletions. The previously-disclosed code-line
move is confirmed reverted.

## Trailing-whitespace repairs (39 lines, 14 files)

Restored verbatim from `git show HEAD:<file>`; all inserted comment lines preserved:

| File | Lines | File | Lines |
|---|---|---|---|
| include/class.lock.php | 10 | include/class.category.php | 2 |
| include/class.team.php | 9 | scp/attachment.php (CRLF; space before `\r`) | 2 |
| include/class.timezone.php | 3 (round-2 repairs re-stripped by conversion) | include/ajax.users.php | 1 |
| scp/banlist.php | 3 | include/class.banlist.php | 1 |
| include/api.cron.php | 2 | include/class.canned.php | 1 |
| include/class.ajax.php | 2 | include/class.group.php | 1 |
| | | include/class.http.php | 1 |
| | | include/class.knowledgebase.php | 1 |

Note: disclosure listed only 4 files (~17 lines); actual was 14 files / 39 lines —
10 additional files had 1–3 stripped lines each. All repaired.

## Format conformity

- `grep -rn '\[FS-' --include='*.php' .` (excl. work/, specs/): **0 hits**.
- Every added line containing a spec id (2189/2189) matches `@implements (FS|BS|EC|KL)-[0-9]` — 0 non-conforming.
- Note-pattern coverage: **96.5%** match `@implements XX-n: Title ...`; **87.7%** carry the
  full `: Title — detail` em-dash note. The ~3.5% remainder use an em-dash instead of a
  colon after the id (e.g. `@implements FS-003.4 — Automatic crypto backend selection`)
  or stop at the title — ids still fully conformant.
- 20-tag random sample (seed 42): all carried `: Title`; 17/20 carried the `— detail` segment.

## php -l

Docker `php:5.6-cli`, all 229 modified files: `No syntax errors detected` on every file.

## Final verification (post-repair)

- `diff <(git diff) <(git diff --ignore-space-at-eol)` → empty.
- Plain `git diff`: 2293 added lines, 0 removed, 0 modified.
- No `@implements` line removed; no code semantics changed; nothing committed.

## Verdict

**PASS.** The @implements conversion is comment-only end-to-end; all 39 whitespace strips
(14 files) repaired back to HEAD bytes; 0 bracket residuals; 0 real code violations;
229/229 files lint clean under php 5.6.

---

# Round 4 — re-crawl closure (2026-06-10)

Verification of the coverage re-crawl round that edited `@implements` comment lines in
10 PHP files (logout.php, logo.php, include/class.client.php, include/staff/templates.inc.php,
scp/admin.inc.php, include/class.config.php, include/staff/attachment.inc.php,
include/staff/teams.inc.php, include/client/faq.inc.php, include/staff/cannedresponses.inc.php)
and appended the new re-crawl ids to specs/FS-010 and specs/FS-040.

## Headline numbers

| Metric | Value |
|--------|------:|
| Final plain `git diff --stat` | `13 files changed, 68 insertions(+), 15 deletions(-)` |
| — PHP portion (10 files) | **+17 / −8** — every changed line is a comment line |
| — Specs portion | FS-010 +40/−1, FS-040 +7/−3, specs/CLAUDE.md +4/−3 (totals refresh) |
| `diff <(git diff) <(git diff --ignore-space-at-eol)` | **empty** (identical) |
| Non-comment changed lines in PHP diff | **0** (added and removed lines all `//`, `/*`, or `<?php /* … */`) |
| Trailing-whitespace repairs needed | **0** |
| Bracket `[FS-` residuals (excl. work/, specs/) | **0** |
| `php -l` (Docker php:5.6-cli) | **10/10 pass** |

## Trailing whitespace / line endings

- `git diff --check` flagged 3 added lines in `include/staff/attachment.inc.php` — false
  positive: that file is **natively CRLF at HEAD** (89 CR chars in both HEAD and working
  copy; 89 lines total). The inserted comment lines correctly follow the file's CRLF
  convention; the flagged "trailing whitespace" is the `\r` byte. No delta from HEAD
  conventions, so **no repair performed** (an LF "fix" would have been the regression).
- The single genuine trailing-space line in that file (line 68) is pre-existing at HEAD,
  untouched.
- All other 9 PHP files: LF at HEAD and in the working copy; `--ignore-space-at-eol`
  comparison empty across the whole tree.

## Format conformity (Task-2 spot-check) — 6 tag lines repaired

`grep -rn '\[FS-' --include='*.php' .` (excl. work/, specs/): **0 hits**.

Spot-check of the new/edited tag lines against the canonical
`@implements <id>: <Title> — <note>` shape (Title = exact spec section heading) found
**6 non-conforming lines, all repaired in place** (comment-only edits, re-linted):

| File | Tag | Defect | Repair |
|---|---|---|---|
| include/staff/attachment.inc.php (×3) | FS-032.1 | Title used the clause name "Attachments settings tab (out-of-band)" instead of the spec heading | Title → `Settings Panel Entry & Tab Routing`; clause moved into the note |
| include/class.config.php | FS-011.5 | Title "CAPTCHA on Public Forms" ≠ spec heading | Title → `CAPTCHA Challenge (Anonymous Submitters)` |
| include/class.config.php | KL-011.1 | Missing `<Title> — <note>` structure entirely | Title → `CAPTCHA Silently Disabled Without Image Capability` + em-dash note |
| include/client/faq.inc.php | KL-050.2 | Missing Title segment | Title → `Public Article "Last Updated" Reads the Category Timestamp` + em-dash note |

Three additional abbreviated titles were tightened to the exact spec headings
(KL-010.11 in class.client.php, BS-010.15 in logo.php, KL-030-12 in teams.inc.php —
the last keeps FS-030's dash-style id convention, matching its spec). The remaining
new tags (FS-010.8 ×6, BS-010.14, EC-010.13, KL-010.10, FS-001.10 ×2, FS-022.6 ×2,
KL-040.11, FS-050.7) verified conformant with ids present in their specs.

## php -l

Docker `php:5.6-cli`, all 10 touched files (post-repair): `No syntax errors detected`
on every file. **10/10 pass.**

## Verdict

**PASS.** The re-crawl round is comment-only end-to-end (0 non-comment changed lines,
0 whitespace deltas vs HEAD, diffs identical under `--ignore-space-at-eol`); 0 bracket
residuals; 6 tag-format repairs applied and re-verified; 10/10 lint clean under php 5.6.
New ids cross-checked present in specs: BS-010.14, BS-010.15, EC-010.13, KL-010.10,
KL-010.11 (FS-010) and KL-040.11 (FS-040; KL-040.5 factually corrected, not added).
