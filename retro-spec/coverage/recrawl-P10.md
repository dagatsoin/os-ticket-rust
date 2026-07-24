# Re-crawl P10 — Independent Coverage Verification (Adversarial)

Partition **P10**: `include/staff/*.inc.php` chunk B (settings→tpl) + `include/client/*` + client KB pages.
Run: 2026-06-10 (verification round). Method: fresh-eyes walk of EVERY section in all 30 files;
did NOT trust prior `@implements` tags or the prior `P10-uncovered.md` "zero uncovered" claim.
Cited-id spot-check: ~50 distinct FS-XXX.N ids verified present across FS-010/011/020/021/030/031/032/033/040/050/090 (well over the 30% bar) — **all sampled ids resolve correctly**.

## Summary counts

| Metric | Count |
|--------|------:|
| Files in partition | 30 |
| Files walked (every section) | 30 |
| Sections COVERED (tag present + cited id verified) | ~70 |
| UNCOVERED findings | 1 |
| MIS-TAGGED findings | 1 (spec factual error) + 2 minor (incomplete cite, borderline) |
| TRIVIA (chrome/markup, correctly untagged or tagged once) | n/a (no new) |

Not DRY: 2 substantive findings recorded below (1 UNCOVERED behavior, 1 MIS-TAGGED spec contradiction), plus 2 minor incomplete-cite nits.

## Findings

| # | File:line | Unit | Behavior | Why it's a finding | Suggested spec | New id? |
|---|-----------|------|----------|--------------------|----------------|---------|
| 1 | `include/staff/templates.inc.php:67` (+ sort map line 10, `$inuse_sort` line 28/67) | Email-template-set list "In-Use" column header | The **In-Use** column header renders `<a href="...&sort=inuse">`, but `inuse` is **not** in `$sortOptions` (`name/status/created/updated` only) and `$inuse_sort` is never assigned. Clicking it falls back to the default `name` sort — a dead/no-op sort link. | **UNCOVERED.** FS-040.5 line 87 explicitly states only "Name, Status, Created, and Updated are sortable" and lists In-Use as a display-only column — it never documents that the In-Use header *emits a sort link at all*, nor that the link is inert. The two analogous dead-sort-link quirks in this exact partition ARE documented (teams `sort=updated` → KL-030-12; SLA `sort=created`/`updated` → KL-032.10), so the omission for templates In-Use is an inconsistent gap, not an intentional silence. The current `@implements FS-040.5` tag on this block is therefore incomplete (covers the column, not the dead link). | FS-040 | **YES** — add a KL (e.g. KL-040.x: "Template-set In-Use column header is a dead sort link — emits `sort=inuse`, which is not a recognized sort key, so the click silently reverts to default Name sort." Mirror KL-030-12 wording.) |
| 2 | `specs/FS-040-...md:499-501` (KL-040.5) vs actual tree | KL-040.5 "Staff View Partials Absent in This Snapshot" | KL-040.5 asserts: *"The entire `include/staff/` view-partial tree is absent from this 1.7 source snapshot... `templates.inc.php`, `template.inc.php`, `tpl.inc.php` ... are all missing"* and that all FS-040 UI acceptance criteria are *"inferred ... not verifiable against view source."* | **MIS-TAGGED / spec factual error.** These three files **exist and were read in full** in this snapshot (and carry live `@implements FS-040.5/.6/.7/.8/.9/.11` tags). KL-040.5 is therefore false for this snapshot: the FS-040 template-management UI partials ARE present and verifiable. This both invalidates the KL and means the FS-040 UI rules should be marked *verified-against-source*, not *inferred* — and it is why finding #1's dead-sort-link was never caught (the spec authored FS-040 believing the views were unavailable). | FS-040 | Correct/retire KL-040.5 (at minimum scope it to `emails.inc.php`/`email.inc.php` if those are genuinely absent; remove the false `templates.inc.php`/`template.inc.php`/`tpl.inc.php` claim and the blanket "inferred" caveat). No new id; edit existing KL. |

### Minor (incomplete-cite nits — behavior IS documented, tag just doesn't cite the KL)

| # | File:line | Unit | Note |
|---|-----------|------|------|
| 3 | `include/staff/teams.inc.php:66` | "Last Updated" header links `sort=updated`; not in sort map; `$updated_sort` unassigned | Behavior **is** documented (FS-030 **KL-030-12** — verified present). The block is tagged `@implements FS-030.8` but does not also cite KL-030-12. Borderline; FS-030.8 already says the map is `name/status/members/lead/created`. Suggest adding the KL-030-12 cite for parity with how slaplans cites KL-032.10. No new id. |
| 4 | `include/client/faq.inc.php:33` | "Last updated" reads `$category->getUpdateDate()` (parent category ts, not article ts) | Behavior **is** documented (FS-050 **KL-050.2** — verified present). Block tagged `@implements FS-050.7` without citing KL-050.2. Suggest adding the cite. No new id. |

## Verified-correct, high-risk sections (adversarially re-checked, no finding)

