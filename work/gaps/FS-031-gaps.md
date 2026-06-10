# FS-031 Gap Report — Admin: Staff, Groups & Directory

Phase-2 GAP-CLOSE. Source slice: `scp/{staff,groups,directory,profile}.php`,
`include/staff/{staffmembers,staff,groups,group,directory,profile}.inc.php`,
`include/class.staff.php`, `include/class.group.php` (+ `class.validator.php` cross-check).

Report-spec-only: no source files modified. Only `specs/FS-031-*.md` + this file edited.

| id | type | severity | what the code does | what the spec said | fix applied |
|----|------|----------|--------------------|--------------------|-------------|
| G-01 | MISSING | medium | `Staff::updateProfile` engages the password block when **any** of `passwd1`/`passwd2`/`cpasswd` is non-empty; a blank new password then reports `New password required`. | FS-031.5 / BS-031-013 only covered the admin-create `Temp. password required` message; the profile trigger condition and `New password required` were absent. | Added **FS-031.13** (own-profile password section: trigger condition + ordered messages + two auth paths); extended **BS-031-013**; added **EC-031-014**. |
| G-02 | MISSING | medium | Profile current-password path emits literal `Current password required` (blank) and `Invalid current password!` (wrong). | BS-031-013 said current password "must be correct" but did not give the literal messages. | Added the literals to **BS-031-013** and **FS-031.13**. |
| G-03 | MISSING | low | The two staff forms use **different** literals for a duplicate staff email: admin `Email already in use by another staff member` vs profile `Email already in-use by another staff member` (hyphen/spacing differs). System-email collision similarly differs (`Already in-use system email` vs `Already in-use as system email`). | FS-031.5 listed only the admin wording; EC-031-007 covered only the system-email difference. | Added note in FS-031.5 + new **EC-031-016**. |
| G-04 | MISSING | medium | `_do_login` + `isPasswdResetDue` force `change_passwd=1` on login for non-admins when `passwd_change` age exceeds `getPasswdResetPeriod()×30×24×60×60`; admins exempt. | Spec had forced-change banner (FS-031.11) but never documented the periodic password-aging that *sets* the flag. | Added **FS-031.12** (periodic password-reset aging) + **KL-031-007** (30-day-month / TZ caveat). |
| G-05 | INCORRECT | low | Forced-Password-Change checkbox carries literal value `0`; the flag is set by field **presence** (`isset($vars['change_passwd'])`), not its value. | FS-031.3 described it as a plain "forces a password change" checkbox with no mention of the presence-driven / inverted-value quirk. | Clarified in **FS-031.3** + new **EC-031-013**. |
| G-06 | MISSING | low | Staff form (and profile, group form) include a `Reset` button (HTML form reset) alongside Cancel; profile Cancel label is `Cancel Changes`, Reset is `Reset Changes`. Group form Cancel returns to `groups.php`. | FS-031.3 / FS-031.8 / FS-031.10 mentioned only Cancel (and FS-031.8 mentioned neither). | Added Reset/Cancel notes to **FS-031.3**, **FS-031.8**, **FS-031.10**; new **KL-031-009**. |
| G-07 | IMPRECISE | low | Profile Auto-Refresh-Rate options are 1–10 in steps of 1, then 11–30 in steps of 2 (non-uniform). Max-page-size highlights the effective value (per-staff override else system default). | BS-031-012 / FS-031.10 said a uniform "1–30 minutes." | Corrected stepping in **BS-031-012** + **FS-031.10**; also pinned the `none` stored value for default-signature-type and default-paper-size. |
| G-08 | IMPRECISE | low | `Staff::delete` returns 0 (no-op) when no acting staff is in context OR target == acting staff; house-cleaning (unassign open tickets, drop team memberships) runs only when a row was actually removed; deletion signal fires regardless. | BS-031-016 said only "permitted only when it is not the currently acting staff." | Tightened **BS-031-016** (both guards, conditional house-cleaning, unconditional signal). |
| G-09 | MISSING | low | `Staff::getDepartments` has a fallback (when the union query returns nothing) to `group depts ∪ primary dept`, de-duplicated/null-filtered; `canAccessDept` additionally requires NOT Limited-Access. | BS-031-005 gave the union but not the fallback or the de-dup/null-filter or the Limited-Access interaction at the access-check level. | Expanded **BS-031-005**. |
| G-10 | MISSING | low | Staff-list **Group** and **Team** filter drop-downs rely on INNER JOIN (no explicit `HAVING count>0`), while the **Department** drop-down uses an explicit `HAVING users>0`; all show `Name (count)` with unrestricted counts (admin list, so visibility not factored — contrast directory). | FS-031.2 said "groups/teams that have at least one member" but did not document the population mechanism or the visibility contrast. | Added **BS-031-033**. |
| G-11 | MISSING | low | Group list uses LEFT JOINs so zero-member / zero-dept groups still appear; member count links to `staff.php?gid=` only when >0 (else plain `0`); dept count is always a plain number. | FS-031.7 described the columns/links but not the complete-enumeration (left-join) property nor that dept-count is never linked. | Added **BS-031-034**. |
| G-12 | MISSING | low | Reset-token window check on profile (`_config->lastModified` unreadable OR age > `getPwResetWindow()`) → `Invalid reset token. Logout and try again`; observed code shape is order-dependent on token freshness. | EC-031-008 covered the mismatch case but not the window/unreadable-timestamp edge or the observed ordering quirk. | Added **EC-031-015** + **KL-031-008**. |

## Summary

- Gaps found / fixed: **12** (MISSING ×9, INCORRECT ×1, IMPRECISE ×2).
- Most significant:
  1. **G-04 / FS-031.12** — entirely undocumented periodic password-reset aging that auto-forces a password change at login for non-admins (a real security/UX behavior the spec missed).
  2. **G-01/G-02 / FS-031.13** — the own-profile password-change flow (engage condition, ordered messages, reset-token vs current-password auth) was largely absent; now its own FR.
  3. **G-05 / EC-031-013** — Forced-Password-Change checkbox is presence-driven with a misleading literal value `0`, a genuine "gotcha" for admins.

## New artifacts added to FS-031

- New FRs: **FS-031.12**, **FS-031.13** (2).
- New BS: **BS-031-033**, **BS-031-034** (2); revised BS-031-005, BS-031-012, BS-031-013, BS-031-016.
- New ECs: **EC-031-013 … EC-031-016** (4).
- New KLs: **KL-031-007 … KL-031-009** (3).
- Revised FRs: FS-031.3, FS-031.5, FS-031.8, FS-031.10.

No existing ids renumbered; all additions append to the existing sequences.
