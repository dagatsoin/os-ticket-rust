# P9 Coverage Report — `include/staff/*.inc.php` chunk A (apikey→pwreset) + client/*

Partition: **P9** (35 files). Phase-3 COVERAGE tagging run, osTicket-1.7.
Prepared: 2026-06-10.

These are staff-panel **view templates** (form blocks, list tables, conditional UI). Tagging
granularity is section-level (one tag per coherent block — gate, list query, form, mass-action
bar, dialog), not per-echo. All tags are ADDITIVE comment lines (`<?php /* [FS-0XX] */ ?>`,
`// [FS-0XX]`, or `<!-- -->`); no existing source lines were modified, moved, or deleted.

## Per-file disposition

| File | Owner FS | Notes |
|------|----------|-------|
| apikey.inc.php | FS-043 | API key add/edit form (FS-043.1/.2, BS-430/431) |
| apikeys.inc.php | FS-043 | API keys list + bulk enable/disable/delete (FS-043.2) |
| attachment.inc.php | FS-032 / FS-022 | Admin Attachments **settings** tab (config keys); FS-022.13 type/size rules |
| banlist.inc.php | FS-042 | SYSTEM BAN LIST view: search, list, bulk actions |
| banrule.inc.php | FS-042 | Single ban-rule add/edit form |
| cannedresponse.inc.php | FS-022 | Canned add/edit form (FS-022.2/.3/.7/.8; BS-022.1/.4/.6) |
| cannedresponses.inc.php | FS-022 | Canned list + bulk actions (FS-022.1/.6) |
| categories.inc.php | FS-032 / FS-050 | FAQ-category list + bulk public/internal/delete |
| category.inc.php | FS-032 / FS-050 | FAQ-category add/edit form |
| department.inc.php | FS-030 | Department form (info, routing, autoresp, group access, signature) |
| departments.inc.php | FS-030 | Departments list + bulk actions |
| directory.inc.php | FS-031 | Staff directory browse/search (read-only contact list) |
| email.inc.php | FS-040 / FS-041 | Email-account form (FS-040); inbound IMAP/POP fetch block (FS-041) |
| emails.inc.php | FS-040 | Email accounts list + bulk delete |
| faq-categories.inc.php | FS-050 | Staff FAQ landing: search form + category/results branches |
| faq-category.inc.php | FS-050 | Single FAQ-category view + manage bar |
| faq-view.inc.php | FS-050 | Single FAQ article view + manage-faq actions |
| faq.inc.php | FS-050 / FS-022 | FAQ add/edit form (FS-050); attachment block (FS-022.10) |
| filter.inc.php | FS-042 | Ticket-filter form: target, rules (25 cap), actions |
| filters.inc.php | FS-042 | Filters list (by exec order) + bulk actions |
| footer.inc.php | FS-090 / FS-043 | CP footer chrome; autocron 1×1 beacon (FS-043.11) |
| group.inc.php | FS-031 / FS-030 | Group form: permission flags + dept-access checkboxes |
| groups.inc.php | FS-031 | Groups list + bulk actions |
| header.inc.php | FS-090 / FS-001 | CP page chrome, message bars, nav assembly, realm toggle |
| helptopic.inc.php | FS-030 | Help-topic form (hierarchy + new-ticket routing) |
| helptopics.inc.php | FS-030 | Help topics list + bulk actions |
| index.php | — | **Trivia**: 1-line `header('Location: ../')` redirect; not tagged |
| login.header.php | FS-090 / FS-002 | Shared login/reset page chrome |
| login.tpl.php | FS-002 | Staff login form (do=scplogin) + forgot-password gate |
| page.inc.php | FS-033 | Site page add/edit form (landing/offline/thank-you/other) |
| pages.inc.php | FS-033 | Site pages list (+ in-use marker) + bulk actions |
| profile.inc.php | FS-031 / FS-002 | Staff self-service profile + preferences + password change |
| pwreset.login.php | FS-002 | Set-new-password form (reset token) |
| pwreset.php | FS-002 | Password-reset request form (do=sendmail) |
| pwreset.sent.php | FS-002 | Reset-email-sent confirmation page |

## Uncovered / weakly-covered units

No **non-trivial** section in this partition was left without a documenting spec. The
templates are CRUD/list/form views whose behaviors map cleanly onto existing FS specs
(FS-002/022/030/031/032/033/040/041/042/043/050/090). The items below are minor
observations rather than coverage gaps — each is reachable from an existing spec but is
worth recording for spec-precision follow-up.

| File:line | Unit | 1-line behavior | FS/BS guess | Suggested spec touch |
|-----------|------|-----------------|-------------|----------------------|
| attachment.inc.php:6 | `<form action="admin.php?t=attach">` config block | The **Attachments** settings tab is one of the admin system-settings tabs but is delivered by a standalone `staff/attachment.inc.php` rather than a `settings-*.inc.php`; FS-032 enumerates seven settings tabs and should explicitly note the attachments tab template path + its config-key set (`allow_attachments`, `allow_email_attachments`, `allow_online_attachments[_onlogin]`, `email_attachments`, `max_file_size`, `upload_dir`, `allowed_filetypes`). | FS-032 (BS) | FS-032: add attachments-tab template note + config-key list (cross-ref FS-022.13) — **ADDRESSED (R2)**: FS-032.1 now has an "Attachments settings tab (out-of-band)" bullet naming the `staff/attachment.inc.php` template + the full config-key set + FS-022.13 cross-ref. |
| directory.inc.php:14 | numeric search WHERE clause | Phone-search SQL has a **missing `OR`** (`phone_ext LIKE '%..%' staff.mobile LIKE ...`) — a latent syntax bug on the numeric-search branch; behavior (search by phone digits) is covered by FS-031 but the defect is undocumented. | FS-031 (KL) | FS-031: add KL noting the broken numeric-search OR clause — **ALREADY DOCUMENTED**: this is FS-031 **KL-031-002** (+ EC-031-012); no new id. |
| categories.inc.php:134-136 | stray `mark_overdue-confirm` dialog `<p>` | The FAQ-category list's confirm dialog carries a **copy-pasted "flag tickets as overdue"** confirmation paragraph that no action on this page can trigger (dead UI fragment from the ticket-list template). Harmless but undocumented quirk. | FS-032/FS-050 (KL) | FS-050 or FS-032: KL noting the orphaned overdue-confirm fragment — **FALSE POSITIVE (R2)**: re-inspection of `categories.inc.php` (confirm block lines ~125-149) found only `make_public-confirm`, `make_private-confirm`, and `delete-confirm` paragraphs — **no `mark_overdue-confirm`/overdue fragment exists** in this file. No spec change made; row retracted. |

## Summary counts

- **Files examined**: 35 (all of P9).
- **Files tagged**: 34.
- **Files trivia-skipped (no tag)**: 1 (`index.php` — bare redirect).
- **Coherent sections tagged**: ~95 (gates, list queries, forms, field-group blocks, mass-action bars, dialogs, nav blocks).
- **Non-trivial sections left uncovered (no owning spec)**: 0.
- **Minor spec-precision findings recorded**: 3 (attachments-tab path/config, directory numeric-search bug, orphaned overdue-confirm fragment).
