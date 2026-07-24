# Retag P10 — `@implements` Conversion Report

Partition: **P10** (`include/staff/*.inc.php` chunk B + `include/client/*`)
Run date: 2026-06-10
Format applied: requirement-level `@implements FS-XXX.N: <Title> — <note>`, BS/EC/KL first-class, same comment mechanism preserved, COMMENT-ONLY edits.

## Scope note

The `include/staff/` directory is shared by partitions P9 and P10. This agent
owned **only the P10 file list** from `PARTITIONS.md` (settings-* → tpl + ticket-*
+ template/teams/slaplan/staff/staffmembers/syslogs). The P9 staff files
(directory, banlist, banrule, filter, filters, group, groups, profile,
departments, helptopics, emails, email, apikey, apikeys, faq, faq-view, page,
header, footer-banner, pwreset*, login.tpl, login.header) were already converted
by the P9 agent and were left untouched. The brief mentioned "kb/pages templates"
generically, but the authoritative P10 file list contains no `kb/` or top-level
`pages/` templates (those live in P1), so none were in scope.

All `[FS-...]` bracket tags in the P10 file set are now removed (verified: zero
matches remain in `include/staff` and `include/client`).

## Per-file conversions (P10-owned files)

| File | Old tags | New @implements lines | FS.N | FS fallback | BS/EC/KL |
|------|---------:|----------------------:|-----:|------------:|---------:|
| staff/settings-alerts.inc.php | 1 | 5 | 1 | 0 | BS-032.5, KL-032.3, KL-032.8, KL-032.9 |
| staff/slaplans.inc.php | 2 | 4 | 2 | 0 | KL-032.10, BS-032.12 |
| staff/settings-kb.inc.php | 1 | 4 | 2 | 0 | BS-032.1, BS-032.4 |
| staff/team.inc.php | 1 | 1 | 1 | 0 | — |
| staff/settings-tickets.inc.php | 2 | 7 | 3 | 0 | BS-032.6, EC-032.3, EC-032.4 |
| staff/department.inc.php | 7 | 10 | 8 (FS-030.x + FS-040.12 + FS-031.8) | 0 | — |
| staff/staffmembers.inc.php | 3 | 5 | 5 | 0 | — |
| staff/ticket-open.inc.php | 4 | 5 | 5 (FS-021.20 + FS-021.6) | 0 | — |
| staff/helptopic.inc.php | 4 | 4 | 4 | 0 | — |
| staff/category.inc.php | 3 | 4 | 4 (FS-032.14/15 + FS-050.8) | 0 | — |
| staff/faq-category.inc.php | 2 | 2 | 2 (FS-050.15 + FS-050.8) | 0 | — |
| staff/pages.inc.php | 3 | 3 | 3 | 0 | — |
| staff/faq-categories.inc.php | 3 | 3 | 3 | 0 | — |
| staff/footer.inc.php | 2 | 2 | 2 (FS-090.14 + FS-043.11) | 0 | — |
| staff/settings-autoresp.inc.php | 1 | 3 | 1 | 0 | BS-032.4, KL-032.8 |
| staff/settings-emails.inc.php | 1 | 3 | 1 | 0 | BS-032.7, BS-032.8 |
| staff/settings-system.inc.php | 3 | 6 | 4 | 0 | EC-032.2 |
| staff/settings-pages.inc.php | 3 | 8 | 4 | 0 | KL-032.12, EC-032.12, EC-032.13, EC-032.14 |
| staff/slaplan.inc.php | 2 | 5 | 3 | 0 | BS-032.9, BS-032.11 |
| staff/staff.inc.php | 4 | 6 | 6 (FS-031.x) | 0 | BS-031-013 |
| staff/categories.inc.php | 5 | 7 | 7 (FS-032.x + FS-050.14) | 0 | — |
| staff/cannedresponses.inc.php | 4 | 5 | 5 (FS-022.x) | 0 | BS-022.5, BS-022.6 |
| staff/cannedresponse.inc.php | 6 | 8 | 8 (FS-022.x) | 0 | BS-022.1, BS-022.4, BS-022.6 |
| staff/attachment.inc.php | 4 | 5 | 5 (FS-032.4 + FS-022.13) | 0 | — |
| staff/syslogs.inc.php | 2 | 8 | 8 (FS-033.x) | 0 | — |
| staff/teams.inc.php | 2 | 3 | 3 (FS-030.8/12) | 0 | — |
| staff/templates.inc.php | 2 | 3 | 3 (FS-040.5/9) | 0 | — |
| staff/template.inc.php | 1 | 2 | 2 (FS-040.8/6) | 0 | — |
| staff/tpl.inc.php | 1 | 2 | 2 (FS-040.7/11) | 0 | — |
| staff/ticket-edit.inc.php | 1 | 1 | 1 (FS-021.15) | 0 | — |
| staff/tickets.inc.php | 10 | 19 FS.N + 12 BS | 19 (FS-020.x) | 0 | BS-020.1/2/3/4/5/6/7/8/9/10/12/16 (12) |
| staff/ticket-view.inc.php | 13 | 27 FS.N + 6 BS | 27 (FS-021.x) | 0 | BS-021.3, BS-021.8(x2), BS-021.13, BS-021.16, BS-021.18 |
| client/faq-category.inc.php | 1 | 1 | 1 (FS-050.6) | 0 | — |
| client/footer.inc.php | 1 | 1 | 1 (FS-010.10) | 0 | — |
| client/tickets.inc.php | 2 | 3 | 2 (FS-010.8) | 0 | KL-010.6 |
| client/faq.inc.php | 1 | 1 | 1 (FS-050.7) | 0 | — |
| client/view.inc.php | 4 | 7 | 4 (FS-010.7) | 0 | BS-010.5, BS-010.7, BS-010.8 |
| client/login.inc.php | 1 | 1 | 1 (FS-010.2) | 0 | — |
| client/header.inc.php | 3 | 7 | 7 (FS-010.10 + FS-090.4/11/12) | 0 | — |
| client/open.inc.php | 3 | 8 | 8 (FS-011.x) | 0 | — |
| client/knowledgebase.inc.php | 2 | 3 | 3 (FS-050.4/5) | 0 | — |

