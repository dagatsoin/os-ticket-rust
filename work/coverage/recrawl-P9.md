# Recrawl P9 — `include/staff/*.inc.php` chunk A (apikey→pwreset) + client/*

Independent verification re-crawl (fresh eyes, adversarial). Partition **P9** (35 files).
Prepared: 2026-06-10. READ-ONLY on PHP + specs; this report is the only file written.

Method: walked every section in all 35 files; for each `@implements` tag, checked the
cited id actually exists in the owning spec (spot-checked ~35% of ids against spec section
headers — all resolved) AND that the section it sits on is the behavior it claims. Looked
for untagged form-field groups, conditional render branches, and embedded logic blocks.

## Summary counts

| Metric | Value |
|---|---|
| Files walked | 35 / 35 |
| Files clean (every section covered, ids verified) | 33 |
| Files with a finding | 2 (`attachment.inc.php`, `cannedresponses.inc.php`) |
| COVERED sections (verified) | ~95 |
| UNCOVERED (no owning spec) | 0 |
| MIS-TAGGED | 1 |
| TRIVIA / dead-copy under a tagged section | 1 |
| Cited spec ids that failed to resolve | 0 |

Verdict: effectively clean. Every non-trivial section maps to a real, correctly-named spec
id. The two findings below are precision issues, not coverage holes — both behaviors are
already documented somewhere in the spec set.

## Findings

| File:line | Unit | Behavior | Why it's flagged | Suggested spec | New id needed? |
|-----------|------|----------|------------------|----------------|----------------|
| `include/staff/attachment.inc.php:2,6,23` | Attachments settings tab gate + config-key block | Standalone admin **Attachments** settings screen (`form action="admin.php?t=attach"`) persisting `allow_attachments`, `allow_email_attachments`, `allow_online_attachments(_onlogin)`, `email_attachments`, `max_file_size`, `upload_dir`, `allowed_filetypes`. | **MIS-TAG.** Tagged `@implements FS-032.4` ("Ticket Settings & Options Tab Fields"), but FS-032.4 describes the *Tickets* tab. FS-032.4's Attachments subsection does **not** list `upload_dir` and **adds** `max_user_file_uploads` / `max_staff_file_uploads` — neither matches this 1.7 file (it has `upload_dir`; it has no per-user/staff upload caps). The precise owner is the **FS-032.1 "Attachments settings tab (out-of-band)" bullet** (specs/FS-032…md:67-76), whose config-key list matches this file verbatim. So the file is *covered* (by FS-032.1 + FS-022.13) but *mis-cited* to FS-032.4. | FS-032.1 (+ FS-022.13 cross-ref) — already documents this exact tab + key set incl. `upload_dir` | No — re-point tag from FS-032.4 → FS-032.1 |
| `include/staff/cannedresponses.inc.php:143-145` | `id="mark_overdue-confirm"` `<p>` inside the confirm-action dialog | Dead dialog fragment "Are you sure want to flag the selected tickets as **overdue**?" — copy-pasted from the ticket mass-action list; this page's only mass actions are Enable / Disable / Delete (no overdue action can ever surface it). | **TRIVIA / dead-copy** under the already-tagged confirm dialog (FS-022.6). Worth recording because the **prior P9 report retracted this exact item as a "false positive"** after inspecting the wrong file (`categories.inc.php`, which genuinely lacks it). The orphaned overdue fragment is real, but it lives in **`cannedresponses.inc.php`**, not `categories.inc.php`. Harmless; no behavior, no enforcement. | FS-022 (optional KL on the orphaned overdue-confirm fragment) — or leave as trivia | No |

## Notes / non-findings (checked, deliberately NOT flagged)

- `directory.inc.php:18` numeric-search SQL has a missing `OR` before `staff.mobile LIKE`
  (latent bug) — already documented as FS-031 **KL-031-002** per the prior report; confirmed
  still present, no new id.
- `pwreset.login.php` disposition label in the prior report says "Set-new-password form";
  the actual file is a username+token **re-identify/login** form (`do=newpasswd`). The source
  `@implements` tag text is correct ("re-identify & log in"); only the prose disposition table
  was loose. Not a source tagging defect.
- `department.inc.php:153` `group_membership` checkbox, `page.inc.php:70-80` conditional
  "Public URL" branch (type=='other'), `profile.inc.php:181-191` admin/manager-only
  `show_assigned_tickets`, `helptopic.inc.php:157` Thank-you Page select,
  `faq-categories.inc.php:59` search-vs-listing branch — all distinct conditional units, all
  fall correctly under their enclosing section tag (FS-030.4 / FS-033.13 / FS-031.10 /
  FS-030.14 / FS-050.14 respectively). Verified, covered.
- `header.inc.php` global vs per-page message bars are split correctly across FS-090.12 and
  FS-090.13; top-nav / sub-nav / auto-highlight split across FS-090.8 / .9 / .7. Verified.
- `index.php` — 2-line `header('Location: ../')` redirect; correctly carries no coverage tag.

All cited spec ids verified to exist with matching titles: FS-002.1/.3/.18, FS-022.1/.2/.3/.5/.6/.7/.8/.9/.10/.13,
FS-030.1/.3/.4/.7/.13/.14/.17, FS-031.6/.7/.8/.9/.10/.13, FS-032.1/.4/.13/.14/.15/.16,
FS-033.11/.12/.13/.15, FS-040.1/.2/.4/.12, FS-042.1/.2/.3/.4/.5/.6/.10/.11, FS-050.8/.9/.13/.14/.15/.16,
FS-090.7/.8/.9/.10/.12/.13/.14/.043; BS-022.x, BS-030-05, BS-031-012/.013/.020/.021/.030/.031/.034,
BS-042-09/.14, BS-430, BS-431.

## Round 2 closure (2026-06-10)

- **CLOSED** — `include/staff/attachment.inc.php`:2,6,23 MIS-TAG. All three `FS-032.4`
  (Tickets tab — wrong) tags re-pointed to **FS-032.1** ("Attachments settings tab
  (out-of-band)"), whose config-key list (incl. `upload_dir`, no per-user/staff caps)
  matches this 1.7 file verbatim. The FS-022.13 co-tags were left untouched.
- **CLOSED** — `include/staff/cannedresponses.inc.php`:143-145 orphaned `mark_overdue-confirm`
  dead-copy fragment. Added an `@implements FS-022.6` note line marking it a "dead
  copy-paste fragment, unreachable" (no new spec id). Both P9 findings closed.
