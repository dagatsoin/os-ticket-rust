# FS-030 Gap Report — Admin: Departments, Teams & Help Topics

Phase-2 GAP-CLOSE run, 2026-06-10. Source slice: `scp/{departments,teams,helptopics}.php`,
`include/staff/{department(s),team(s),helptopic(s)}.inc.php`,
`include/class.{dept,team,topic}.php`. Report-spec-only; no source modified.

| ID | Type | Severity | What the code does | What the spec said | Fix applied |
|----|------|----------|--------------------|--------------------|-------------|
| G-01 | MISSING | Medium | `Dept::save()` on UPDATE returns true only when `db_query($sql) && db_affected_rows()` — a no-op re-save (no field changed, same one-second tick already in `updated`) returns 0 affected rows → `Unable to update <Name> Dept. Error occurred`. | FS-030.5 stated update succeeds on validation pass; no mention of the affected-row requirement or the spurious no-op error. | Added no-change update guard bullet to FS-030.5; new EC-030-15; new KL-030-11. |
| G-02 | MISSING | Medium | `Team::save()` on UPDATE has the identical `db_affected_rows()` success gate → no-op team save yields `Unable to update the team. Internal error`. | FS-030.10 silent on this. | Added no-change update guard bullet to FS-030.10; folded into EC-030-15 / KL-030-11. |
| G-03 | INCORRECT | Medium | `Topic::save()` on UPDATE returns true on `db_query($sql)` alone — **no** affected-row check; a no-op topic save still reports `Help topic updated successfully`. Behaviorally inconsistent with dept/team. | FS-030.15 implied uniform save behavior across the three objects. | Added explicit "no no-op guard on topic update" bullet to FS-030.15; cross-referenced KL-030-11. |
| G-04 | INCORRECT | Low | Team list partial renders a `sort=updated` header link, but `updated` is not in the team `$sortOptions`; click falls back to default `name` sort (dead link). | FS-030.8 listed valid sort keys (correct) but presented "Last Updated" as a normal sortable column. | Clarified FS-030.8 (header is a dead sort link; Created=date, Updated=datetime); new KL-030-12. |
| G-05 | IMPRECISE | Low | Help-topic Priority select opens on placeholder `— Select Priority —` (value `""`); Department select opens on `— Select Department —` (value `""`). | FS-030.14 described Priority/Department as plain "lists ..." with no leading placeholder. | Added placeholder options to FS-030.14; new EC-030-16. |
| G-06 | MISSING | Low | `noreply_autoresp` is read by `Dept` but is **not** present in the department add/edit form and **not** in the save's column set (read-only at this layer). | Data Requirements + BS-030-09 listed `noreply_autoresp` alongside the editable auto-response flags as if managed here. | Clarified BS-030-09 and Data Requirements that `noreply_autoresp` is read-only / not written by this form. |
| G-07 | MISSING | Low | Team `name`/`notes` and topic `notes` are persisted as submitted (not HTML-stripped); only dept `name`/`signature` and topic `topic` text are stripped. | BS-030-11 covered dept; BS-030-27 covered topic text; nothing stated teams/notes are NOT stripped. | Added "not HTML-stripped" bullets to FS-030.10 and FS-030.15; new KL-030-13. |
| G-08 | IMPRECISE | Low | `departments.inc.php` and `helptopics.inc.php` list partials use the shorter gate `if(!defined('OSTADMININC') \|\| !$thisstaff->isAdmin())` (no `!$thisstaff` term); form partials + team list use the full three-term guard. | FS-030.1 / overview asserted "every view partial begins with" the full three-term `if(...)`. | Clarified FS-030.1 to distinguish the two guard variants and note equivalence in the supported flow. |

## Summary

- **Gaps found / fixed: 8** — MISSING 4, INCORRECT 2, IMPRECISE 2.
- **Severity: 0 high, 3 medium, 5 low.**
- **Most significant:**
  1. (G-01/G-02/G-03) The three objects are behaviorally inconsistent on a no-change update: department and team saves require a nonzero affected-row count and can spuriously error on a no-op re-save, while help-topic saves always report success. This was entirely undocumented and is now captured as a save-rule bullet on each FR plus EC-030-15 / KL-030-11.
  2. (G-04) The team "Last Updated" column is a non-functional sort header (`updated` absent from the team sort map), now flagged as KL-030-12.
  3. (G-06) `noreply_autoresp` is read-only at this management layer (no control, not in the save column set) — previously listed among the editable auto-response flags.

## Spec deltas

- New FRs: 0 (existing FRs amended in place: FS-030.1, FS-030.5, FS-030.8, FS-030.10, FS-030.14, FS-030.15).
- New BS rules: 0 (BS-030-09 amended).
- New ECs: 2 (EC-030-15, EC-030-16).
- New KLs: 3 (KL-030-11, KL-030-12, KL-030-13).
- Data Requirements: `noreply_autoresp` note corrected.
