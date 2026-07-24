# FS-090 Gap Report — Shared UI, Navigation & Data Export

Phase-2 GAP-CLOSE run, 2026-06-10. Source slice re-read with fresh eyes against the spec, section by section.
Sources: `include/class.nav.php`, `include/class.pagenate.php`, `include/class.export.php`, `include/staff/{header,footer}.inc.php`, `include/client/{header,footer}.inc.php`, `include/ajax.reports.php`, `include/staff/tpl.inc.php` (+ corroborating `scp/tickets.php`, `include/staff/tickets.inc.php`).

| # | id | type | severity | What the code does | What the spec said | Fix applied |
|---|----|------|----------|--------------------|--------------------|-------------|
| 1 | FS-090.29 (new) | MISSING | High | `PageNate` constructor has a **second** bounds-correction branch (`class.pagenate.php:35-37`): when `(limit−1)*start > total`, `start -= start % limit` snapping start down to a page boundary — runs in addition to the line-32 reset-to-0 branch. | Only the line-32 reset-to-0 branch documented (FS-090.15, BS-090.8). The modulo-snap branch was entirely absent. | Added FS-090.29 describing the secondary modulo correction and its ordering vs the primary reset. |
| 2 | FS-090.30 / BS-090.15 / EC-090.12 (new) | MISSING | High | `ResultSetExporter` ctor (`class.export.php:73-96`) only runs the header→column intersection projection **inside** the `if ($row = db_fetch_array(...))` guard. With zero rows, `$this->headers` stays as the full raw `array_values($headers)` and keys/lookups are never set — so an empty CSV export emits **all** header-map columns. | BS-090.10 stated the export always emits the intersection of header map and query columns; the empty-result fallback to the full raw map was undocumented. | Added FS-090.30 (header behavior), BS-090.15 (rule), EC-090.12 (edge case). |
| 3 | FS-090.28 (new) + FS-090.4 fix | INCORRECT | Medium | Client nav render (`client/header.inc.php:51`) emits `class="<active> <key>"` for **every** link — the link-key class is unconditional, not active-only. | FS-090.4 said "the active link carries CSS class `active` plus a class equal to its link key", implying the key class accompanies only the active link. | Corrected the FS-090.4 bullet and added FS-090.28 with the precise always-present-key-class render contract. |
| 4 | FS-090.32 / EC-090.15 (new) | MISSING | Medium | `getPageLinks()` (`class.pagenate.php:84-126`) does edge-credit redistribution: `stopcredit`/`startcredit` carry the under/overflow at one edge to the opposite side to keep the window width stable; chevrons target `start_loop−span` / `stop_loop+span` clamped. | FS-090.17 described a fixed 5-each-side window and chevrons but omitted the edge-compensation redistribution. | Added FS-090.32 (window edge compensation) + EC-090.15. |
| 5 | FS-090.31 / EC-090.13 (new) | MISSING | Low-Med | `Export::dumpQuery` (`class.export.php:19-26`) looks `$how` up in a 2-key map with **no default**; an unknown value yields a null class name → fatal `new $null(...)`. | FS-090.20 said format is "selected by a `how` argument resolving to a concrete exporter; supported `csv`/`json`" but did not state there is no fallback / unknown is fatal. | Added FS-090.31 + EC-090.13 documenting the no-fallback fatal path. |
| 6 | BS-090.16 / KL-090.6 | IMPRECISE | Medium | `downloadTabularData()` (`ajax.reports.php:155-163`) hand-builds CSV with `implode('","', ...)` and **no** `str_replace('"','""',...)` quote-doubling, unlike `CsvResultsExporter`. | KL-090.6 noted the duplication and "can drift" but did not pin the concrete correctness consequence (report CSV is not quote-safe). | Added BS-090.16 stating the report inline builder does not escape embedded quotes, with a malformed-output example. |
| 7 | FS-090.23 | IMPRECISE | Low | Hash is specifically `md5($query)` (`tickets.inc.php:292-293`); the stored query string still contains its trailing `LIMIT start,limit` clause; export link carries the raw `status` filter (`tickets.inc.php:477-478`). | FS-090.23 said "key derived from the query's hash" without naming MD5, and did not note the stored SQL retains its LIMIT (stripped later) nor that `status` is carried verbatim. | Made FS-090.23 precise: MD5 digest, LIMIT-retained-then-stripped, verbatim status. |
| 8 | FS-090.26 | IMPRECISE | Low | `downloadTabularData` filename uses raw `get('group','Department')` → `dept-report.csv` / `topic-report.csv` / `staff-report.csv`, default literal `Department-report.csv`; the stem is not normalized to a display name. | FS-090.26 said `<group>` "is the grouping key (defaulting to 'Department')" without clarifying the present-param value is the lowercase key while the default is the display word. | Clarified FS-090.26 with the exact raw-param vs default-word behavior. |
| 9 | FS-090.11 | IMPRECISE | Low | Client logo anchor (`client/header.inc.php:23-26`) carries fixed `title="Support Center"`, is `ROOT_PATH`-prefixed, and alt text comes from `$ost->getConfig()->getTitle()` (a different access path than the local `$title`). | FS-090.11 said logo "served via `logo.php` with the configured helpdesk title as alt text and links to `index.php`" — omitted the anchor title, ROOT_PATH prefix, and the distinct config access path. | Expanded FS-090.11 logo bullet. |
| 10 | EC-090.14 | MISSING | Low | `DatabaseExporter::dump()` (`class.export.php:209-213`) on a no-fields table writes the diagnostic then calls bare `die()` — hard process termination mid-stream, after prior tables' blocks already flushed. | EC-090.10 said the backup "aborts mid-stream with an error" but did not note it is a hard `die()` leaving prior blocks already written. | Added EC-090.14 noting the hard-termination semantics and partial-output side effect. |

## Summary

- **Gaps found / fixed: 10** (all fixed).
  - By type: **MISSING 5** (#1, #2, #4, #5, #10), **INCORRECT 1** (#3), **IMPRECISE 4** (#6, #7, #8, #9).
  - By severity: High 2, Medium 4, Low-Med/Low 4.
- **New IDs added to FS-090:**
  - FRs: **FS-090.28 – FS-090.32** (5 new).
  - BS rules: **BS-090.15, BS-090.16** (2 new).
  - Edge cases: **EC-090.12 – EC-090.15** (4 new).
  - Plus in-place corrections to FS-090.4, FS-090.11, FS-090.23, FS-090.26 (no renumbering).

## Most significant

1. **(#1) Undocumented second pagination bounds branch** — `start -= start % limit` was completely missing; a real second code path that snaps the offset to a page boundary.
2. **(#2) Empty-result export emits the full header-map column set** — the intersection projection only runs when ≥1 row exists, so empty exports carry a *different* (wider) column set than populated ones — a behavior contradiction with the previously documented BS-090.10.
3. **(#3) Client nav key-class is unconditional** — the spec wrongly implied the link-key CSS class accompanies only the active link; it is emitted for every link.

No source files modified (report spec only). Only `specs/FS-090-shared-ui-navigation-data-export.md` and this gap file were written.
