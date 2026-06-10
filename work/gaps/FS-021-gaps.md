# FS-021 Gap Report — Staff Ticket View & Workflow

Source slice re-read: `scp/tickets.php`, `include/class.ticket.php` (full 2298 lines), `include/class.thread.php`, `include/class.lock.php`, `include/ajax.tickets.php`, `include/staff/ticket-view.inc.php`, `include/staff/ticket-open.inc.php`.

Constraint honored: no source file modified; only `specs/FS-021-staff-ticket-view-workflow.md` + this gap file edited.

| # | Type | Severity | What the code does | What the spec said | Fix applied |
|---|------|----------|--------------------|--------------------|-------------|
| 1 | MISSING | High | `scp/tickets.php` listing branch serves `a=export` CSV via `search_<token>` session query; errors "Query token required" / "Query token not found" / "Internal error: Unable to dump query results". | No mention of the export action in this script. | Added bullet to FS-021.1 + EC-021.22. |
| 2 | INCORRECT | High | Close/reopen reason note field is `ticket_status_notes`; the generic close/reopen note POST is not `notes`. | Data Requirements listed `notes` for the status-note. | Corrected POST-fields list; `notes` now documented as the print "include internal notes" flag. |
| 3 | IMPRECISE | Medium | `reply_ticket_status` carries literal `Open`/`Closed`; status row renders when `isClosed() OR canCloseTickets()`; transition runs **before** `onResponse`. | FS-021.3 said only "Close/Reopen on Reply" applied via setStatus, no value/order detail. | Expanded FS-021.3; added BS-021.22. |
| 4 | MISSING | Medium | `previewTicket` returns HTTP 404 "No such ticket" on no-access; its lock banner keys off `getStaffId()==thisstaff->getId()` (own lock), not someone else's. | FS-021.18 said preview "shows a lock warning" for another's lock. | Corrected FS-021.18 + added KL-021.10 + EC-021.23. |
| 5 | MISSING | High | Note-form `state=unassigned` is inert: `setState` switch tests misspelled `unassined`; only `process/release` actually unassigns. | FS-021.9 implied note-form release works. | Corrected FS-021.9 + added KL-021.9 + EC-021.18. |
| 6 | MISSING | Medium | Max-open-tickets ceiling enforced only when `origin != 'staff'`; staff bypass it; user over-limit notice suppressed for staff origin. | FS-021.20 didn't state the staff bypass. | Added bullets to FS-021.20 + BS-021.19 + EC-021.21. |
| 7 | MISSING | Medium | `create()` suppresses auto-response for system-account sender, detected auto-reply, `mailer-daemon@`/`postmaster@`, or posted canned reply. | FS-021.16 listed alerts but not the suppression matrix. | Added bullet to FS-021.16 + BS-021.20 + EC-021.20. |
| 8 | IMPRECISE | Low | `markOverdue`/`markAnswered`/`markUnAnswered` short-circuit to success when already in state (no event/alert/write). | Not stated. | Added bullet to FS-021.13 + BS-021.21. |
| 9 | MISSING | Low | External ticket id: random fixed-length (regenerated on collision) vs sequential = auto-increment id written back post-insert. | "random or sequential per config" only, no mechanics. | Added bullet to FS-021.20 + BS-021.23. |
| 10 | MISSING | Low | onMessage auto-assign-to-last-respondent gated by `autoAssignReopenedTickets()` config + respondent availability. | FS-021.5 said "when configured" without naming the gate. | Named the config gate in FS-021.16. |
| 11 | MISSING | Low | Thread `create` rejects non-M/R/N types; auto message-id `<{rand24}@{md5(url) last 10}>`; `reply_to` → pid inheritance; response defaults pid to msgId; failed uploads logged as SYSTEM notes. | FS-021.22 only mentioned auto message-id generically. | Expanded FS-021.22. |
| 12 | MISSING | Low | `pdfExport` resolves a non-string `psize` via session → staff default → `Letter`; print reachable by GET preset links and POST dialog; streams `Ticket-<n>.pdf` and exits. | FS-021.17 had the size fallback but not the non-string entry path / GET+POST. | Expanded FS-021.17. |
| 13 | MISSING | Low | View "More" menu gated by `canBanEmails() OR dept-manager`; assigned-elsewhere warning banner; view re-checks access ("Access Denied" die); banner precedence err→msg→warn; view-banner ban wording differs ("…before any reply/response"). | View affordances/banners not specified. | Added FS-021.23a + FS-021.1 bullets. |
| 14 | MISSING | Low | Canned reply form has an "Append" checkbox (default checked); "Email Reply" pre-checked by default; canned reply leaves ticket unanswered. | Not stated. | Added bullets to FS-021.3. |
| 15 | IMPRECISE | Low | Unknown action → "Unknown action"; empty process sub-action → "You must select action to perform"; generic reply/note failure messages. | Error strings not enumerated. | Added bullets to FS-021.1, FS-021.3, FS-021.4. |
| 16 | MISSING | Low | Quick-stats nav: assigned>10 warning + overdue>10 sysnotice text. | Not stated (queue belongs to FS-020 but these fire in this script). | Added bullet to FS-021.1. |
| 17 | MISSING | Low | `postEmail` idempotent on already-fetched message-id (returns success). | Not stated. | Added EC-021.19. |
| 18 | MISSING | Low | Lock `renew` with no lock-time re-extends by original window; `acquire` uses insert-if-absent; renew-when-owned in `acquireLock`/`renewLock`. | FS-021.18 lacked these mechanics. | Added bullets to FS-021.18. |

## Summary
- Gaps found/fixed: **18** — MISSING ×12, INCORRECT ×2, IMPRECISE ×4.
- Severity: High ×4, Medium ×4, Low ×10.
- New spec ids added: **1 FR** (FS-021.23a), **5 BS** (BS-021.19–023), **6 EC** (EC-021.18–023), **2 KL** (KL-021.9–10), plus numerous in-place clarifications to existing FRs (021.1, .3, .4, .9, .13, .16, .17, .18, .20, .22).
- Most significant: (5) note-form release is silently inert due to the `unassined` typo; (4) preview lock banner keys off the wrong owner so it never warns about another agent's lock; (6/19) staff-created tickets bypass the max-open-tickets ceiling.
