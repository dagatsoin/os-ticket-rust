# Retag P2 — scp/*.php

Converted all Phase-3 bracketed `// [FS-0XX]` coverage tags in partition **P2**
(`scp/*.php`) to requirement-level `@implements` annotations. Comment-only edits;
every file is byte-identical to HEAD except the rewritten tag comment lines.

- Files in partition: 37
- Files processed: 37 (all)
- Old bracketed tags: 153
- New `@implements` lines: 233 (one id per line; multi-requirement units expanded)
- Remaining `[FS-` brackets in partition: 0

## New-line classification

| Class | Count |
|-------|------:|
| FS-XXX.N (requirement-level) | 207 |
| FS-XXX (spec-level fallback) | 0 |
| BS-XXX (business rule) | 26 |
| EC-/KL- (edge case / limitation) | 1 (EC-042-1) |

No spec-level `FS-XXX` fallbacks were needed — every unit mapped to one or more
concrete requirement ids.

## Per-file (old bracketed count → new @implements count)

| File | old | new | notes |
|------|----:|----:|-------|
| pages.php | 6 | 8 | FS-033.11/.12/.13/.15 |
| helptopics.php | 6 | 7 | FS-030.2/.13/.14/.15/.17 |
| staff.php | 6 | 8 | FS-031.2/.3/.4 + BS-031-015 |
| teams.php | 6 | 7 | FS-030.2/.8/.9/.10/.12 |
| groups.php | 6 | 8 | FS-031.7/.8 + BS-031-023 |
| departments.php | 6 | 8 | FS-030.2/.3/.4/.5/.7 + BS-030-05 |
| categories.php | 7 | 8 | FS-032.13/.14/.15 (perm gate split FS-032/FS-050 → FS-032.15) |
| slas.php | 6 | 7 | FS-032.9/.10 (KL-032.4 copy-paste error noted) |
| settings.php | 2 | 3 | FS-032.1/.2 + BS-032.2 |
| banlist.php | 7 | 8 | FS-042.11 + BS-042-14 |
| filters.php | 6 | 8 | FS-042.1/.2/.10 + EC-042-1 |
| canned.php | 7 | 8 | FS-022.1/.2/.3/.6/.9 |
| apikeys.php | 6 | 6 | FS-043.1/.2 |
| emails.php | 6 | 8 | FS-040.1/.2/.3/.4 + BS-040.3 |
| templates.php | 8 | 10 | FS-040.5/.6/.7/.8/.9 |
| emailtest.php | 2 | 2 | FS-040.12 |
| faq.php | 6 | 6 | FS-050.11/.12/.13 |
| kb.php | 2 | 2 | FS-050.11 |
| logs.php | 3 | 3 | FS-033.1/.6 |
| image.php | 2 | 3 | FS-022.10/.11 + BS-022.8 |
| file.php | 2 | 3 | FS-022.10/.11 + BS-022.8 |
| attachment.php | 3 | 4 | FS-022.10/.11 + BS-022.8 |
| dashboard.php | 2 | 3 | FS-020.12/.13 |
| ajax.php | 3 | 6 | FS-043.3/.13 + FS-033.8/.10 + FS-002.11 |
| autocron.php | 2 | 3 | FS-043.11 + BS-435 |
| upgrade.php | 5 | 8 | FS-061.4/.5/.9/.13/.15/.16/.17 |
| admin.php | 1 | 1 | FS-032.1 |
| admin.inc.php | 3 | 5 | FS-001.10/.12 + FS-002.16 + FS-061.2 + FS-090.3 |
| staff.inc.php | 7 | 8 | FS-001.9 + FS-002.7/.11/.12/.16 + FS-061.2 + FS-090.2 |
| l.php | 1 | 2 | FS-003.9 + FS-001.16 |
| login.php | 2 | 3 | FS-002.1/.7/.8 |
| logout.php | 1 | 1 | FS-002.10 |
| pwreset.php | 4 | 5 | FS-002.3/.4/.7 |
| profile.php | 2 | 3 | FS-031.10/.11 + BS-031-032 |
| directory.php | 1 | 1 | FS-031.9 |
| index.php | 1 | 1 | FS-020.1 |
| tickets.php | 38 | 48 | FS-021.1–.21 + FS-020.1/.2/.9/.10/.11 + FS-090.23 + FS-042.12 + 9 BS rules |

## Ambiguous / noteworthy mappings (resolved, not fallback)

1. **categories.php permission gate** (was `[FS-032][FS-050]`): the canManageFAQ
   redirect gate is owned by FS-032.15 (FAQ Category Access Control); FS-050
   consumes categories but the gate code is FS-032's. Mapped to FS-032.15.
   (Edit preserved `$category=null;` at its original line — comment-only.)
2. **tickets.php case 'open'** (was `[FS-011]`): this is the staff-initiated
   create path, owned by FS-021.20 per the spec scope boundary (web/email/api
   creation is FS-011/041/043; only the staff path is FS-021). Mapped to FS-021.20.
3. **tickets.php banemail/unbanemail** (was `[FS-021][FS-042]`): dual-owned —
   the per-ticket workflow consequence (FS-021.14) plus the ban action mechanics
   (FS-042.12). Both emitted, one per line.
4. **ajax.php dispatcher table** (was `[FS-043][FS-033]`): the URL dispatcher is
   FS-043.3/.13; the two mounted routes called out by the original tag are the
   content-log (FS-033.8) and config-scp (FS-033.10) endpoints. All four emitted.
5. **slas.php lookup error** carries the literal "API key" copy-paste text
   (documented as KL-032.4); kept FS-032.9 with a note in the annotation.
6. **emails.php / staff.php / groups.php / departments.php mass_process** each got
   a first-class BS rule alongside the FR (BS-040.3, BS-031-015, BS-031-023,
   BS-030-05) for the self/association/default-protection guards.

No genuinely-ambiguous units required a spec-level fallback.
