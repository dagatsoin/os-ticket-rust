# Retag P9 — `include/staff/*.inc.php` chunk A (apikey→settings) + client/*

Converted Phase-3 bracketed spec tags (`[FS-0XX]`) into requirement-level `@implements`
annotations. COMMENT-ONLY edits; no non-comment line touched.

## Concurrency note

A parallel pass converted a large subset of P9 files between the initial grep snapshot and
this run. Files found already in `@implements` form were **left untouched** (verified clean of
`[FS-` tags, correct format). They are listed below under "already converted (not by me)".

## Files I converted

| File | Owning spec(s) | Old tag count | New @implements lines | FS.N | FS fallback | BS/EC/KL |
|------|----------------|--------------:|----------------------:|-----:|------------:|---------:|
| apikey.inc.php | FS-043 | 5 | 7 | 5 | 0 | 2 (BS-430, BS-431) |
| apikeys.inc.php | FS-043 | 5 | 5 | 5 | 0 | 0 |
| login.tpl.php | FS-002 | 2 | 2 | 1 | 0 | 1 (BS-002-09) |
| login.header.php | FS-090 | 2 | 1 | 0 | 1 (FS-090) | 0 |
| pwreset.php | FS-002 | 2 | 1 | 1 | 0 | 0 |
| pwreset.login.php | FS-002 | 2 | 1 | 1 | 0 | 0 |
| pwreset.sent.php | FS-002 | 2 | 1 | 1 | 0 | 0 |
| group.inc.php | FS-031 | 4 | 6 | 3 | 0 | 3 (BS-031-020, BS-031-021) |
| groups.inc.php | FS-031 | 4 | 5 | 4 | 0 | 1 (BS-031-034) |
| directory.inc.php | FS-031 | 4 | 6 | 4 | 0 | 2 (BS-031-030, BS-031-031) |
| profile.inc.php | FS-031 / FS-002 | 4 | 7 | 4 | 0 | 3 (BS-031-012, BS-031-013) |
| departments.inc.php | FS-030 | 4 | 5 | 4 | 0 | 1 (BS-030-05) |
| helptopics.inc.php | FS-030 | 3 | 3 | 3 | 0 | 0 |
| banrule.inc.php | FS-042 | 3 | 4 | 3 | 0 | 1 (BS-042-14) |
| banlist.inc.php | FS-042 | 5 | 5 | 5 | 0 | 0 |
| filter.inc.php | FS-042 | 5 | 8 | 7 | 0 | 1 (BS-042-09) |
| filters.inc.php | FS-042 | 3 | 3 | 3 | 0 | 0 |
| faq.inc.php | FS-050 / FS-022 | 4 | 6 | 6 | 0 | 0 |
| faq-view.inc.php | FS-050 | 3 | 4 | 4 | 0 | 0 |
| page.inc.php | FS-033 | 2 | 2 | 2 | 0 | 0 |
| header.inc.php | FS-090 | 6 | 7 | 7 | 0 | 0 |
| email.inc.php | FS-040 | 5 | 5 | 5 | 0 | 0 |
| emails.inc.php | FS-040 | 3 | 4 | 4 | 0 | 0 |

Totals (my edits): 22 files, **88 old tags → 102 @implements lines**
(FS.N = 95, FS fallback = 1, BS = 6).

## Already converted (not by me — verified `@implements`, left untouched)

department.inc.php, helptopic.inc.php, faq-category.inc.php, faq-categories.inc.php,
categories.inc.php, category.inc.php, cannedresponse.inc.php, cannedresponses.inc.php,
attachment.inc.php, footer.inc.php, pages.inc.php.

(index.php carries no coverage tags — 3-line stub.)

## Granularity decisions / re-attributions

- **email.inc.php L144/L213**: Phase-3 tagged the inbound-fetch and SMTP blocks as `[FS-041]`
  and `[FS-040]` respectively. Per FS-040.2 acceptance criteria, BOTH the "Mail Account
  (inbound fetch)" and "SMTP (outbound)" blocks are part of the **Email Account Create/Edit
  Form** — re-attributed both to **FS-040.2** (FS-041 owns runtime fetch mechanics, not the
  form fields).
- **login.header.php** (FS-090 fallback): the login-page chrome (head assets / noindex /
  autofocus, shared by login + pwreset templates) has no precise FR. FS-090.10 is the
  *authenticated SCP header* (`header.inc.php`), which is a different surface. Used spec-level
  `FS-090` fallback rather than mis-pinning to FS-090.10.
- **header.inc.php**: split the 6 spec-level tags into precise FRs — head/identity →
  FS-090.10, global bars → FS-090.12, per-page bars → FS-090.13, top-nav render →
  FS-090.8, sub-nav render + auto-highlight → FS-090.9 + FS-090.7.
- **profile.inc.php / group.inc.php / directory.inc.php**: lifted BS rules to first-class
  annotations (BS-031-012 per-staff prefs, BS-031-013 password rules, BS-031-020 11-flag
  matrix, BS-031-021 group↔dept matrix, BS-031-030/031 directory search/visibility).
- **faq.inc.php / faq-view.inc.php**: dual-tagged the canManageFAQ gate (FS-050.8) alongside
  the form/view FR (FS-050.16 / FS-050.13); the session-bound attachment hash link →
  FS-022.10.

## Ambiguous cases

**1** ambiguous case requiring spec-level fallback:

- **login.header.php** — login-page head chrome; no requirement-level FR matches precisely
  (FS-090.10 covers the *authenticated* SCP header, not the login template). Resolved with
  `@implements FS-090` spec-level fallback + explanatory note. All other 21 files resolved to
  precise `FS-XXX.N` / BS ids.