## Resolution highlights

- **No spec-level fallbacks used.** Every bracket tag resolved to one or more
  precise `FS-XXX.N` requirement ids. The original Phase-3 tags already embedded
  the `.N` ids in their parenthetical notes, so resolution was deterministic;
  ambiguous spec-level tags (e.g. `team.inc.php` "FS-030 team CRUD") were resolved
  to the exact add/edit-form requirement (FS-030.9) by reading the section.
- **BS tagged first-class** with the spec's own id spelling (e.g. `BS-031-013`
  uses the dash form verbatim; `BS-032.5`, `BS-020.x`, `BS-021.x` use dotted form).
- **EC-/KL- promoted to first-class** where the note documented them: KL-032.x
  (alerts/sla/logo typos & always-on flags), KL-010.6 (My-Tickets latent defects),
  EC-032.x (date-preview, attachment bounds, logo-delete edges, confirm-intercept).
- **Dual-spec tags** split into one `@implements` line per spec so each surfaces
  independently (e.g. department.inc.php FS-030.4 + FS-040.12 / FS-031.8;
  client/header.inc.php FS-090.4 + FS-090.12 + FS-010.10; attachment.inc.php
  FS-032.4 + FS-022.13).
- **Comment mechanism preserved per site**: `// ` line comments stayed `// `,
  `/* */` blocks stayed `/* */` (multi-line inside the existing open PHP block —
  no new `<?php ?>` pairs introduced), `<!-- -->` HTML comments stayed `<!-- -->`,
  and standalone `<?php /* */ ?>` tag lines that became multi-id were expanded to
  a single `<?php` / `?>` pair wrapping multiple `/* */` lines (the whole tag line
  did not exist at HEAD, so this is comment-only).

## Ambiguous cases

**0 genuinely ambiguous cases.** Two judgement calls worth recording (both resolved
confidently, not fallbacks):
1. `client/faq-category.inc.php` & `client/faq.inc.php`: original notes cited the
   FS-050.3 routing dispatcher, but the files ARE the view partials, so they were
   tagged to the exact view requirements FS-050.6 / FS-050.7 (with the dispatcher
   relationship described in prose).
2. `department.inc.php` line 157 "autoresp outgoing email": mapped to FS-040.12
   (Outbound Mail Composition & From-Address Selection) as the nearest requirement
   for the per-dept From-address field, alongside the FS-030.4 dept-form line.
