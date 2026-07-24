# FS-032 Gap Report — Admin System Settings, SLA, Priorities & FAQ Categories

Phase-2 GAP-CLOSE. Source slice: `scp/settings.php`, `scp/slas.php`,
`scp/categories.php`, `include/staff/settings-{system,tickets,emails,pages,kb,autoresp,alerts}.inc.php`,
`include/staff/slaplan(s).inc.php`, `include/staff/categor*.inc.php`,
`include/class.sla.php`, `include/class.priority.php`, `include/class.category.php`,
`include/class.config.php` (+ cross-check `class.ticket.php` overdue sweep,
`install-mysql.sql` seeds).

All gaps fixed directly in `specs/FS-032-admin-system-settings-sla-priorities-categories.md`.
No source files modified.

| # | ID(s) | Type | Severity | What the code does | What the spec said | Fix applied |
|---|-------|------|----------|--------------------|--------------------|-------------|
| 1 | FS-032.1, BS-032.1, KL-032.8 | INCORRECT | Medium | `settings-alerts.inc.php` and `settings-autoresp.inc.php` have NO `die('Access Denied')` in-partial guard; the other five partials (system/tickets/emails/pages/kb) do. | "Each tab body … the partial dies with Access Denied unless …"; "every settings tab partial … independently re-assert this gate." Asserted uniformly. | Corrected FS-032.1 and BS-032.1 to name the five guarded partials and the two unguarded ones; added KL-032.8 documenting the inconsistent defense-in-depth. |
| 2 | FS-032.6, KL-032.3 | INCORRECT | Medium | `send_sys_errors` checkbox is `disabled`, never submitted, so `updateAlertsSettings` writes `send_sys_errors=0` on every save. No consumer reads the key (no `alertONSysError()`). | "shown checked and *disabled* (always on)"; KL-032.3 "system error alerts … always on." | Rewrote FS-032.6 System-Alerts bullet to state the save persists `0`; rewrote KL-032.3 to explain the UI-vs-stored disagreement and that the key is effectively dead. |
| 3 | FS-032.6, KL-032.9 | MISSING | Low | Transfer-alert status row renders `$errors['alert_alert_active']` (typo); the real error is keyed `transfer_alert_active`, so the "Select recipient(s)" message for transfers never displays though the save is still blocked. | Not mentioned. | Added the defect to FS-032.6 and new KL-032.9. |
| 4 | FS-032.6 | IMPRECISE | Low | Six `*_alert_active` flags are written raw radio `1`/`0`; recipients are presence-based 0/1. | Recipient/active distinction not spelled out per-flag; BS-032.4 lumped generically. | Added a bullet to FS-032.6 distinguishing raw-radio active flags from presence-based recipient flags; clarified the per-event error key. |
| 5 | FS-032.4 | IMPRECISE | Low | Default-SLA option labels are `"{name} ({grace} hrs - Active|Disabled)"` from `SLA::getSLAs()` (name-ASC, includes disabled plans). Default-Priority select has no ORDER BY. | "the SLA plan list (see FS-032.8)" with no format; "enumerated from the priority set." | Documented the exact `getSLAs()` label format + inclusion of disabled plans, and that the priority select is unordered (natural id order). |
| 6 | FS-032.9, KL-032.10 | MISSING | Low | SLA list "Date Added" header links `sort=created`, not the recognized `date` key → silent fallback to name-ASC. `$created_sort`/`$updated_sort` marker vars never assigned. | Said "date (`created`)" maps and columns are sortable, implying Date Added sorts. | Rewrote FS-032.9 sort bullet with the recognized key map and the broken `sort=created` link; added KL-032.10. |
| 7 | FS-032.11, BS-032.10 | IMPRECISE | Medium | Overdue sweep (`class.ticket.php` ~L2280) measures grace from `reopened` for reopened tickets, from `created` otherwise; the derived `sla_duedate` column is always `created + grace`. | "due date is the ticket creation time plus the plan's grace period" — flat from creation only. | Added a reopened-exception clause to FS-032.11 and a "Reopen exception" note under BS-032.10 distinguishing the derived column from the reopened-aware sweep. |
| 8 | FS-032.14, EC-032.8, KL-032.11 | MISSING | Low | `Category::lookup($id)` returns an object for any non-zero numeric id WITHOUT the `getId()==$id` re-check that SLA/Priority use → hollow object for a nonexistent numeric id; "Unknown or invalid category ID" only fires for non-numeric/zero. | EC-032.8 implied any unresolved id errors symmetrically with SLA. | Added a lookup-robustness note to FS-032.14, split EC-032.8 into SLA vs Category behavior, added KL-032.11. Also documented name tag-strip-before-validate and description filter-on-save-only in FS-032.14. |
| 9 | EC-032.12, EC-032.13, EC-032.14 | IMPRECISE | Low | Logo confirm-dialog JS fires on ANY checked checkbox (not specifically delete); upload and `client_logo_id` selection are independent on save; system-default radio (value 0 / non-numeric) stores `client_logo_id=false`. | EC-032.12 said "the confirm dialog only intercepts when a delete checkbox is set." | Refined EC-032.12 to "any checkbox"; added EC-032.13 (upload-without-select) and EC-032.14 (system-default selection ⇒ `false`). |
| 10 | KL-032.12 | MISSING | Low | Site-Pages logo file input is `name="logo[]"` (array) but the save reads scalar `$_FILES['logo']`. Single-file path works; multi-file would not. | Not mentioned. | Added KL-032.12. |
| 11 | FS-032.9 | IMPRECISE | Low | `slaplans.inc.php` gate is `!$thisstaff->isAdmin()` only (no `!$thisstaff` null-guard); the form partial adds the `!$thisstaff` check. | Spec's gate prose implied a uniform "staff exists AND isAdmin()" guard. | Added a bullet to FS-032.9 noting the list-partial gate omits the null guard. |

## Summary

- **Gaps found / fixed**: 11 (all fixed).
- **By type**: MISSING 4 · INCORRECT 2 · IMPRECISE 5.
- **By severity**: Medium 3 · Low 8.
- **New IDs added to FS-032**: 0 new FRs, 0 new BS rules (existing FRs/BS corrected
  in place), +3 ECs (EC-032.13, EC-032.14, plus split/expansion of EC-032.8 &
  EC-032.12), +5 KLs (KL-032.8 … KL-032.12). No existing IDs renumbered.

## Most significant gaps

1. **Two settings partials skip the in-partial admin re-check (gap #1)** — the
   spec asserted a uniform per-partial `isAdmin()` gate, but `alerts` and
   `autoresp` rely on the entry-script gate alone. Material for any security
   reasoning about defense-in-depth.
2. **`send_sys_errors` "always on" is actually stored as 0 and is dead (gap #2)**
   — the spec's claim that system-error alerts are always-on via this key is
   false; the disabled checkbox forces `0` on every save and no consumer reads it.
3. **Reopened tickets restart the SLA grace clock from reopen, not creation
   (gap #7)** — a core overdue-semantics nuance the spec's flat "creation + grace"
   rule omitted, with a real divergence between the derived `sla_duedate` column
   and the overdue sweep.
