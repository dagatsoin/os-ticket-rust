# Phase-2 GAP-CLOSE — Consolidated Report (osTicket-1.7)

Date: 2026-06-10. Method: per-spec `spec-writer` re-read of the assigned source slice with
fresh eyes, gaps found vs. implementation, fixed in the spec (report-spec-only — no source
file modified). One detail file per spec under `work/gaps/FS-*-gaps.md`.

## Totals

- **Specs processed:** 21 / 21
- **Gaps found:** **249**
- **Gaps fixed:** **249** (0 open)
- **By type:** MISSING **120** · IMPRECISE **84** · INCORRECT **45**

The orchestrator-collected figure (249 found / 249 fixed) is **verified accurate** — the
sum of the per-file gap rows matches exactly. One internal inconsistency was noted and is
called out below (FS-003) but it does not change the 249 total.

> Counting note: the by-type split is taken from each file's own summary. FS-003's prose
> summary says "MISSING 11 / IMPRECISE 6 / INCORRECT 3" (=20) while its table has 19 rows
> (row G-19 is typed "Info", a KL-only addition). This report counts FS-003 at its true
> **19 rows** (INCORRECT 3 / IMPRECISE 6 / MISSING 10, folding the "Info" row into MISSING),
> so the column totals reconcile to 249.

## Per-spec gap counts

