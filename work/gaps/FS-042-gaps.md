# FS-042 Gap Report — Ticket Filters, Banlist & Inbound Routing

Phase-2 GAP-CLOSE. Source slice re-read with fresh eyes against `include/class.filter.php`,
`include/class.banlist.php`, `scp/filters.php`, `scp/banlist.php`, `include/staff/filter(s).inc.php`,
`include/staff/banlist.inc.php`, plus the invocation sites in `include/class.ticket.php` (create),
`include/class.mailfetch.php`, `scp/tickets.php`, `include/staff/ticket-view.inc.php`, and the
`filter`/`filter_rule` schema in `install-mysql.sql`.

| id | type | severity | what the code does | what the spec said | fix applied |
|----|------|----------|--------------------|--------------------|-------------|
| G1 | MISSING | HIGH | `TicketFilter::apply()` is called **twice** in `Ticket::create()` (line ~2019 rejection-detect pass, line ~2037 mutation pass) — non-reject actions apply twice. | Spec described a single pass over matching filters. | Added FS-042.7 bullet on double-apply, BS-042-25, EC-042-13. |
| G2 | MISSING | HIGH | `isBanned` `equal` operator is mis-coded: acceptance condition `($pos===null && $result===$neg) || $result !== $neg` reduces to `result !== null`, always true → `equal` behaves like `contains` in the pre-screen. | Spec FS-042.8 said `equal` matches exact address only. | Added KL-042.8 documenting the operator-logic bug; clarified FS-042.8 / EC-042-6 distinction stands. |
| G3 | MISSING | MED | Create-time pre-screen (`class.ticket.php:1940`) runs only inside `if($vars['email'] && Validator::is_email(...))`; mailfetch + ticket-view ban checks have no validity guard. | FS-042.8 said pre-screen invoked "at the front of ticket creation" with no validity condition. | Added FS-042.8 bullets, BS-042-20, EC-042-16. |
| G4 | MISSING | MED | Filter `apply()` writes camelCase pending-var keys (`deptId`,`priorityId`,`slaId`,`staffId`,`teamId`,`autorespond`,`email`,`name`,`cannedResponseId`); help-topic/email-account defaults backfill only unset fields; canned response disables auto-response & leaves ticket un-answered; topic can still supply the *other* assignment slot. | FS-042.5 used `dept_id`/`priority_id` etc. and did not describe the default-interaction ordering or canned-response side effects. | Added FS-042.13, BS-042-21, BS-042-22; expanded KL-042.7. |
| G5 | MISSING | MED | `scp/tickets.php:56` rejects a staff **reply/response** when owner email is banned ("Email is in banlist. Must be removed to reply."); staff-origin new tickets also hit the create-time pre-screen. | FS-042.12 only described an informational banner. | Added FS-042.14, BS-042-23. |
| G6 | MISSING | MED | `matches()` email-id scope guard fires only when `target == 'Email'`; an `Any`-target filter with non-zero `email_id` ignores account scope. `getAllActive`/`quickList` only add the `email_id` clause when the incoming ticket carries an emailId. | FS-042.6 implied email-id scoping applied generally. | Added FS-042.6 bullets, BS-042-26, EC-042-14. |
| G7 | MISSING | MED | `quickList()` data-layer pre-narrowing heuristic (LOCATE substring match + negative-logic inclusion + un-considered-criterion inclusion) builds the candidate set when a sender email is present; `getAllActive()` used otherwise. | FS-042.7 described building "the active, target-scoped filter set" with no mention of the heuristic pre-filter. | Added FS-042.7 candidate-set bullet, KL-042.9, KL-042.10. |
| G8 | MISSING | LOW | Ticket-view "More" menu container shown when `canBanEmails() OR dept manager`, but Ban/Unban items require `canBanEmails()`; Ban shown only when not banned, Unban only when banned AND `unbannable`; submitter name passed but not persisted. | FS-042.12 said actions require permission, omitted the menu-container gate and item-visibility conditions. | Added FS-042.12 bullets. |
| G9 | MISSING | LOW | `Banlist::createSystemBanList` passes `'rules'=>array()`, which hits the `save_rules` "pre-built rules" bypass (`if(!$rules && is_array($vars['rules']))`), allowing a zero-rule filter despite BS-042-08. | BS-042-08 stated a filter with no rules cannot be saved (no exception noted). | Added BS-042-24. |
| G10 | IMPRECISE | LOW | `origin2target` returns null for unknown origins → `target IN ('Any', NULL)` loads only `Any` filters. | FS-042.6/BS-042-06 listed the known origin→target map but not the unknown-origin fallback. | Added EC-042-15. |
| G11 | MISSING | LOW | `save()` re-runs `save_rules` after the filter row is created and ignores its errors (`# nolint`). | Spec did not note best-effort rule persistence after row save. | Added KL-042.12. |
| G12 | INCORRECT | LOW | `banlist.inc.php`: duplicate `'created'` key maps Date-Added sort to `rule.updated`; sort header links point to `staff.php` not `banlist.php`. | FS-042.11 described searchable/sortable list without noting these source defects. | Added KL-042.13. |
| G13 | IMPRECISE | LOW | `Banlist::getFilter()` auto-creates the reserved filter, so `banlist.php`'s `if(!($filter=...))` "System ban list is empty." path is dead. | FS-042.11/EC referenced "System ban list is empty." as a live empty-state warning. | Added KL-042.14 (kept the message reference, flagged as effectively unreachable). |
| G14 | MISSING | LOW | Auto-response self-loop guard in create path suppresses auto-response for system-account / `mailer-daemon@` / `postmaster@` senders independent of the filter action. | Spec described only `disable_autoresponder` filter action. | Added EC-042-17. |
| G15 | MISSING | LOW | `name` ≤ 32 chars (DB), `val` ≤ 255 chars; editor does not validate, store truncates. | Data Requirements noted the lengths but spec body did not flag the silent-truncation behavior. | Added KL-042.15. |
| G16 | IMPRECISE | LOW | Reserved banlist filter created with no `target` in code → defaults to data-model `Any`; no `email_id`. | BS-042-13 called it "unscoped" without tying it to the `Any` default + 0 account. | Added KL-042.11. |

## Most significant gaps

1. **G2 — `isBanned` `equal` operator logic bug (KL-042.8)**: an `equal` ban rule in the fast
   pre-screen effectively behaves as `contains`, so any address containing the banned value as a
   substring is denied at the front door, while the full pipeline enforces true equality. This is a
   real divergence between the two ban paths that the original spec did not capture.

2. **G1 — double application of filter actions (BS-042-25 / FS-042.7)**: the action set runs twice
   per ticket; harmless for set-field actions but a genuine behavioral fact (and a latent footgun for
   any future non-idempotent action).

3. **G5 — reply-time ban enforcement (FS-042.14 / BS-042-23)**: banning an owner doesn't merely warn,
   it blocks staff replies at submit time — an enforcement the spec previously presented as cosmetic.

## Counts

- Gaps found / fixed: **16** (MISSING 11, INCORRECT 1, IMPRECISE 4).
- New requirements added: **FR +2** (FS-042.13, FS-042.14), **BS +7** (BS-042-20..26),
  **EC +5** (EC-042-13..17), **KL +8** (KL-042.8..15). Plus in-place clarifications to
  FS-042.6, FS-042.7, FS-042.8, FS-042.12.
