# FS-010 Gap Report — Public Client Portal (Lookup, View & Login)

Source slice re-read: `index.php`, `view.php`, `login.php`, `client.inc.php`,
`include/class.client.php`, `include/class.usersession.php`, `captcha.php`,
`include/class.captcha.php`, and the client templates
`include/client/{header,footer,login,view,tickets}.inc.php`.
Cross-checked against `include/class.staff.php` login routine for the lockout contrast.

Counts: **13 gaps** — by type MISSING ×6, INCORRECT ×3, IMPRECISE ×4.
Spec deltas: **+2 BS** (BS-010.12, BS-010.13), **+2 KL** (KL-010.8, KL-010.9),
KL-010.6 broadened from 1 to 4 sub-defects; FR/EC edits across FS-010.2/.5/.7/.8/.10/.11,
EC-010.1/.10, Data Requirements.

| # | id | type | sev | what the code does | what the spec said | fix applied |
|---|----|------|-----|--------------------|--------------------|-------------|
| 1 | BS-010.12 / KL-010.8 | INCORRECT | HIGH | `class.client.php:209` sets `$errors['login']='Invalid login'` immediately before the lockout guard `if(!$errors && strikes>max)` (line 211). `$errors` is non-empty → guard never true → lockout branch (Access Denied msg, `laststrike` set, excessive-attempts admin alert) is **dead code**. `laststrike` is therefore never written, so the top-of-routine lockout-window check (155-165) never engages either. Only strike increment + every-other warning log are live. Staff routine (`class.staff.php:631`) doesn't pre-set the error, so its lockout *does* fire. | FS-010.5 / BS-010.5 / EC-010.1 documented the lockout, `laststrike`, and admin alert as functional. | Added BS-010.12 + KL-010.8 documenting the dead branch; rewrote BS-010.5 (lockout "intended", inert), EC-010.1 (intended vs actual snapshot behavior), and the `laststrike` data-requirements row. |
| 2 | KL-010.6 (sub-3) | INCORRECT | MED | My-Tickets `tickets.inc.php` row markup echoes `$row['phone']`/`$row['phone_ext']` but the SELECT (`$qselect`) never projects those columns → Phone Number cell is always blank. | FS-010.8 listed "Phone Number" as a populated column. | Added a Columns bullet flagging the defect + KL-010.6 sub-defect 4. |
| 3 | KL-010.6 (sub-1) | INCORRECT | MED | Numeric-search branch interpolates `$queryterm` which is only assigned in the non-numeric branch → numeric ticket-number search uses an empty/undefined term. | Was KL-010.6 (already flagged, numeric only). | Folded into expanded KL-010.6 as sub-defect 1; FS-010.8 Search bullet unchanged (already correct intent). |
| 4 | KL-010.6 (sub-2) | MISSING | LOW | Subject column header links `sort=subj`; `subj` is not a recognized sort key (`subject` is) → Subject sort silently falls back to date. | Spec said sortable by subject; didn't note the header-key mismatch. | Expanded FS-010.8 Sort bullet + KL-010.6 sub-defect 2. |
| 5 | KL-010.6 (sub-3b) | MISSING | LOW | Order-by fallback literal is `'ticket_created'` (underscore), not the real `ticket.created` column. | Not documented. | Noted in FS-010.8 Sort bullet + KL-010.6. |
| 6 | BS-010.13 | MISSING | LOW | When no `status` requested and client has 0 open tickets, `$status` stays null → list shows ALL (not "open"). Explicit non-open/closed status normalized to "all". | FS-010.8 only documented the has-open-tickets → "open" default. | Added BS-010.13 covering the zero-open-tickets → "all" fallback. |
| 7 | FS-010.5 (warning log) | IMPRECISE | LOW | Even-numbered failed attempts log a *warning* "Failed login attempt (user)" — this IS live even though lockout is dead. | Spec listed it among lockout behaviors without distinguishing live vs dead. | BS-010.5 / EC-010.1 now separate the live warning log from the dead lockout. |
| 8 | FS-010.11 / EC-010.10 | IMPRECISE | LOW | Captcha generator requires **GD specifically** (`extension_loaded('gd')` + `gd_info`); on absence it returns BEFORE clearing/setting `$_SESSION['captcha']`, so the session value is left untouched. | Spec said "image library" and "session captcha value is first cleared and then set" (implying always). | FS-010.11 + EC-010.10 now specify GD and that clear/set happen only on the GD-present path. |
| 9 | FS-010.11 | MISSING | LOW | Background set is exactly 10 named PNGs; captcha string = `strtoupper(substr(md5(rand(0,9999)),rand(0,24),len))`; class ctor defaults len 6/font 7 vs endpoint 5/12; bg dir `images/captcha/`. | Spec said "fixed set" / "uppercase substring of random MD5" generically. | FS-010.11 now lists the 10 PNG names, the exact string formula, the dir, and the ctor-default vs invocation difference. |
| 10 | FS-010.7 | MISSING | LOW | `view.inc.php` repopulates the Message textarea from `Format::htmlchars($_POST)` on a failed POST; renders an inline `$errors['message']`; and renders its OWN `#msg_error/notice/warning` banner between thread and reply form. | Spec described the reply form but not error repopulation or the in-body banner. | Added two bullets to FS-010.7 (repopulation + in-body banner) and the inline message-error note. |
| 11 | FS-010.2 | MISSING | LOW | `login.inc.php` renders heading "Check Ticket Status" + prompt "To view the status of a ticket, provide us with the login details below." above the form. | Not documented. | Added to FS-010.2. |
| 12 | FS-010.10 | IMPRECISE | LOW | `header.inc.php`: the guest identity strip and the whole `#nav` list render only `if($nav)`; when `$nav` is unset an `<hr>` is rendered instead and no guest strip shows. Logged-in strip requires `$thisclient` object + `isValid()`; logout token from `$ost->getLinkToken()`. | Spec described strip/nav unconditionally. | FS-010.10 now gates both on `$nav` and notes the `<hr>` fallback + the exact guards. |
| 13 | KL-010.9 | MISSING | LOW | Deep search LEFT JOINs `ticket_thread` (M/R only); listing groups by `ticket.ticket_id` and the total uses `count(DISTINCT ticket.ticket_id)` to collapse the per-thread-entry fan-out. | Not documented as a structural sensitivity. | Added KL-010.9. |

## Notes / verified-correct (no change)

- FS-010.3/.4 login validation order, auth-token rule (BS-010.2/.3), CSRF rotation
  (BS-010.6), session token shape (FS-010.6), IP-binding-disabled-for-clients
  (BS-010.11), internal-notes exclusion (BS-010.7), staff-name hiding (BS-010.8),
  non-public dept substitution (BS-010.5), offline gate + logo exemption (BS-010.10),
  and the absent `tickets.php` controller (KL-010.1) all re-verified accurate
  against source.
- `ClientSession($email, $key)` construction in `client.inc.php` (email = userID,
  key = ticket number) matches the constructor signature and the spec.
