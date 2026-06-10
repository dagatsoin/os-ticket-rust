# FS-043 Gap Report — External API & Cron Scheduler

Phase-2 GAP-CLOSE. Source re-read with fresh eyes against the assigned slice
(`api/http.php`, `api/index.php`, `api/api.inc.php`, `api/cron.php`, `include/api.cron.php`,
`include/class.api.php`, `include/api.tickets.php`, `include/class.ajax.php`,
`include/class.dispatcher.php`, `include/class.cron.php`, `include/class.http.php`,
`scp/apikeys.php`, `scp/autocron.php`, `include/staff/apikey(s).inc.php`, plus referenced
`Ticket::checkOverdue`).

Result: **15 gaps** (8 MISSING, 5 INCORRECT/IMPRECISE split below, 2 verified-correct re-checks not
counted). All fixed in the spec.

| ID | Type | Severity | What the code does | What the spec said | Fix applied |
|----|------|----------|--------------------|--------------------|-------------|
| G1 | INCORRECT | High | Cron route is a **nested sub-dispatcher**: top matcher `^/tasks/` → child dispatcher with sub-pattern `^cron$` (prefix stripped, args carried forward). | Route table listed a flat `^/tasks/cron$` POST matcher. | FS-043.3 route table + new bullet describing the `/tasks/` → `cron` nesting (concrete instance of the nesting mechanic already abstractly described). |
| G2 | MISSING | Low | Dispatcher defines generic `url_get`/`url_post`/`url_del` + untyped matcher helpers; method-agnostic. | Spec implied creation-only was a routing limitation; no mention of GET/DELETE matcher capability. | FS-043.3 new bullet + KL-440 — creation-only is a function of *which* routes are mounted, not a framework limit. Overview updated. |
| G3 | MISSING | Low | Dispatcher is lazy-loadable (URL file `include`d on first resolve, then file ref cleared → loads once). | Not documented. | FS-043.3 new lazy-load bullet; Overview note. |
| G4 | MISSING | Med | Email format runs a distinct **reply-detection branch**: `ticketId`→`postMessage`; else header-thread `lookupByEmailHeaders`→`postEmail`; else create new. All →201. | FS-043.4 said email "parsed as RFC-822 (see FS-041)"; the threading-before-create branch was not specified. | New **FS-043.14** (Email-Format Reply Detection & Default Fill-In) + EC-447 + BS-439; FS-043.4 controller-branching bullet. |
| G5 | MISSING | Med | `ApiEmailDataParser::fixup` forces `source=Email`; `message`←subject/`-`; `subject`←`[No Subject]`; `emailId`←default; strips `priorityId` when email-priority disabled. | Not documented (only XML/JSON fixups were). | FS-043.4 email bullet + FS-043.14 acceptance criteria. |
| G6 | MISSING | Med | `scp/apikeys.php` boots via `admin.inc.php` — admin gate enforced at the **entry point**, in addition to the per-template `die('Access Denied')`. | FS-043.2 mentioned only the template-level `die`. | FS-043.2 reworded to a two-layer gate (entry bootstrap + template guard). |
| G7 | MISSING | Low | Bad `id` request param → "Unknown or invalid API key ID."; unknown mass action → "Unknown action - get technical help"; unknown command → "Unknown action/command". | These exact messages/branches not listed. | FS-043.2 new bullets (unknown-id + unknown-action/command). |
| G8 | IMPRECISE | Low | Enable/Disable = single bulk SQL UPDATE compared against selection count; Delete = per-id lookup+delete loop counting successes. | FS-043.2 stated outcomes but not the enable/disable-bulk vs delete-loop mechanics. | FS-043.2 "Bulk action mechanics" bullet. |
| G9 | MISSING | Med | `PipeApiController` (local mail pipe): reuses reply-detection, **no API key**, maps HTTP→MTA exit codes (201→0, 400→66, 401/403→77, 415/416/417/501→65, 503→69, 500/else→75); failure → 416 "Request failed - retry again!". | Pipe path not documented in this spec (only referenced as FS-041). The API-level response-code mapping specializes the API controller and belonged here. | New **FS-043.15** + BS-440 + EC-449. |
| G10 | MISSING | Low | `getApiKey()` only attempts lookup when **both** `HTTP_X_API_KEY` and `REMOTE_ADDR` are set; result cached on the controller (looked up at most once). | FS-043.6 didn't state the both-present precondition or the per-request caching. | FS-043.6 new bullet. |
| G11 | IMPRECISE | Low | `getApiKey` query already filters on IP, so the `requireApiKey` re-check primarily contributes the disabled-key rejection (IP redundant). | FS-043.6 implied the IP check is the active gate in `requireApiKey`. | FS-043.6 parenthetical clarification. |
| G12 | MISSING | Low | `getRequest` returns 400 "Unable to read request body" when the input/stdin stream can't be opened. | Not documented. | FS-043.4 bullet + EC-448. |
| G13 | INCORRECT (bug) | Low | JSON `data:` attachment with `charset=` hint computes a UTF-8 copy into a local `$contents` that is never written back into `$info['data']` — charset re-encode is a no-op despite the inline comment. | FS-043.4 json bullet claimed "non-UTF-8 charsets are converted to UTF-8 for storage." | FS-043.4 json bullet left as the *intent*; added **KL-438** documenting the dead-write so the divergence is recorded (code authoritative). |
| G14 | MISSING | Low | `api/cron.php` includes `api.inc.php` and `@chdir` **twice** before `LocalCronApiController::call()`. | Not documented. | KL-439. |
| G15 | IMPRECISE | Low | `create()` non-email path: missing ticket → 500 "Unable to create new ticket: unknown error"; validation errors → 400 "...validation errors:" + flattened list. | FS-043.4 mentioned 400/500 generically; exact strings/branch not given. | FS-043.4 new error-branch bullet (403 filter case already in EC-439). |

## Verified correct on re-read (no change)
- BS-432 overdue SQL: confirmed `T2.isactive=1` join, `grace_period*3600`, `LIMIT 50`, three OR
  branches, `markOverdue` + `logActivity('Ticket Marked Overdue', ...)`. Spec accurate; the
  unimplemented escalation TODO is already KL-434.
- `Http::header_code_verbose` enum set + `Http::response` default `text/html; charset=UTF-8`,
  Content-Length / Connection: Close. FS-043.12 accurate.
- AJAX base (`staffOnly` 401 "Access Denied. IP <ip>", `json_encode`/`encode`, `get`). FS-043.13
  accurate.
- Autocron: `$caller` captured before `$thisstaff=null`; `lastcroncall` only stamped when work runs;
  180s throttle; ignore_user_abort + finish_request. FS-043.11 accurate.

## Net spec deltas
- New FRs: **FS-043.14**, **FS-043.15** (was 13 → now 15).
- New BS: **BS-439**, **BS-440** (was BS-430..438 → +2).
- New EC: **EC-447, EC-448, EC-449**.
- New KL: **KL-438, KL-439, KL-440**.
- Plus in-place precision edits to FS-043.2, .3, .4, .6 and the Overview.