| Spec | Domain | Gaps | M / INC / IMP | Most significant finding |
|------|--------|------|---------------|--------------------------|
| FS-001 | App bootstrap & request lifecycle | 9 | 3 / 1 / 5 | Six request/env utility groups (`is_https`, `is_cli`, `get_path_info`, safe accessors, `isFileTypeAllowed`, DB signature) were claimed but never enumerated; latent `getError`/`setError` key mismatch (`err` vs `error`). |
| FS-002 | Staff auth, sessions & access | 17 | 8 / 6 / 3 | Logout link-token guard is a no-op (missing `exit` after redirect) — the forced-logout-CSRF defence does not exist; password-reset time window never enforced (dead boolean); DB-backed session handler bypassed on installed systems. |
| FS-003 | Crypto, validation & formatting | 19 | 10 / 3 / 6 | `randNumber` silently ignores `start`/`end` unless `len==0`; `Timezone::getName/getDesc` read the wrong member slot (name permanently unreadable). |
| FS-010 | Public client portal (lookup/view/login) | 13 | 6 / 3 / 4 | Client login lockout is **dead code** — an error is pre-set before the lockout guard, so `laststrike`/admin-alert/Access-Denied never fire (staff path is unaffected). |
| FS-011 | Public ticket submission (web form) | 9 | 3 / 3 / 3 | Over-limit admin alert is NOT unconditional — both user e-mail and admin alert are gated behind `overlimit_notice_active` plus an `(autorespond && non-staff)` flag; only the warning log is unconditional. |
| FS-020 | Staff ticket queue, dashboard & search | 8 | 2 / 2 / 4 | The unassigned (`staff_id=0`) constraint in quick-stats is appended only to the `open` UNION branch, not answered/overdue/assigned/closed; show-assigned override needs admin-or-manager. |
| FS-021 | Staff ticket view & workflow | 18 | 12 / 2 / 4 | Note-form `state=unassigned` is inert (switch tests misspelled `unassined`); preview lock banner keys off the wrong owner so it never warns about another agent's lock; staff-created tickets bypass the max-open ceiling. |
| FS-022 | Canned responses & ticket attachments | 13 | 6 / 4 / 3 | `isFileTypeAllowed` is **default-deny** — an empty allow-list rejects all uploads (only `.*` allows all); `cannedResp` JSON field names were wrong (`ticket` field holds the title); `kb/file.php` has no session gate. |
| FS-030 | Admin — departments, teams & help topics | 8 | 4 / 2 / 2 | No-change save is behaviorally inconsistent: dept/team require nonzero affected-rows (spurious error on no-op re-save) while help-topic always reports success. |
| FS-031 | Admin — staff, groups & directory | 12 | 9 / 1 / 2 | Undocumented periodic password-reset aging auto-forces a password change at login for non-admins; own-profile password-change flow was largely absent; Forced-Change checkbox is presence-driven with misleading value `0`. |
| FS-032 | Admin — system settings, SLA, priorities & FAQ categories | 11 | 4 / 2 / 5 | Two settings partials (`alerts`, `autoresp`) skip the in-partial admin re-check; `send_sys_errors` "always on" is actually stored as `0` and is dead; reopened tickets restart the SLA grace clock from reopen, not creation. |
| FS-033 | Admin — logs, pages & content | 9 | 5 / 1 / 3 | **CRITICAL**: the spec flatly denied a fully-implemented Site Pages CMS (`scp/pages.php`, `class.page.php`, `ost_page`) backing landing/offline/thank-you/public-slug pages — roughly half the domain was missing. |
| FS-040 | Email accounts, templates & outbound mail | 16 | 7 / 3 / 6 | Packaged default template is a structured `.yaml` with mandatory `subject`/`body` keys (not a first-line/remainder text file); the entire `include/staff/` view tree is absent so all UI ACs are inferred; the alert-email account is also undeletable. |
| FS-041 | Inbound email pipeline — fetch, pipe & parse | 11 | 4 / 2 / 5 | Empty-pipe "retry/defer" outcome actually emits permanent data-error exit 65 (an MTA would mishandle it); seen-flag suppression is polled-channel-only; HTTP/pipe `fixup` substitutes empty body with subject and empty target with the system-default account. |
| FS-042 | Ticket filters & banlist (inbound routing) | 16 | 11 / 1 / 4 | `isBanned` `equal` operator is mis-coded and behaves like `contains` in the pre-screen; filter actions apply twice per ticket; banning an owner blocks staff replies at submit time (not merely a warning). |
| FS-043 | External API & cron scheduler | 15 | 8 / 3 / 4 | Cron route is a nested `/tasks/` → `cron` sub-dispatcher (not a flat matcher); email-format reply-detection branch + the local mail-pipe HTTP→MTA exit-code mapping belonged in this spec and were missing. |
| FS-050 | Knowledge base & FAQ | 10 | 7 / 1 / 2 | `Category::lookup` returns a hollow object for any numeric id (no `getId()==id` re-check), so the "Unknown or invalid FAQ category" error path is dead; several FAQ-save DB-failure messages were undocumented. |
| FS-060 | Installer & setup wizard | 11 | 6 / 1 / 4 | Stream registration uses `streams.cfg` + `.sig`-file contents compared (case-insensitively) against the SQL md5, with dual existence requirement — the mechanism, the `?s=ns` GET transition, and a mislocated `$_SESSION['info']` side-effect were all off. |
| FS-061 | Upgrader & database migration streams | 10 | 4 / 1 / 5 | `check_mysql()` is purely `extension_loaded('mysql')` — there is **no** MySQL version gate (the "v4.4" is a static label); the AJAX upgrade endpoint is POST-only and CSRF-protected; most migration tasks are single-shot, not resumable. |
| FS-090 | Shared UI, navigation & data export | 10 | 5 / 1 / 4 | An undocumented second pagination bounds branch (`start -= start % limit`); empty-result export emits the **full** header-map column set (wider than populated exports); client nav key-class is emitted for every link, not just the active one. |
| FS-091 | Reference data, enums & data model | 4 | 1 / 1 / 2 | The schema has exactly **34** `CREATE TABLE`s, not 35 — the spec padded to 35 with a fabricated entity that double-counted the `email_template_group`/`email_template` pair. |
| **Total** | | **249** | **120 / 45 / 84** | |

## Per-spec detail pointers

Full gap tables (every row: ID · type · severity · what the code does · what the spec said ·
fix applied) live in the per-spec detail files. Summaries below; see the file for the table.

