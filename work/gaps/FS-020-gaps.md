# FS-020 Gap Report — Staff Ticket Queue, Dashboard & Search

Source slice: `scp/tickets.php`, `scp/index.php`, `scp/dashboard.php`, `include/staff/tickets.inc.php`, `include/ajax.reports.php`, plus `Ticket::getStaffStats` (`include/class.ticket.php`), staff permission/visibility helpers (`include/class.staff.php`), page-size resolution (`main.inc.php`, `scp/staff.inc.php`).

The Phase-1 draft was already strong (KL-020.4 even pre-documented the team-assignment gap). Gaps below are mostly correctness/precision refinements around the quick-stats query and advanced-search query shapes.

| id | type | severity | what the code does | what the spec said | fix applied |
|----|------|----------|--------------------|--------------------|-------------|
| G1 | INCORRECT | High | In `Ticket::getStaffStats`, the `staff_id=0` (unassigned) constraint `$where2` is appended **only** to the `open` UNION branch; `answered`, `overdue`, `assigned`, `closed` branches do not get it. | FS-020.11 said "the open/answered/overdue scope is additionally constrained to unassigned tickets (`staff_id=0`)" — implying answered/overdue too. | Corrected FS-020.11 and added BS-020.17: constraint applies only to the `open` count branch. |
| G2 | INCORRECT | High | `Staff::showAssignedTickets()` returns true only when the per-staff flag is set AND (isAdmin OR isManager of home dept). | BS-020.4 / FS-020.11 referred to "the staff member is configured to show assigned tickets" / "the staff override" with no admin/manager gate. | Added BS-020.18; updated BS-020.4 and FS-020.11 to require admin-or-manager for the override to take effect. |
| G3 | IMPRECISE | Medium | Advanced search assignee+closed-by has three distinct query shapes; notably when `staffId` is set **and a status is present**, the OR clause is `OR status="closed"` (ALL closed in scope), not closed-by-X; and a standalone `staffId`-only path exists. | FS-020.8 described only a single "ORs in a closed-by clause when staffId provided". | Expanded FS-020.8 into three enumerated paths and added BS-020.16. |
| G4 | IMPRECISE | Low | Assignee filter is suppressed when search status == "closed" (`strcasecmp(status,'closed')`); plain-numeric assignee → staff id; predicate wrapped as `(status='open' AND …)`. | FS-020.8 said "only applied when status is not closed" / "plain numeric is treated as a staff id" but lacked the wrapping/`strcasecmp` detail. | Clarified FS-020.8 assignee bullet. |
| G5 | MISSING | Low | A `mass_process` with an unrecognized `do` value (past the gate + non-empty selection) returns "Unknown or unsupported action - get technical help" with no changes. | No edge case documented for unknown bulk action. | Added EC-020.13. |
| G6 | MISSING | Low | On a search request the query string collector appends `&t=<urlencode($_REQUEST['t'])>` — an opaque pass-through token with no listing effect, preserved across sort/page/export. | The `t` parameter was undocumented. | Added `t` (and `basic_search`) to the Request/State Inputs table. |
| G7 | IMPRECISE | Low | When no display sort is set, the sort CSS-class variable defaults to key `urgency` (not a visible column), so no header shows active-sort; resolved order defaults to `DESC`; multi-term default sorts splice the order before each non-ASC/DESC comma term. | FS-020.5 noted "urgency is not on display table" only in passing and didn't cover the order-splice/default-DESC behavior. | Expanded FS-020.5 acceptance criteria. |
| G8 | IMPRECISE | Low | Closed-By select "Anyone" has value `0` (falsy → no filter); page-size resolution chain confirmed `getPageLimit() ?: (getPageSize() ?: 25)`. | FS-020.8 listed "Anyone" without noting the falsy-0 no-op; page-size chain already correct. | Clarified Closed-By "Anyone (value 0, falsy)" in FS-020.8; page-size left as-is (already correct). |

## Summary

- Gaps found: **8** — INCORRECT 2, IMPRECISE 4, MISSING 2.
- Gaps fixed: **8** (all).
- By severity: High 2, Medium 1, Low 5.
- New rules added to FS-020: **3 BS** (BS-020.16, BS-020.17, BS-020.18), **2 EC** (EC-020.13, EC-020.14). No new FRs (existing FRs corrected/expanded in place: FS-020.5, FS-020.8, FS-020.11; BS-020.4 corrected).
- No source files modified (report-spec-only constraint honored).
