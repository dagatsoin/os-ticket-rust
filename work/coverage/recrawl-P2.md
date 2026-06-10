# Re-crawl Coverage Verification — P2 (`scp/*.php`)

Verification round (fresh-eyes, adversarial). Repo root: `/Users/warfog/dev/osTicket-1.7`
Prepared: 2026-06-10. READ-ONLY on all PHP and specs.

Method: walked every file in the P2 partition (37 files) top-to-bottom — every lookup
guard, every `if($_POST){ switch($_POST['do'|'a']) }` action branch, every nested
`process`/`mass_process` sub-switch, every render/page-selection dispatch, and every
error/redirect path. Each unit classified COVERED / UNCOVERED / MIS-TAGGED / TRIVIA.
Spot-checked id resolution on **>30%** of distinct tags by opening the cited spec
section and confirming it documents the behavior (verified: FS-001.10/.12/.16,
FS-002.1/.3/.7/.10/.11/.16, FS-020.1/.2/.10/.11, FS-021.1/.17/.20, FS-022.8/.10/.11,
FS-030.5/.6, BS-030-05/06, FS-032.15, FS-033.6, FS-040.4, BS-040.3, FS-042.11,
FS-043.3/.11, BS-435, FS-050.11/.12/.13 + KL-050.1, EC-042-1).

## Summary counts

| Metric | Count |
|--------|------:|
| Files in partition | 37 |
| Files processed | 37 (all) |
| Significant units walked | ~150 |
| COVERED (tag present + cited id confirmed) | ~149 |
| UNCOVERED (untagged non-trivial behavior) | 0 |
| MIS-TAGGED (wrong id cited) | 1 |
| TRIVIA / shell-glue (requires, setTabActive, header/footer includes) | ~90 |

**Verdict: NOT dry — 1 MIS-TAG finding.** No UNCOVERED behavior. The previous
P2-uncovered / retag-P2 reports' "weak/quirk" items were re-examined and are all
genuinely documented (see "Re-examined and confirmed COVERED" below), so they are
NOT re-raised.

## Findings (MIS-TAGGED / UNCOVERED)

| File:line | Unit | Behavior | Why it's wrong | Suggested spec | New id? |
|-----------|------|----------|----------------|----------------|---------|
| `scp/admin.inc.php`:31 (block 30–63) | Security/maintenance sysnotice battery | Sets `$sysnotice` via a first-match-wins battery: (a) legacy `settings.php` config name → rename advice (+ hard-die if running script is settings.php); (b) `setup/` dir still present → delete-setup advice; (c) config file writable AND group/world-write bit set (`0x0010`/`0x0002`) → `chmod 644` advice; (d) `register_globals` on → turn-off advice. | Tagged `@implements FS-001.12` (line 31). FS-001.12 is **"System Logging & Admin Alerting"** (the syslog write facility + admin-email alerting + log purge) — none of which this block does (it only calls `$ost->setWarning()`). The exact battery (clauses a–d, the `0x0010`/`0x0002` bit test, the `chmod 644` example, the register-globals precedence) is documented **verbatim in FS-001.10** clause (d) (Admin-Realm Gate, spec line 140) and in **EC-010 / EC-011 / KL-010 / KL-026** which all back-reference FS-001.10. | **FS-001.10** (already the correct owner; EC-010/EC-011/KL-010 supporting). The companion `FS-001.12` tag on this block should be **replaced with FS-001.10** (the FS-061.2 tag on the upgrade-pending branch at lines 30/33-39 is correct and stays). | No — retag only |

## Re-examined prior "weak/quirk" items — confirmed COVERED (NOT re-raised)

- `scp/pwreset.php`:61-63 — defective reset-window boolean (`!($ts=lastModified()) && (window < time-strtotime($ts))`) never rejects a stored token by age. **Documented**: FS-002 EC-002-13, KL-002-09, BS-002-07, plus narrative at FS-002 lines 59/450/507. COVERED.
- `scp/logout.php`:20-21 — missing/invalid `auth` link-token sets a redirect header but does NOT `exit`; session is destroyed regardless. **Documented**: FS-002.10 + the known link-token defect KL. COVERED.
- `scp/faq.php`:32-85 — POST mutation handlers (create/add/update/edit/manage-faq publish/unpublish/delete) have **no** `canManageFAQ()` gate (only the edit-form *view* routing at lines 97/99 is gated). **Documented explicitly**: FS-050 KL-050.1 (spec line 480) + EC-050.6 (line 417-418). COVERED.
- `scp/staff.php`:44 — mass action comment says "Enable / Lock / Delete" but code is enable/disable/delete; self-protection via `in_array($thisstaff->getId(),...)` + per-row `staff_id!=self` SQL guard. Behavior owned by FS-031.4 + BS-031-015. Tag wording ("Lock") is cosmetic, not a coverage gap. COVERED.
- `scp/departments.php`:80-85 — bulk delete refuses the entire batch if any selected dept has staff. **Documented**: FS-030.6, BS-030-06, EC-030-04, KL-030-05. COVERED.
- `scp/image.php`:29 vs `scp/file.php`:28 — image.php compares the full 64-char `getDownloadHash()` while file.php recomputes only the last 32 chars; both equivalent session-bound checks. Both tagged BS-022.8 + FS-022.10. COVERED.
- `scp/tickets.php`:591 — `a=print` PDF export gated only by ticket presence (no explicit print permission). Owned by FS-021.17. COVERED (documented as owned; no untagged code).
- `scp/ajax.php`:53-69 — dispatcher mounts `/report/overview/*`, `/users`, `/tickets/*` (lock/preview/lookup/search), `/upgrader` routes that are not individually tagged here; only the dispatcher (FS-043.3/.13) + the two called-out routes (FS-033.8/.10) carry tags. The route **handlers** live in `include/ajax.*.php` (P8 partition) where they are tagged; the dispatcher mounting itself is config owned by FS-043.3. Not a P2 gap.
- `scp/emailtest.php`:54 — `$action` is undefined in the hidden form field (renders empty). Cosmetic/inert; the test-send behavior is FS-040.12. Not a coverage gap.

## Trivia (not tagged, by design)

License header blocks, `require('staff.inc.php')`/`require('admin.inc.php')` bootstrap
lines, `include_once(INCLUDE_DIR.'class.*.php')` requires, `$nav->setTabActive(...)`,
and the trailing `header.inc.php … $page … footer.inc.php` render trio (the *page-
selection* logic that chooses which `.inc.php` to load WAS tagged; the literal
header/footer includes are FS-090 shell chrome). ~90 such lines across the partition.

## Round 2 closure (2026-06-10)

- **CLOSED** — `scp/admin.inc.php`:31 MIS-TAG. The security/maintenance sysnotice battery
  tag was re-pointed from `FS-001.12` (System Logging & Admin Alerting — wrong) to
  **FS-001.10** (Admin-Realm Gate clause (d); EC-010/EC-011/KL-010 noted in the comment).
  The adjacent `FS-061.2` upgrade-pending tag was left untouched. Finding closed.