- **FS-001** — `work/gaps/FS-001-gaps.md` — 9 gaps. +1 FR (FS-001.16), +2 BS (BS-015/016), +4 EC, +2 KL; refinements to FS-001.5/.7/.10–.13.
- **FS-002** — `work/gaps/FS-002-gaps.md` — 17 gaps (5 HIGH). +2 FR (FS-002.18/.19), +3 BS, +6 EC, +4 KL; corrected BS-002-07/14/21/22.
- **FS-003** — `work/gaps/FS-003-gaps.md` — 19 gaps (table rows; prose split sums to 20 — see counting note). +3 BS, +5 EC, +5 KL; all corrections folded into FS-003.1–.25.
- **FS-010** — `work/gaps/FS-010-gaps.md` — 13 gaps. +2 BS (BS-010.12/.13), +2 KL; KL-010.6 broadened to 4 sub-defects; edits across FS-010.2/.5/.7/.8/.10/.11.
- **FS-011** — `work/gaps/FS-011-gaps.md` — 9 gaps. +4 BS (BS-011.9–.12), +2 EC, +2 KL; folded into FS-011.1/.3/.7/.9/.12/.13.
- **FS-020** — `work/gaps/FS-020-gaps.md` — 8 gaps. +3 BS (BS-020.16–.18), +2 EC; FS-020.5/.8/.11 corrected in place.
- **FS-021** — `work/gaps/FS-021-gaps.md` — 18 gaps (4 HIGH). +1 FR (FS-021.23a), +5 BS, +6 EC, +2 KL; many in-place clarifications.
- **FS-022** — `work/gaps/FS-022-gaps.md` — 13 gaps. +1 FR (FS-022.15), +2 BS, +5 EC; KL-022.3 corrected in place.
- **FS-030** — `work/gaps/FS-030-gaps.md` — 8 gaps. +0 FR/BS (amended in place), +2 EC, +3 KL.
- **FS-031** — `work/gaps/FS-031-gaps.md` — 12 gaps. +2 FR (FS-031.12/.13), +2 BS, +4 EC, +3 KL.
- **FS-032** — `work/gaps/FS-032-gaps.md` — 11 gaps. +0 FR/BS (corrected in place), +3 EC, +5 KL.
- **FS-033** — `work/gaps/FS-033-gaps.md` — 9 gaps (1 CRITICAL Site Pages). +6 FR (FS-033.11–.16), +3 BS, +5 EC, +2 KL, +1 KL inverted; new data store + 2 flows + 6 dependency rows.
- **FS-040** — `work/gaps/FS-040-gaps.md` — 16 gaps. +5 BS (BS-040.25–.29), +3 EC, +3 KL; rewrites to FS-040.4/.10–.12 + multiple BS.
- **FS-041** — `work/gaps/FS-041-gaps.md` — 11 gaps. +1 FR (FS-041.5.3), +4 BS, +3 EC; corrections to FS-041.1/.2/.5/.6/.7 + BS-041.10/.13/.16.
- **FS-042** — `work/gaps/FS-042-gaps.md` — 16 gaps. +2 FR (FS-042.13/.14), +7 BS (BS-042-20..26), +5 EC, +8 KL.
- **FS-043** — `work/gaps/FS-043-gaps.md` — 15 gaps. +2 FR (FS-043.14/.15), +2 BS, +3 EC, +3 KL; precision edits to FS-043.2/.3/.4/.6.
- **FS-050** — `work/gaps/FS-050-gaps.md` — 10 gaps. +0 FR/BS (amended), +4 EC, +4 KL.
- **FS-060** — `work/gaps/FS-060-gaps.md` — 11 gaps. +0 FR/BS (expanded in place), +3 EC, +3 KL.
- **FS-061** — `work/gaps/FS-061-gaps.md` — 10 gaps. +2 FR (FS-061.16/.17), +5 BS, +5 EC, +3 KL.
- **FS-090** — `work/gaps/FS-090-gaps.md` — 10 gaps. +5 FR (FS-090.28–.32), +2 BS, +4 EC; corrections to FS-090.4/.11/.23/.26.
- **FS-091** — `work/gaps/FS-091-gaps.md` — 4 gaps. +1 FR (FS-091.15), +2 KL; corrected the 35→34 table-count error.

## Cross-cutting themes

1. **Dead / inert guards** recurred across specs: client-login lockout (FS-010), logout
   link-token (FS-002), password-reset time window (FS-002), `equal` ban operator (FS-042),
   note-form unassign typo (FS-021), `send_sys_errors` (FS-032), "FAQ category not found"
   (FS-050). Several documented "protections" simply do not execute.
2. **Channel asymmetries** in the email pipeline (FS-041/FS-042/FS-043): polled-fetch vs.
   HTTP/pipe diverge on seen-flag suppression, emailId fallback, ban timing, and bounce-drop.
3. **Default-deny vs default-allow** ambiguities resolved (FS-022 file-type gate).
4. **Whole-feature omission** corrected once (FS-033 Site Pages CMS) and a structural
   count error (FS-091, 34 not 35 tables).

## Constraint compliance

Report-spec-only honored across all 21 runs: only `specs/FS-*.md` and the `work/gaps/*.md`
detail files were written. No PHP/source file was modified.