These were the largest / most logic-dense surfaces; all map cleanly and the cited ids verify:
- `staff/tickets.inc.php` (612 ln): overloaded `status` queue resolution, dept+assignment visibility predicate, 3-shape keyword search (numeric/email/deep LIKE), sticky per-queue sort, per-queue rightmost column, mass-action button matrix, CSV export token — fully owned by FS-020.1–.10 + BS-020.1/2/3/4/5/6/7/8/9/10/12/16. COVERED.
- `staff/ticket-view.inc.php` (870 ln): banner computation, action-bar permission/state gating, More-menu, thread render (inline vs separate notes), reply/note/transfer/assign forms, print dialog, close/reopen + process-action confirms, auto-lock seed — FS-021.2/.3/.4/.6/.7/.8/.9/.10/.11/.12/.13/.14/.17/.18/.19/.22/.23a + BS-021.x. COVERED.
- `staff/ticket-open.inc.php`, `staff/ticket-edit.inc.php`: every conditional branch (notifyONNewStaffTicket / canAssignTickets / canPostReply / allowAttachments / canCloseTickets / signature) tagged FS-021.20/.6/.15. COVERED.
- `staff/settings-system.inc.php` / `settings-tickets.inc.php` / `settings-alerts.inc.php` / `settings-pages.inc.php`: per-field-group tags; documented quirks KL-032.3 (send_sys_errors checked+disabled), KL-032.9 (`alert_alert_active` typo), KL-032.12/EC-032.12/13/14 (logo upload/delete) all tagged in place. COVERED.
- `staff/staff.inc.php` / `staffmembers.inc.php` / `team.inc.php` / `teams.inc.php` / `slaplan.inc.php` / `slaplans.inc.php` / `syslogs.inc.php`: list/filter/sort/paginate + add/edit forms + mass actions all tagged; KL-032.10 (SLA dead sort) tagged in place. COVERED.
- `client/open.inc.php` (FS-011.2–.7), `client/view.inc.php` (FS-010.7 + BS-010.5/.7/.8), `client/tickets.inc.php` (FS-010.8 + KL-010.6 four latent defects), `client/header.inc.php`/`footer.inc.php`/`login.inc.php` (FS-010.10 + FS-090.4/.11/.12), `client/knowledgebase.inc.php`/`faq.inc.php`/`faq-category.inc.php` (FS-050.4/.5/.6/.7). COVERED.

## Bottom line

Prior `P10-uncovered.md` claimed "zero genuinely-uncovered sections." Adversarial re-crawl finds that
claim **almost** holds for the staff/client view layer itself, BUT surfaces:
1. one **genuinely undocumented behavior** (templates In-Use dead sort link — finding #1), masked because
2. the owning spec carries a **false "views absent" KL** (KL-040.5 — finding #2) that wrongly marks the
   entire FS-040 template UI as unverifiable/inferred. Fixing #2 is the higher-leverage action; #1 is the
   concrete behavioral gap it was hiding.

## Round 2 closure (2026-06-10)

- **CLOSED nit #3** — `include/staff/teams.inc.php`:66 "Last Updated" dead sort link.
  Added a `@implements KL-030-12` cite alongside the existing FS-030.8 block tag.
- **CLOSED nit #4** — `include/client/faq.inc.php`:33 "Last updated" reads the parent
  category timestamp. Added a `@implements KL-050.2` cite alongside the existing FS-050.7
  block tag.
- Findings #1 (templates In-Use dead sort link / new KL) and #2 (KL-040.5 spec factual
  error) require spec edits and are OUT OF SCOPE for this tag-precision pass — left open.

### Round 2 closure (closure agent B) — findings #1 & #2

Findings **#1 and #2 CLOSED** (nits #3/#4 already closed above by another agent).

- **Finding #2 (KL-040.5 spec factual error) — CLOSED.** Re-verified against the filesystem
  (read-only) before editing. NOTE: the re-crawl body above (and the closure brief) claimed
  "only `tpl.inc.php` is genuinely absent" — that is **also wrong**. `include/staff/tpl.inc.php`
  EXISTS and was read in full (per-message editor, `@implements FS-040.7/.11`), as do
  `templates.inc.php` (`FS-040.5/.9`) and `template.inc.php` (`FS-040.8/.6`), plus
  `emails.inc.php`/`email.inc.php`/`header.inc.php`/`footer.inc.php`. The entire "staff
  view-partial tree absent" premise of KL-040.5 is false for this snapshot — **nothing** in
  that tree is missing. KL-040.5 rewritten in place (id kept) from "Staff View Partials Absent
  in This Snapshot (UI Behavior Inferred)" → "Staff View Partials Present and Verified Against
  Source"; the blanket "inferred / not verifiable against view source" caveat removed and the
  listed UI criteria re-marked verified-against-source.
- **Finding #1 (templates.inc.php:67 dead In-Use sort link) — CLOSED.** New KL **KL-040.11**
  added to FS-040 ("Template-Set 'In-Use' Column Header Is a Dead Sort Link"), wording mirrored
  on KL-030-12. Confirmed at source: `$sortOptions` (line 10) = `name/status/created/updated`
  only; the `$$x` assignment (lines 27–28) only sets the active `*_sort`; `$inuse_sort` never
  assigned; header (line 67) emits `&sort=inuse` → falls through to default `name` sort
  (line 12). The `@implements` block above the list table (line 53) extended to also cite
  KL-040.11.
