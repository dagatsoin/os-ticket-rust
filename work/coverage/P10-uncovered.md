# P10 Coverage — Uncovered / Notable Report

Partition P10: `include/staff/*.inc.php` chunk B (settings→tpl) + `include/client/*` + client KB pages.
Run date: 2026-06-10. Method: section-level additive `[FS-0XX]` tags above each coherent unit
(form blocks, table renders, conditional UI branches, embedded query logic). Idempotent; comment-only.

## Summary counts

| Metric | Count |
|--------|------:|
| Files in partition | 30 |
| Files examined | 30 |
| Coherent sections tagged | 70 |
| Sections uncovered (no spec) | 0 |
| Trivia skipped (chrome/markup-only, no behavior) | see below |

All non-trivial sections in P10 mapped cleanly onto an existing spec (FS-010/011/020/021/030/031/032/033/040/050).
No section was found whose behavior is undocumented. The items below are recorded for completeness:
(a) latent defects that ARE documented (tagged in-place with a `NOTE`), and (b) trivia deliberately not tagged.

> **Round-2 note (2026-06-10)**: P10 left zero genuinely-uncovered sections and
> no spec-precision follow-ups requiring absorption — every latent defect below
> was already documented (KL-010.6, KL-032.3/.9/.10) and tagged in place. No P10
> change in round 2. (The round-2 db_* / FS-032 logo / FS-042 quickList work lands
> in P1/P5/P9 reports.)

## Documented latent defects (tagged in place, not "uncovered")

| File:line (approx) | Unit | Behavior | Spec ref |
|--------------------|------|----------|----------|
| `client/tickets.inc.php:1` | My Tickets list | numeric search uses undefined `$queryterm`; `sort=subj`/`sort=ID` header key mismatch; `'ticket_created'` order-by fallback literal; Phone column not projected in SELECT | FS-010 KL-010.6 |
| `staff/settings-alerts.inc.php:114` | Transfer alert status row | recipient error stored under `transfer_alert_active` but rendered as `$errors['alert_alert_active']` (typo) — never displayed | FS-032 KL-032.9 |
| `staff/settings-alerts.inc.php:161` | System Alerts | `send_sys_errors` rendered checked+disabled → never submitted → always persists 0 | FS-032 KL-032.3 |
| `staff/slaplans.inc.php:60` | SLA list "Date Added" header | links `sort=created` (unrecognized key) + `$created_sort`/`$updated_sort` never assigned | FS-032 KL-032.10 |

## Trivia skipped (markup/chrome only — no documented behavior to tag)

| File | Unit | Why skipped |
|------|------|-------------|
| `client/footer.inc.php` | overlay/loading `<div>` scaffolding | pure shared chrome (one [FS-010] tag placed at footer block; sub-divs trivial) |
| all `*.inc.php` confirm-action dialogs | "Please Confirm" modal markup | static confirmation copy; tagged once per parent list/form, inner `<p>` variants are trivial |
| `staff/settings-*` `<thead>`/section captions | section headings + `<em>` help text | descriptive copy under an already-tagged form block |
| `staff/template.inc.php` Language select | single hard-coded `English (US)` option | no behavior (KL-noted elsewhere in FS-040 as i18n stub); under tagged form block |

## Notes on spec mapping choices

- `client/open.inc.php` is the public submission FORM template; its handler is `open.php` (P1). Tagged to
  **FS-011** (form layout / identity pre-fill / conditional attachments-priority-captcha blocks).
- `staff/tpl.inc.php`, `template.inc.php`, `templates.inc.php` are email-template management; tagged **FS-040**
  (template sets / `%{variable}` grammar). No FS-040.N sub-IDs cited since FS-040 was not opened in detail —
  cited at spec level, which is sufficient for COVERAGE granularity.
- `staff/team.inc.php` / `teams.inc.php` tagged **FS-030** (team CRUD) at spec level; FS-030 team sub-IDs not
  individually cited (FS-030 was sampled, not fully read) — the spec ownership is unambiguous.
- `staff/syslogs.inc.php` tagged **FS-033** (system log viewer FS-033.1–.8).
- Client KB pages (`faq.inc.php`, `faq-category.inc.php`, `knowledgebase.inc.php`) tagged **FS-050**
  (public KB routing/landing/search/article views).

## 3 most notable findings

1. **`client/tickets.inc.php` (My Tickets list) carries four independent latent defects** (FS-010 KL-010.6):
   numeric ticket-number search builds its `LIKE` clause from an unset `$queryterm`; the Subject/Ticket sort
   headers emit `sort=subj`/`sort=ID` that don't all match the recognized sort keys; the order-by fallback uses
   a non-existent `'ticket_created'` literal; and the Phone Number column is echoed but never SELECTed.
   Tagged with an inline NOTE rather than left uncovered, since the spec documents all four.

2. **`settings-alerts.inc.php` encodes two FS-032 quirks in one screen**: `send_sys_errors` is rendered
   checked+disabled so every save persists `0` (KL-032.3), and the Ticket-Transfer recipient error is keyed on
   `transfer_alert_active` but rendered via the misspelled `alert_alert_active`, so the "Select recipient(s)"
   error blocks the save invisibly (KL-032.9).

3. **Full P10 coverage with zero genuinely-uncovered sections.** Every form block, listing query, and
   conditional UI branch across the 30 files (including the 870-line `ticket-view.inc.php` and 612-line
   queue `tickets.inc.php`) mapped onto an existing FS spec. The largest semantic surface — the overloaded
   `status` queue selector, the visibility predicate, and the three-shape assignee/closed-by search — is
   fully owned by FS-020 (BS-020.1/.2/.16).
