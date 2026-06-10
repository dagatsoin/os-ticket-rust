# FS-021: Staff Ticket View & Workflow

## Overview

The Staff Ticket View & Workflow specification defines everything a staff member does to an **individual ticket** inside the staff control panel: viewing the full ticket record and its conversation thread, posting replies to the requester, posting internal notes, assigning/reassigning to staff or teams, claiming a ticket, releasing (unassigning), transferring between departments, closing/reopening, flagging overdue/answered/unanswered, banning/unbanning the requester's email, editing ticket properties, deleting, setting/computing due dates and SLA overdue marking, collaborative-edit locking, staff-initiated (phone) ticket creation, and printing the ticket to PDF. It also covers the alert e-mails fired by each workflow event and the AJAX endpoints that support inline ticket preview and lock acquisition/renewal.

This specification owns the single-ticket request handler (the POST action dispatcher in `scp/tickets.php`), the ticket-view, ticket-edit and ticket-open view templates, the `Ticket` domain class workflow methods (`postReply`, `postMessage`, `postNote`, `assign`, `transfer`, `close`, `reopen`, `markOverdue`, `clearOverdue`, `update`, `delete`, `open`, `create` for the staff path), the `Thread` / `ThreadEntry` / `Message` / `Response` / `Note` thread model, the `TicketLock` lock helper, and the `Ticket2PDF` exporter.

**Scope boundaries** (owned elsewhere, referenced here):
- The ticket **queue / listing / dashboard / search / mass-process listing** and per-status submenus belong to **FS-020**. Only the per-ticket actions reachable from `scp/tickets.php` are in scope; the mass-process action handler is documented here only because it shares the same entry script (see FS-021.18).
- **Ticket creation via the public web form** belongs to **FS-011**; via **inbound e-mail** to **FS-041**; via the **external API** to **FS-043**. Only the **staff-initiated (phone/other) creation path** (`Ticket::open()` → `Ticket::create($vars,…,'staff',…)`) is owned here.
- **Canned responses** and **attachment storage/serving** belong to **FS-022**; this spec references how a reply consumes a canned response and uploads attachments.
- **E-mail templates, outbound mailer, and the variable replacer** belong to **FS-040**; this spec lists which template each event uses and which alert config flags gate it, but does not own the template content.
- **Departments, teams, help topics, SLA plans, priorities** (the lookup objects and their defaults) belong to **FS-030 / FS-032**.
- **Staff authentication, the group permission flags themselves, and department access** belong to **FS-002 / FS-031**. This spec consumes the permission predicates (`canPostReply`, `canCloseTickets`, etc.) and the `checkStaffAccess` visibility rule.
- **Enum value sets, table/column schemas** belong canonically to **FS-091** (ticket status, thread type, source, event state); referenced, not restated.

---

## Functional Requirements

### FS-021.1: Ticket Lookup, Access Gate & Page Routing

**Description**: The system shall resolve the requested ticket, verify the acting staff member's access, and route to the appropriate per-ticket view, edit, print, or new-ticket form.

**Acceptance Criteria**:
- The entry script (`scp/tickets.php`) requires staff authentication (`staff.inc.php` bootstrap) before any processing.
- When a ticket id is supplied (`id` request parameter), the system looks it up by **internal ticket id** (`Ticket::lookup`, numeric local id only — not the external ticket number). On miss it sets error **"Unknown or invalid ticket ID"** and renders no ticket.
- When the ticket exists but the staff member fails the access check (FS-021.2), it sets error **"Access denied. Contact admin if you believe this is in error"** and clears the ticket object.
- After access is established, a POST body (with no pre-existing errors) is dispatched to the per-action handlers (FS-021.3+).
- After POST processing, if a ticket object still exists it is reloaded to reflect changes, and the acting staff's quick-stats are reset (`resetStats()`) only when there were no errors.
- An unknown top-level action errors **"Unknown action"**; an empty `process` sub-action errors **"You must select action to perform"**.
- The page renders the staff quick-stats submenus and their count-threshold warnings (e.g. the "My Tickets > 10" and "overdue > 10" notices); the submenu construction, listing, and quick-stat warning text are owned by **FS-020** (FS-020.2 / FS-020.11). This spec only triggers the render off the shared entry script.
- View routing (GET): when a ticket is resolved, the page renders `ticket-view.inc.php` and the page title becomes **"Ticket #" + external number**; when `a=edit` and the staff can edit, it renders `ticket-edit.inc.php`; when `a=print`, it triggers PDF export (FS-021.17). When no ticket is resolved, it renders the listing (`tickets.inc.php`, FS-020) or, when `a=open` and the staff can create, the new-ticket form `ticket-open.inc.php`.
- When no ticket is open, no action is queued, and no request action is set, and the staff has a configured auto-refresh rate, the page injects a meta-refresh header at `rate × 60` seconds.
- The same listing branch also serves a CSV **export** action: when no ticket is resolved and `a=export`, the system requires a search-query token (`h` request parameter, keyed `search_<token>` in session) held in session. The export error strings, the export filename, and the CSV mechanics are owned by **FS-020.10** (strings/filename per FS-090.23 / BS-090.12). This spec owns only the queue-side trigger and session query-token storage off the shared script.
- A `search` request with no `status` clears the active submenu (search overlay handled by the AJAX search endpoint, FS-020).

### FS-021.2: Staff Access Check (`checkStaffAccess`)

**Description**: The system shall grant a staff member access to a ticket based on department access or direct/team assignment.

**Acceptance Criteria**:
- Access is granted when the staff member is **not** restricted to assigned-only AND has access to the ticket's department.
- For assigned-only staff, or staff without department access, access is granted only when the ticket is **open** AND (the staff is the directly-assigned staff member OR the ticket is assigned to a team the staff belongs to).
- A closed ticket is reachable only via department access (assignment-based access does not apply once closed).
- The access check is re-evaluated after any action that can move the ticket out of the staff's reach (transfer, assignment, edit/dept change, reply-that-closes); if access is lost the ticket object is cleared and the staff is returned to the listing.

  > See [FS-002] for the underlying group/department permission and assigned-only flags consumed by this check.

### FS-021.3: Post Reply to Requester (`reply` → `postReply`)

**Description**: The system shall let a permitted staff member post a public response that is appended to the thread and optionally e-mailed to the requester.

**Acceptance Criteria**:
- Requires the `canPostReply` permission; otherwise error **"Action denied. Contact admin for access"**.
- A non-empty response body is required (field error **"Response required"**).
- The action is blocked when the ticket is locked by a **different** staff member (error **"Action Denied. Ticket is locked by someone else!"**).
- The action is blocked when the requester's e-mail is in the banlist (error **"Email is in banlist. Must be removed to reply."**).
- The response is stored as a thread entry of type Response (`R`), with poster and staff id defaulting to the acting staff when not supplied.
- An **"Email Reply"** checkbox controls whether the requester is e-mailed; when checked, the reply is sent using the department's (or default) reply template and e-mail account, honoring the chosen signature option (FS-021.6), reply-separator quoting, and attaching files when attachment-emailing is enabled.
- A **ticket-status checkbox** lets the reply also set status: **"Close on Reply"** (open tickets, requires `canCloseTickets`) or **"Reopen on Reply"** (closed tickets). The checkbox field is `reply_ticket_status` and its submitted value is the literal `Closed` (close) or `Open` (reopen); status is applied via `setStatus()`, which routes `open`→`reopen()` and `closed`→`close()` and is a no-op when the requested status equals the current one. The status row is rendered whenever the ticket is closed OR the staff can close tickets.
- Posting a response marks the ticket **answered** (`isanswered=1`), sets `lastresponse`/`updated` timestamps (`onResponse`). The reply reloads the ticket before any e-mail is composed.
- The status-on-reply transition happens **before** `onResponse`, so a close-on-reply still leaves the answered flag set on the just-closed ticket.
- If the reply closed an open ticket, the staff is returned to the listing (the in-view ticket object is cleared). On a generic failure with no field error, the dispatcher sets **"Unable to post the reply. Correct the errors below and try again!"**.
- Optional file attachments uploaded with the reply are saved to the thread entry; a canned-response selector can prefill the body, accompanied by an **"Append"** checkbox (default checked) controlling whether the canned text appends to or replaces the current body (canned mechanics owned by FS-022).
- The **"Email Reply"** checkbox is **pre-checked by default** on the reply form (checked unless the operator previously unchecked it on a failed submit).

### FS-021.4: Post Internal Note (`postnote` → `postNote`)

**Description**: The system shall let any staff member with ticket access attach a private internal note, optionally changing ticket state.

**Acceptance Criteria**:
- A non-empty note body is required (field error **"Note required"**); a note title (summary) is optional.
- An optional **state change** dropdown may accompany the note. State permission gates:
  - `closed` requires `canCloseTickets` (else **"You don't have permission to close tickets"**).
  - `overdue`, `notdue`, `unassigned` require the staff to be the ticket's **department manager** (else **"You don't have permission to set the state"**).
  - `answered` / `unanswered` are offered without an additional manager gate in the note form but are only shown for open tickets.
- The note is stored as a thread entry of type Note (`N`); poster is the acting staff (or `SYSTEM` when posted programmatically).
- When a state value is supplied and applied, the ticket is reloaded; a state-change failure is non-critical (the note still posts). The note-form state dropdown values are `closed`, `open` (labeled "Reopen Ticket"), `answered`, `unanswered`, `overdue`, `notdue`, and `unassigned`.
- The note posts even when the state dropdown is `unchanged` (empty value); the note body itself is always required.
- On a generic note-post failure with no specific error, the dispatcher sets **"Unable to post internal note - missing or invalid data."** plus a field hint.
- If the note closed an open ticket, the staff is returned to the listing.
- Optional attachments may be uploaded with the note.
- Internal notes are never e-mailed to the requester; they may trigger **staff** note-alert e-mails per config (FS-021.16).

### FS-021.5: Posting a Requester Message (`postMessage`) — system path

**Description**: The system shall append a requester-originated message to the thread, used by the creation and inbound-email paths, not by direct staff UI POST.

**Acceptance Criteria**:
- A message is stored as a thread entry of type Message (`M`) with `staff_id = 0`.
- On a new message the ticket is marked **unanswered**, `lastmessage` is set, and (when configured) the ticket is auto-assigned to the last respondent if the current assignee is unavailable, else unassigned.
- A message on a **closed** ticket reopens it (unless the message is an auto-response, which must not reopen).
- This requirement is documented here because `postMessage` is part of the Ticket workflow spine; its inbound-email trigger is owned by **FS-041** and its web/API triggers by **FS-011 / FS-043**.

### FS-021.6: Reply Signature Selection

**Description**: When e-mailing a reply (or staff-new-ticket notice), the system shall append the signature the staff member selects.

**Acceptance Criteria**:
- Three options are offered: **None** (default), **My signature** (the staff member's personal signature, value `mine`, shown only if set), and **Dept. Signature** (value `dept`, shown only when the department permits signature append and is public).
- The reply form pre-selects the staff member's configured default signature type.
- Resolution: `mine` → staff signature; `dept` → department signature only when the department is public; otherwise empty.

### FS-021.7: Assign / Reassign Ticket (`assign` → `assign`)

**Description**: The system shall let a permitted staff member assign or reassign a ticket to a staff member or a team.

**Acceptance Criteria**:
- Requires `canAssignTickets`; otherwise error **"Action Denied. You are not allowed to assign/reassign tickets."**.
- The assignee id is prefixed: `s<id>` = staff member, `t<id>` = team; a bare numeric id equal to the acting staff's own id is treated as a **claim** (self-assignment).
- An assignee must be selected (**"Select assignee"**); an invalid prefix yields **"Invalid assignee ID - get technical support"**.
- Re-selecting the already-assigned staff/team is rejected (**"Ticket already assigned to the staff."** / **"…to the team."**).
- Assignment comments are required and must be at least 5 characters (**"Assignment comments required"** / **"Comment too short"**); on a claim, comments are optional and default to **"Ticket claimed by <name>"**.
- Assigning a **closed** ticket reopens it (assigned tickets must be open).
- Staff assignment (`s`) sets the ticket's staff id; team assignment (`t`) sets the team id and, if the ticket was closed, clears the staff id (staff_id is overloaded as assignee-and-closer).
- On a non-claim assignment, the acting staff's locks on the ticket are removed and the staff returns to the listing; on a claim, the message is **"Ticket is NOW assigned to you!"** and the staff stays on the ticket.
- An assignment logs an internal note (no requester e-mail) and fires assignment alert e-mails to the assignee/team per config (FS-021.16). **No alert is sent on self-assignment (claim).**

### FS-021.8: Claim Ticket (`process/claim` and reply-form claim)

**Description**: The system shall let a permitted staff member self-assign an open, unassigned ticket.

**Acceptance Criteria**:
- Requires `canAssignTickets`; otherwise **"Perm. Denied. You are not allowed to assign/claim tickets."**.
- Only **open** tickets may be claimed (**"Only open tickets can be assigned"**); an already-assigned ticket is rejected (**"Ticket is already assigned to <assignee>"**).
- A successful claim assigns the ticket to the acting staff with note **"Ticket claimed by <name>"**, no alert, and message **"Ticket is now assigned to you!"**.

### FS-021.9: Release / Unassign Ticket (`process/release` → `release`/`unassign`)

**Description**: The system shall let a department manager unassign an open ticket from its current assignee(s).

**Acceptance Criteria**:
- Offered only for open, assigned tickets to the ticket's department manager.
- An unassigned ticket is rejected (**"Ticket is not assigned!"**).
- Release clears the staff id and/or team id; a closed ticket cannot be released (`unassign` returns false on a closed ticket). Releasing an unassigned ticket is a no-op success.
- A successful release logs an activity note **"Ticket released (unassigned) from <assignee> by <name>"** (the activity title is "Ticket unassigned").
- Release is reachable two ways: the dedicated process action `process/release` (which calls `release()`/`unassign()` directly), and the note-form state value `unassigned`. **The note-form path is currently inert** because the internal `setState` switch tests for the misspelled state `unassined` (a source typo), so `state=unassigned` falls through `setState` without unassigning; the dedicated release action is the working path (see KL-021.9).

### FS-021.10: Transfer Between Departments (`transfer` → `transfer`)

**Description**: The system shall let a permitted staff member move a ticket to a different department, re-evaluating SLA and access.

**Acceptance Criteria**:
- Requires `canTransferTickets`; otherwise **"Action Denied. You are not allowed to transfer tickets."**.
- A target department must be selected (**"Select department"**), must differ from the current one (**"Ticket already in the department"**), and must exist (**"Unknown or invalid department"**).
- Transfer comments are required and must be at least 5 characters (**"Transfer comments required"** / **"Transfer comments too short!"**).
- Transfer changes the ticket's department, **reopens it if closed**, and re-selects the SLA when the current SLA is empty or transient — applying the new department's SLA (so the overdue window reflects the new owner; see FS-021.13).
- Transfer logs an internal note titled **"Ticket transfered from <old> to <new>"**, records a `transferred` lifecycle event, and fires transfer alert e-mails per config (FS-021.16).
- After transfer, staff access is re-checked; if the acting staff loses access to the new department they are returned to the listing. Success message: **"Ticket transferred successfully to <dept>"**.

### FS-021.11: Close Ticket (`process/close` → `close`)

**Description**: The system shall let a permitted staff member close an open ticket.

**Acceptance Criteria**:
- Requires `canCloseTickets`; otherwise **"Perm. Denied. You are not allowed to close tickets."**.
- An already-closed ticket is rejected (**"Ticket is already closed!"**).
- Closing sets status to closed, stamps the close timestamp, **clears the overdue flag and any due date**, and credits the closing staff (sets `staff_id` to the acting staff). A `closed` lifecycle event is recorded.
- An internal note is logged with the operator-supplied reason or **"Ticket closed (without comments)"**.
- The acting staff's locks are removed and they are returned to the listing. Success message: **"Ticket #<number> status set to CLOSED"**.

### FS-021.12: Reopen Ticket (`process/reopen` → `reopen`)

**Description**: The system shall let a permitted staff member reopen a closed ticket.

**Acceptance Criteria**:
- Requires `canCloseTickets` **OR** `canCreateTickets` (the system assumes either implies reopen authority); otherwise **"Perm. Denied. You are not allowed to reopen tickets."**.
- An already-open ticket is rejected (**"Ticket is already open!"**).
- Reopening sets status to open, stamps `reopened`, sets answered state to 0 (unanswered by default), records a `reopened` lifecycle event, and **annuls** any prior `closed` event (for statistics).
- An internal note is logged with the operator reason or **"Ticket reopened (without comments)"**. Success message: **"Ticket REOPENED"**.

### FS-021.13: Due Date, SLA Selection & Overdue Marking

**Description**: The system shall compute when a ticket is overdue from an explicit due date or the SLA grace period, mark/clear the overdue flag, and apply the correct SLA on department/topic changes.

**Acceptance Criteria**:
- The **estimated due-date / SLA-grace math** (explicit due date else `created + SLA grace_period hours`, reopened-aware) is owned by **FS-032.11**; this spec consumes the resulting effective due date. An operator may override SLA timing per ticket via an explicit due date (the per-ticket workflow framing kept here).
- **SLA selection precedence is canonical here** (`selectSLAId`, the runtime resolver): an explicit trump value (e.g. from a filter) > the **department's** SLA > the help **topic's** SLA > the **system default** SLA — i.e. department is evaluated **before** topic. This is the single full precedence statement; FS-030 and FS-091.6 cross-ref this requirement (D03X-10 / Conflict 2).
- **Manual mark overdue** (`process/overdue`): offered only to the ticket's department manager (**"Perm. Denied. You are not allowed to flag tickets overdue"**). Sets the overdue flag, records an `overdue` event, fires overdue alerts (FS-021.16), and logs activity **"Ticket flagged as overdue by <name>"**.
- **Clear overdue** (note-form `notdue`, manager only): clears the overdue flag; additionally nulls the due date if it is in the past and zeroes the SLA if the SLA due date is in the past. A previously-logged overdue event is **not** annulled.
- **System overdue sweep** (`checkOverdue`): marks newly-overdue open tickets and logs **"Ticket flagged as overdue by the system."**. The sweep **cadence, the 50-per-run cap, and the eligibility (reopened-aware grace) qualification math** are owned by **FS-043 BS-432** (cadence/cap) and **FS-032.11** (due-date/grace math); this spec owns only the `markOverdue`/`checkOverdue` action and its activity note.
- Editing a due date / SLA that puts the est. due date in the future (or clears it) clears the overdue flag automatically (FS-021.15).
- Manager-only **mark answered / mark unanswered** flags (`process/answered`, `process/unanswered`) set the answered state and log activity; **"Ticket flagged as answered"** / **"…unanswered"**.
- `markOverdue` is idempotent: it returns success immediately if the ticket is already overdue (no event logged, no alert fired). `markAnswered` / `markUnAnswered` likewise short-circuit to success when the answered flag already matches, so no redundant DB write or activity occurs.
- The manual mark-overdue activity note text is **"Ticket flagged as overdue by <name>"** (process action) vs. **"Ticket flagged as overdue by the system."** (sweep); both share the activity title "Ticket Marked Overdue".

### FS-021.14: Ban / Unban Requester E-mail (`process/banemail` / `unbanemail`)

**Description**: The system shall let a permitted staff member add or remove the ticket requester's e-mail from the banlist.

**Acceptance Criteria**:
- Both actions require `canBanEmails` (else **"Perm. Denied. You are not allowed to ban emails"** / **"…remove emails from banlist."**).
- The ban/unban mechanics, the already-banned / added / removed / not-in-banlist result messages, and the "only addresses with an explicit editable banlist entry are removable" rule are owned by **FS-042.12**; the reply-block enforcement and the banner-vs-enforcement wording divergence are owned by **FS-042.14**.
- Workflow consequence (owned here): while the requester's e-mail is banned, the reply form is blocked and a banner warns the e-mail must be removed before replying (see FS-021.3, EC-021.3).

  > Banlist storage, ban/unban actions, and inbound-filter rejection are owned by **FS-042**.

### FS-021.15: Edit Ticket Properties (`a=edit` → `update`)

**Description**: The system shall let a permitted staff member edit a ticket's requester details, routing, priority, SLA, source, and due date, with a mandatory reason note.

**Acceptance Criteria**:
- Requires `canEditTickets`; otherwise **"Perm. Denied. You are not allowed to edit tickets"**.
- Required fields with validation: name (required), valid e-mail (required), subject (required), help topic (int, required), priority (int, required), and a **reason-for-update note** (required, **"Reason for the update required"**). Optional: SLA, phone (validated), phone extension (numeric, requires a phone number), source, due date.
- **Due date rules**: cannot be set on a closed ticket (**"Due date can NOT be set on a closed ticket"**); requires a time (**"Select time"**); must parse (**"Invalid due date"**) and be in the future (**"Due date must be in the future"**). Setting a due date clears the overdue flag.
- After a successful update the ticket reloads, the SLA is re-selected if empty/transient, and the overdue flag is cleared automatically when the new est. due date is in the future or absent.
- An internal note **"Ticket Updated"** is logged with the supplied reason (or a generated one). On dept change, staff access is re-checked. Success message: **"Ticket updated successfully"**.
- Source values offered in edit: Phone, Email, Web, API, Other.

### FS-021.16: Workflow Event Alert E-mails

**Description**: The system shall fire e-mail notifications to staff/admin on workflow events, each gated by global alert config flags and a per-event recipient policy.

**Acceptance Criteria**:
- **New ticket** (`onNewTicket`): auto-response to requester (when enabled per config + department + help topic), and a new-ticket **alert** to admin (when enabled), department members (only when the ticket is unassigned), and the department manager — per the corresponding `alert…ONNewTicket` flags.
- **New message** (`onMessage` / `postMessage`): new-message alert to the last respondent and/or assigned staff/team and/or department manager, per flags; auto-response to requester per config.
- **New response/reply** (`postReply`): the requester is e-mailed the reply (when "Email Reply" is checked) using the reply template — this is the only event that e-mails the requester directly on a staff action.
- **New note** (`postNote`): note alert to last respondent / assigned staff / team / department manager per flags; the note poster is never alerted, vacationing staff are skipped, duplicates suppressed, and on a closed ticket only staff who still have access are alerted.
- **Assignment** (`onAssign`): alert to the assigned staff, or to team members / team lead per flags; never on self-assignment.
- **Transfer** (`transfer`): alert to assigned staff/team or department members (unassigned only) and/or the department manager per flags.
- **Overdue** (`onOverdue`): alert to assigned staff/team or department members (unassigned only) and/or the department manager per flags, suppressed when the SLA disables overdue alerting.
- **Open-limit reached** (`onOpenLimit`): over-limit notice to requester (when enabled) plus an admin warning (the admin warning has no disable flag).
- **Auto-response suppression**: the new-ticket / new-message auto-response is force-disabled when (a) the requester address matches one of the system's own e-mail accounts (loop control), (b) the inbound message is detected as an auto-response (by its e-mail headers), (c) the requester address begins with `mailer-daemon@` or `postmaster@`, or (d) a canned auto-response was posted (which disables the standard new-ticket auto-response and leaves the ticket unanswered).
- **New-message auto-assignment**: on a new message, if the current assignee is unavailable, the ticket auto-assigns to the last respondent **only when** the auto-assign-reopened-tickets config flag is enabled and that respondent is available; otherwise it is unassigned. An auto-response message must not reopen a closed ticket.
- Every recipient loop deduplicates by e-mail, skips unavailable (vacationing) staff, and substitutes the recipient's first name into the template body via the `%{recipient}` token (admin recipients get the literal "Admin").

  > Template content, the outbound mailer, and the `%{…}` variable replacer are owned by **FS-040**. The specific config flag names are enumerated in **FS-032 / FS-091**.

### FS-021.17: Print Ticket to PDF (`a=print` → `pdfExport` / `Ticket2PDF`)

**Description**: The system shall render the ticket header and thread as a downloadable PDF, optionally including internal notes, in a selectable paper size.

**Acceptance Criteria**:
- Print is offered from the ticket view with two presets — **Ticket Thread** (notes excluded) and **Thread + Internal Notes** (notes included) — and a print-options dialog letting the operator toggle notes and pick paper size.
- Supported paper sizes are **Letter, Legal, A4, A3** — verified against the print-options dialog option list in `ticket-view.inc.php` and the bundled FPDF page formats (`a3, a4, a5, letter, legal`); no `Ledger`/`Tabloid` size exists in source (see paper-size note in Dependencies / cf. FS-091.11). The selection is remembered in session and pre-selected next time, falling back to the staff's default paper size, then `Letter`. When the print action supplies no explicit (string) paper size, the same session → staff-default → `Letter` fallback chain resolves it; the resolved size is then written back to the session for next time.
- The print action is reachable by GET (`a=print` with `notes` and `psize` parameters from the preset links) and by POST (the print-options dialog form). The PDF streams inline (filename `Ticket-<number>.pdf`) and terminates the request.
- The PDF header carries the configured site title, the current date/time with GMT offset, and a logo (the configured client logo, else a bundled default).
- The PDF body renders the ticket facts (status, name, priority, e-mail, department, phone, create date, source+IP, assignee/closed-by, help topic, SLA, last response, due/close date, last message) followed by each thread entry (message / response / and notes when included), color-coded by type, listing attachment filenames.
- The footer shows `Ticket #<number> printed by <username> on <date>` and a page number.
- A PDF export failure surfaces **"Internal error: Unable to export the ticket to PDF for print."**.

### FS-021.18: Collaborative Edit Locking (`TicketLock`, auto-lock, AJAX renew/release)

**Description**: The system shall lock a ticket to one staff member while they view it, with a configurable TTL, automatic acquisition/renewal, and takeover of expired locks.

**Acceptance Criteria**:
- When lock-time is configured, opening the ticket view auto-acquires a lock for the acting staff (or renews their existing one); failure surfaces warning **"Unable to obtain a lock on the ticket"**.
- A lock has a TTL of the configured lock-time in minutes; `acquire` first deletes any expired locks on the ticket, then inserts a new lock (insert-if-absent) expiring `now + lockTime` minutes. If a non-expired lock already exists and is owned by the requesting staff, `acquireLock` renews it in place rather than inserting; if owned by another, it returns no lock.
- `renew` with no explicit lock-time re-extends the lock by its original window (the minute span between its prior create and expire timestamps), not a fresh full lock-time.
- Lock remaining time is computed at load as `expire − now` (seconds); a lock is "expired" when current time exceeds the load-time expire snapshot (no continuous server-side re-validation — see KL-021.2).
- A staff member viewing a ticket locked by **someone else** sees error **"This ticket is currently locked by <name>"** and cannot post a reply (FS-021.3).
- The inline AJAX **preview** (`previewTicket`) requires the staff to pass the access check and otherwise responds HTTP 404 **"No such ticket"**. Within the preview it surfaces at most one banner, in precedence order: a lock note, else an "Marked overdue!" note. (Note: the preview's lock banner is rendered when the lock is owned by the **current** viewer — i.e. it reads "Ticket is locked by <name>" using the lock owner's name even though the viewer holds it — a source quirk; see KL-021.10.)
- A lock auto-acquired on the view emits a client-side `autoLock.setLock(...,'acquire')` directive seeded with the current lock id and the configured lock time so the browser begins renew polling.
- The browser periodically renews the lock via AJAX (`renewLock`): if the lock is gone/expired it re-acquires; if owned by another staff it gives up (no retry); otherwise it renews and returns remaining time.
- `acquireLock` (AJAX) returns the lock id and remaining time, denies when the staff lacks ticket access, denies when the ticket is locked by another non-expired owner, and signals retry on transient failure.
- `releaseLock` (AJAX) releases a specific lock only when the requester owns it, or releases **all** of the requester's locks on the ticket when no lock id is given.
- Locks are removed when the owner assigns away, closes, or otherwise leaves the ticket; a cron sweep (`cleanup`) deletes all expired locks and optimizes the table.

### FS-021.19: Delete Ticket (`process/delete` → `delete`)

**Description**: The system shall let a permitted staff member permanently delete a ticket and its thread/attachments.

**Acceptance Criteria**:
- Requires `canDeleteTickets`; otherwise **"Perm. Denied. You are not allowed to DELETE tickets!!"**.
- Deletion removes the ticket row, then deletes the orphaned thread entries and their attachments (orphaned attachment files are purged).
- A debug log entry records who deleted which ticket. Success message: **"Ticket #<number> deleted successfully"**.
- The action is **irreversible**; the confirmation dialog warns deleted tickets and attachments cannot be recovered.

### FS-021.20: Staff-Initiated (Phone) New Ticket (`a=open` → `Ticket::open`)

**Description**: The system shall let a permitted staff member open a new ticket on a requester's behalf (e.g. a phone call), optionally with an immediate response, assignment, and user notification.

**Acceptance Criteria**:
- Requires `canCreateTickets`; otherwise **"You do not have permission to create tickets…"**.
- The form collects requester e-mail+name (required) and phone, a **source** (Phone / Email / Other), department (required), help topic (required), priority, SLA, due date, optional assignee (only if the staff can assign), a subject and **issue** summary (the initial message body, required), an optional response, and an optional internal note.
- Creation runs the shared `Ticket::create(..., origin='staff', autorespond=false)` path: it trims whitespace on email/phone/subject/name, rejects banned requester e-mails up front (**"Ticket denied. Error #403"**, `errno=403`, logged as a warning), validates fields, runs inbound filters (filter rejection → Error #403, logging the rejecting filter's name), maps department/priority/SLA/auto-assignment from the help topic (or from the inbound e-mail account for `email` origin), generates an external ticket id, posts the issue as the initial Message, applies the SLA, and assigns staff/team when chosen.
- The **max-open-tickets** limit is enforced **only for non-staff origins**: a staff-created ticket bypasses the per-requester open-ticket ceiling (the limit is skipped when `origin == 'staff'`). After creation, if the requester just hit the ceiling, an over-limit notice/alert fires (FS-021.16) — but the user notice is suppressed for the staff origin.
- The external ticket id is a random number of fixed length when random ids are enabled, regenerated on collision; when sequential ids are configured the external id is overwritten post-insert with the auto-increment internal id.
- Help-topic mapping precedence: an explicitly supplied dept/priority/SLA/staff/team wins; otherwise the help topic supplies dept id, priority id, SLA id, and an auto-assign staff or team. For `email` origin without an explicit dept, the source e-mail account supplies dept and priority and the source is forced to `Email`. Final fallbacks are the system default priority and default department.
- Source must be one of email/phone/other (else **"Invalid source"**); the issue is required (**"Summary of the issue required"**); a due date (staff origin only) is validated like FS-021.15.
- If a response is entered and the staff can reply, it is posted (optionally **Close on Response** when the staff can close); a canned response may prefill it.
- When no assignment and an internal note is given, the note is logged; otherwise a "New Ticket by Staff" activity note is logged.
- A **"Send alert to user"** option (shown only when staff-new-ticket notification is enabled) e-mails the requester a new-ticket notice using the reply/notice template and chosen signature.
- On success: **"Ticket created successfully"**; if the staff loses access to the new ticket (e.g. department) or it is already closed, they return to the listing.

  > Web/email/API creation paths are owned by **FS-011 / FS-041 / FS-043**; only the staff path is in scope here.

### FS-021.21: Mass Ticket Actions From the View Script (`mass_process`)

**Description**: The system shall process bulk actions (reopen / close / mark-overdue / delete) submitted from the listing through the same entry script.

**Acceptance Criteria**:
- Requires `canManageTickets` (else **"You do not have permission to mass manage tickets…"**); at least one ticket must be selected (else **"No tickets selected…"**).
- **reopen** requires close-or-create permission and only affects currently-closed tickets; **close** requires close permission and only affects open tickets; **mark_overdue** affects not-yet-overdue tickets; **delete** requires delete permission.
- Each successful item logs the corresponding internal note/activity; a per-batch result message reports `all`, `partial (i of count)`, or `none`.
- This handler shares the script but the **listing UI that submits it is owned by FS-020**; documented here only for completeness of the entry-script action map.

### FS-021.22: Ticket Thread Model (`Thread` / `ThreadEntry` / `Message` / `Response` / `Note`)

**Description**: The system shall represent a ticket's conversation as an ordered set of typed thread entries with attachments and e-mail-threading metadata.

**Acceptance Criteria**:
- Three entry types exist: **Message (M)** — requester-originated; **Response (R)** — staff reply to requester; **Note (N)** — internal note. The "thread count" shown to staff is messages + responses; notes are counted separately (and shown inline only when configured).
- Each entry stores title, body, poster name, staff id (0 for requester/system), source, creation time, and optional parent id (`pid`) for reply threading.
- Each entry may have attachments (joined by ticket id + entry id + entry type) and an e-mail message-id / headers record used to thread inbound replies.
- A thread entry is only created for a recognized type (`M`/`R`/`N`); other types are rejected. Titles and bodies are sanitized/HTML-escaped on save (plain-text effective — see KL-021.7).
- New entries auto-generate a unique e-mail message-id when none is supplied, of the form `<{random-24}@{last-10-of-md5(site-url)}>`, enabling later inbound replies to be matched back to the thread (by References / In-Reply-To headers, then by a `#<number>` token in the subject as a last resort, which additionally requires the sender e-mail to match the ticket owner).
- A reply/note with no explicit parent id but a `reply_to` predecessor entry inherits that entry's id as its parent (`pid`); a response also defaults its parent to the message id (`msgId`) it answers.
- Per-entry attachment uploads tolerate the empty-file case (a no-file upload slot is skipped); failed uploads/imports are logged as `SYSTEM` internal notes ("File Upload Error" / "File Import Error") rather than aborting the post.
- The first and last Message of a ticket are addressable (used for variable substitution like `original` / `last_message`).

  > Inbound e-mail matching (`lookupByEmailHeaders`, `postEmail`) is consumed by **FS-041**; attachment storage by **FS-022**.

### FS-021.23a: Ticket-View Banners, Action Visibility & Reload Affordance

**Description**: The ticket view renders contextual banners and conditionally shows action affordances based on the staff member's permissions, the ticket's state, and lock/ban conditions.

**Acceptance Criteria**:
- The view re-checks `checkStaffAccess` on render and dies with "Access Denied" if the viewer no longer has access (defense in depth on top of the dispatcher gate).
- Exactly one top banner shows, in precedence: error (`errors['err']`) → notice (`msg`) → warning (`warn`). Warnings accumulate: "Unable to obtain a lock…", "Ticket is assigned to <assignee>" (shown when the ticket is assigned to a staff member other than the viewer or a team the viewer is not on), and "Marked overdue!".
- When the ticket is locked by another staff member, the view sets the error **"This ticket is currently locked by <name>"**; when the requester e-mail is banned it sets **"Email is in banlist! Must be removed before any reply/response"** (note: this view-banner wording differs from the dispatcher's reply-block message in FS-021.3).
- The **"More"** action menu is shown only when the staff can ban e-mails OR is the ticket's department manager. Within it, Release / Mark-Overdue (or Clear-Overdue) / Mark-Answered (or Mark-Unanswered) entries appear only for an **open** ticket whose department the viewer manages; the Ban / Unban entry appears per `canBanEmails` and current ban state (Unban shown only when the address is removable from the editable banlist).
- Delete / Close-or-Reopen / Edit / Claim buttons render per their respective permissions and the ticket's open/closed/assigned state (Claim only for an open, unassigned ticket with assign permission).
- The thread header offers a self-link "Ticket #<number>" that reloads the view; thread entry counts shown are messages + responses, with internal notes either inline or in a separate tab per the show-notes-inline config.

### FS-021.23: Lifecycle Event Log (`logEvent`) & Activity Notes (`logActivity`)

**Description**: The system shall record ticket lifecycle events for reporting and optionally mirror operator actions as internal activity notes.

**Acceptance Criteria**:
- `logEvent(state[, annul])` inserts a row capturing ticket/staff/team/dept/topic ids, timestamp, the state string (e.g. `created`, `closed`, `reopened`, `assigned`, `transferred`, `overdue`), and the actor username (or `SYSTEM`). When `annul` is given, prior matching-state events are flagged annulled (e.g. reopen annuls the prior `closed`).
- `logActivity(title, note)` records a `SYSTEM` internal note **only when** ticket-activity logging is enabled in config; it never alerts.

---

## Business Rules

### BS-021.1: Per-Action Permission Gating
**Rule**: Every workflow action is gated by a specific staff group permission flag — reply (`canPostReply`), close (`canCloseTickets`), reopen (`canCloseTickets` OR `canCreateTickets`), assign/claim (`canAssignTickets`), transfer (`canTransferTickets`), edit (`canEditTickets`), delete (`canDeleteTickets`), ban (`canBanEmails`), create (`canCreateTickets`), mass-manage (`canManageTickets`) — and certain state changes (overdue/notdue/answered/unanswered/release) additionally require **department-manager** status on the ticket's department.
**Rationale**: Authorization is enforced server-side at the action dispatcher regardless of which buttons the view chose to render.
**Examples**: A staff member without `canDeleteTickets` who POSTs a delete action is rejected with a permission error even though the Delete button is hidden in their view.

### BS-021.2: Ticket Visibility By Department Access Or Assignment
**Rule**: A staff member may view/act on a ticket only if they have access to its department, or (for open tickets) they are its assigned staff or a member of its assigned team. Assignment-based access does not survive closing.
**Rationale**: Limits exposure to the departments a staff member belongs to while still letting cross-department assignees work their open tickets.
**Examples**: An agent assigned an open ticket outside their department can work it; once it is closed, they can no longer open it unless they have department access.

### BS-021.3: One Active Lock Per Ticket; Locks Block Conflicting Replies
**Rule**: At most one non-expired lock exists per ticket (expired locks are cleared on acquire). A staff member cannot post a reply while another staff member holds a live lock.
**Rationale**: Prevents two agents from double-replying to the same requester simultaneously.
**Examples**: Agent B opening a ticket Agent A is actively viewing sees "currently locked by A" and cannot reply until A's lock expires or is released.

### BS-021.4: Closing Clears Overdue & Due Date and Credits the Closer
**Rule**: Closing a ticket zeroes the overdue flag, nulls the due date, stamps the close time, and sets the ticket's staff id to the closing staff member (staff_id doubles as assignee and closed-by).
**Rationale**: A closed ticket can no longer be overdue, and the closer is recorded for reporting and the "Closed By" display.
**Examples**: An overdue ticket closed by Agent C shows "Closed By: C" and no longer appears in overdue counts.

### BS-021.5: Assignment/Transfer/Message Reopen Closed Tickets
**Rule**: Assigning (staff/team/claim), transferring, and a new requester message all reopen a closed ticket (an auto-response message is exempt from reopening).
**Rationale**: Acting on a closed ticket implies it needs to be open again; auto-responses must not resurrect closed tickets.
**Examples**: Reassigning a closed ticket to a team reopens it and clears the prior closer's staff id.

### BS-021.6: Estimated Due Date = Explicit Due Date Else SLA Grace
**Rule**: A ticket's effective due date is its explicit due date if set; otherwise it derives from the SLA grace period (reopened-aware). The due-date / SLA-grace derivation math is owned by **FS-032.11**; this rule records only that the per-ticket workflow lets an operator override SLA timing via an explicit due date.
**Rationale**: Operators can override SLA timing per ticket; otherwise the SLA grace period governs.
**Examples**: A ticket with a 24-hour SLA and no explicit due date becomes overdue per the FS-032.11 grace computation; setting an explicit future due date defers that.

### BS-021.7: SLA Follows the Owning Department on Transfer
**Rule**: When a ticket is transferred (or edited) and its SLA is empty or transient, the system re-selects the SLA by the precedence canonically stated in FS-021.13 (trump > department > help topic > system default), so the overdue window reflects the new owner.
**Rationale**: A shorter-grace department should make the ticket overdue sooner once it owns it.
**Examples**: Transferring a ticket into a department with a 4-hour SLA recomputes the due date from the 4-hour grace.

### BS-021.8: Assignment & Transfer Require Justifying Comments (≥5 chars); Claims Don't
**Rule**: Assignment and transfer require comments of at least 5 characters; a self-assignment (claim) needs no comment and defaults to "Ticket claimed by <name>".
**Rationale**: Cross-staff hand-offs are auditable; self-claims are self-evident.
**Examples**: A 3-character transfer comment is rejected with "Transfer comments too short!".

### BS-021.9: No Self-Assignment Alerts
**Rule**: Assignment alert e-mails are suppressed when the assignee is the acting staff member (a claim).
**Rationale**: Notifying yourself of an action you just took is noise.
**Examples**: Claiming a ticket logs the assignment note but sends no alert e-mail.

### BS-021.10: Internal Notes Are Never E-mailed to the Requester
**Rule**: Notes (type N) are private; they may generate staff alert e-mails but are never sent to the ticket requester. Only a staff **reply** (type R) with "Email Reply" checked reaches the requester.
**Rationale**: Internal commentary must not leak to customers.
**Examples**: A note documenting an escalation alerts the assigned agent but not the customer.

### BS-021.11: Reply Blocked While Requester E-mail Is Banned
**Rule**: A staff reply cannot be posted while the requester's e-mail is in the banlist; the address must be removed first. The ban/unban mechanics and banlist messages are owned by **FS-042.12 / FS-042.10**; this rule records only the per-ticket reply-block consequence.
**Rationale**: A banned address is one the system should not correspond with.
**Examples**: Replying to a ticket whose requester was banned shows "Email is in banlist. Must be removed to reply." (message wording per FS-042).

### BS-021.12: Alert Recipients Are Deduplicated, Available, and Exclude the Actor
**Rule**: Every alert fan-out skips unavailable (vacationing) staff, suppresses duplicate addresses, excludes the note poster for note alerts, and on closed tickets only alerts staff who retain access.
**Rationale**: Reduces redundant and irrelevant notifications.
**Examples**: If the assigned staff is also the last respondent, they receive one new-message alert, not two.

### BS-021.13: Manual Overdue/Answered/State Flags Are Manager-Only
**Rule**: Directly flagging a ticket overdue, clearing overdue, marking answered/unanswered via the process actions, and releasing via the note form require the acting staff to be the **department manager**; staff with edit permission can still influence overdue indirectly via due date/SLA edits.
**Rationale**: Direct status manipulation is a supervisory action; routine agents change timing only through edits.
**Examples**: A non-manager agent cannot click "Mark as Overdue" but can set a past due date via Edit (subject to the future-date rule) to influence SLA.

### BS-021.14: Reopen Annuls the Prior Close Event
**Rule**: Reopening a ticket records a `reopened` event and annuls the most recent `closed` event for statistics.
**Rationale**: Prevents a reopened-then-closed ticket from inflating closed-event counts.
**Examples**: A ticket closed and reopened twice yields accurate net close statistics because each reopen annuls its preceding close.

### BS-021.15: Staff-Created Tickets Default to No Auto-Response
**Rule**: The staff creation path calls `Ticket::create` with auto-respond disabled; the requester is only e-mailed when the operator explicitly checks "Send alert to user" (and staff-new-ticket notification is enabled).
**Rationale**: A phone-logged ticket should not blast an autoresponder unless the agent opts in.
**Examples**: Logging a phone call without checking the alert box creates the ticket silently.

### BS-021.16: Deletion Is Permanent and Cascades to Thread & Attachments
**Rule**: Deleting a ticket removes the ticket, its thread entries, and their attachment files (orphans purged); there is no recovery.
**Rationale**: Hard delete is the documented behavior; the UI warns accordingly.
**Examples**: A deleted ticket's attachments are removed from content-addressed storage if no other entry references them.

### BS-021.17: System Overdue Sweep — Cadence & Cap Owned by FS-043 BS-432
**Rule**: The automated overdue check (`checkOverdue`) marks newly-overdue open tickets; its per-run cap (50/run, oldest first), cron cadence, and reopened-aware eligibility math are owned by **FS-043 BS-432** (cadence/cap) and **FS-032.11** (grace math). This spec owns only the per-ticket `markOverdue`/`checkOverdue` action and activity note.
**Rationale**: Bounds the per-cron workload (cap/cadence are the cron owner's concern).
**Examples**: With 200 newly-overdue tickets, multiple cron runs are needed to flag them all (run sizing per FS-043 BS-432).

### BS-021.18: Reply Status Checkbox Can Close or Reopen On Reply
**Rule**: Posting a reply may also set status — "Close on Reply" (open tickets, close permission) or "Reopen on Reply" (closed tickets) — applied through the same status transition rules (BS-021.4 / FS-021.12).
**Rationale**: Lets agents answer-and-close (or answer-and-reopen) in one action.
**Examples**: Answering a question and checking "Close on Reply" sends the reply then closes the ticket, returning the agent to the queue.

### BS-021.19: Max-Open-Tickets Limit Excludes Staff-Created Tickets
**Rule**: The per-requester open-ticket ceiling blocks only non-staff origins (web/email/api); a staff member can open a ticket for a requester already at the limit.
**Rationale**: Operators logging legitimate phone/walk-in tickets must not be blocked by an anti-flood control aimed at end users.
**Examples**: A requester with the maximum open tickets is rejected on the public form but can still have a phone ticket logged by staff.

### BS-021.20: Auto-Response Is Suppressed for Loop/Bounce/System Senders
**Rule**: New-ticket / new-message auto-responses are suppressed when the sender is one of the system's own e-mail accounts, the message is itself an auto-response, the sender is a `mailer-daemon@`/`postmaster@` address, or a canned auto-reply was already posted.
**Rationale**: Prevents mail loops and avoids auto-replying to non-human senders.
**Examples**: An out-of-office auto-reply that creates/updates a ticket does not trigger an outbound auto-response.

### BS-021.21: Mark-Overdue / Mark-Answered Are Idempotent
**Rule**: Marking a ticket overdue (or answered/unanswered) is a no-op success when it is already in that state — no event, alert, DB write, or activity note is produced on the redundant call.
**Rationale**: Avoids duplicate overdue events/alerts and redundant statistics.
**Examples**: Clicking "Mark as Overdue" twice fires the overdue alert once.

### BS-021.22: Status-on-Reply Applies Before the Answered Stamp
**Rule**: When a reply both sets status and is posted, the status transition runs first and `onResponse` (answered=1, lastresponse) runs after, so a close-on-reply leaves the closed ticket flagged answered.
**Rationale**: A closed-on-reply ticket should reflect that it was answered at close time.
**Examples**: Answer-and-close yields a closed ticket whose answered flag is set.

### BS-021.23: External Ticket Number Is Random-Unique or Sequential by Config
**Rule**: The external ticket number is a fixed-length random number (regenerated on collision) when random ids are enabled, or the auto-increment internal id when sequential ids are configured (written back after insert).
**Rationale**: Sites can choose unpredictable vs. human-sequential ticket numbers.
**Examples**: With sequential ids enabled, the first ticket's external number equals its internal id.

---

## Data Requirements

> Canonical table/column schemas and enum value sets are owned by **FS-091**. The functional fields each workflow reads/writes are summarized here.

### Ticket Record (workflow-relevant fields)
- Identity: internal id, external ticket number (random or sequential per config), requester name/email/phone/phone-ext, subject, source, IP.
- Routing & classification: department id, help-topic id, priority id, SLA id, assigned staff id (overloaded as closed-by), assigned team id.
- Status & timing: status (open/closed), answered flag, overdue flag, created, reopened, updated, closed, last-message, last-response, explicit due date, derived SLA due date (grace computation owned by FS-032.11), lock id.

### Thread Entry (`ticket_thread`)
- Type (M/R/N), ticket id, parent id (threading), title, body, poster name, staff id (0 = requester/system), source, created, updated, plus a linked e-mail message-id/headers record and attachment join rows.

### Ticket Lock (`ticket_lock`)
- Lock id, ticket id, staff id, created, expire timestamp; remaining time derived as `expire − now`.

### Lifecycle Event (`ticket_event`)
- Ticket/staff/team/dept/topic ids, timestamp, state string, actor username, annulled flag.

### Form/POST Fields Consumed by the Dispatcher
- `id`, `a` (action: reply/transfer/assign/postnote/edit/update/process/open/mass_process/print/search/export), `do` (process sub-action: close/reopen/release/claim/overdue/answered/unanswered/banemail/unbanemail/delete; mass: reopen/close/mark_overdue/delete), `response`, `emailreply`, `append` (canned-response append-vs-replace toggle), `signature`, `reply_ticket_status` (value `Open`/`Closed`), `note`, `title`, `state` (note-form state: closed/open/answered/unanswered/overdue/notdue/unassigned), `ticket_status_notes` (close/reopen reason note), `deptId`, `transfer_comments`, `assignId` (prefixed `s`/`t`/bare-numeric), `assign_comments`, `duedate`, `time`, `slaId`, `priorityId`, `topicId`, `source`, `issue`, `cannedResp`/`cannedResponseId`, `attachments[]`, `tids[]`, `alertuser`, `ticket_state` (staff-create close-on-response), `psize`, `notes` (print: include internal notes), `h` (export query token).

### Source Values (staff context)
- New-ticket form offers Phone / Email / Other; edit form offers Phone / Email / Web / API / Other. (Enum canonical in FS-091.)

---

## User Flows / Interactions

### Flow 1: Reply and Close
1. Agent opens a ticket; the view auto-locks it to them.
2. Agent selects a canned response (optional), edits the body, picks a signature, checks "Email Reply" and "Close on Reply".
3. On submit the reply is appended as a Response, e-mailed to the requester, the ticket is marked answered then closed, the lock is released, and the agent returns to the queue with "Ticket #… status set to CLOSED".

### Flow 2: Transfer to Another Department
1. Agent opens the "Dept. Transfer" form, selects a target department, and enters ≥5-char comments.
2. On submit the ticket moves department, reopens if closed, re-selects the new department's SLA, logs a transfer note + event, and fires transfer alerts.
3. If the agent loses access to the new department they are returned to the queue with the success message.

### Flow 3: Assign / Claim
1. Agent opens "Assign Ticket", picks a staff member or team (or the "Claim Ticket" option), enters comments (optional for a claim).
2. On submit the ticket is assigned, an internal note is logged, alerts fire (except on a claim); a non-claim assignment returns the agent to the queue, a claim keeps them on the ticket.

### Flow 4: Post Internal Note with State Change
1. Agent opens "Post Internal Note", writes the note, and (if permitted) chooses a state (e.g. Close / Mark Answered / Flag Overdue).
2. On submit the note is stored privately, the state is applied (subject to manager gates), and staff note-alerts fire.

### Flow 5: Edit Ticket
1. Agent opens Edit, changes routing/priority/SLA/due date and enters a mandatory reason note.
2. On submit fields are validated (future due date, valid e-mail, etc.), the ticket updates, SLA/overdue are recomputed, an "Ticket Updated" note is logged, and the agent returns to the ticket view.

### Flow 6: Open a Phone Ticket
1. Agent picks "New Ticket", enters requester details, source=Phone, department, topic, the issue summary, and (optionally) an immediate response, assignee, and note.
2. On submit the ticket is created without auto-response (unless "Send alert to user" is checked), the response is posted, and the agent lands on the new ticket (or the queue if access is lost).

### Flow 7: Print to PDF
1. Agent clicks Print and chooses "Thread" or "Thread + Internal Notes" (or opens the dialog to pick paper size).
2. The PDF streams to the browser; the chosen paper size is remembered for next time.

---

## Edge Cases & Error Scenarios

### EC-021.1: Reply While Locked by Another Agent
**Scenario**: Agent B posts a reply while Agent A holds a live lock.
**Expected**: Reply rejected with "Action Denied. Ticket is locked by someone else!"; the view also shows "currently locked by A".

### EC-021.2: Empty Response / Note / Comments
**Scenario**: Required text fields submitted empty or too short.
**Expected**: Field-specific errors ("Response required", "Note required", "Assignment comments required"/"Comment too short", "Transfer comments required"/"…too short!"); nothing is posted.

### EC-021.3: Replying to a Banned Requester
**Scenario**: The requester's e-mail is on the banlist when an agent tries to reply.
**Expected**: Reply blocked with "Email is in banlist. Must be removed to reply."; the view banner instructs removing the ban first.

### EC-021.4: Reply That Closes Removes Ticket From View
**Scenario**: An open ticket is replied-and-closed.
**Expected**: The in-view ticket object is cleared and the agent is returned to the listing (the just-closed ticket is no longer the active view).

### EC-021.5: Transfer to Current Department / Invalid Department
**Scenario**: Target department equals the current one, is unset, or doesn't exist.
**Expected**: "Ticket already in the department" / "Select department" / "Unknown or invalid department"; no transfer occurs.

### EC-021.6: Assigning to the Existing Assignee
**Scenario**: Re-selecting the already-assigned staff/team.
**Expected**: "Ticket already assigned to the staff." / "…to the team."; no change.

### EC-021.7: Claiming an Assigned or Closed Ticket
**Scenario**: Claim attempted on an already-assigned or non-open ticket.
**Expected**: "Ticket is already assigned to <assignee>" or "Only open tickets can be assigned"; no claim.

### EC-021.8: Closing an Already-Closed / Reopening an Already-Open Ticket
**Scenario**: Redundant status action.
**Expected**: "Ticket is already closed!" / "Ticket is already open!"; no change.

### EC-021.9: Due Date in the Past / on a Closed Ticket / Without Time
**Scenario**: Edit/create supplies an invalid due date.
**Expected**: "Due date must be in the future" / "Due date can NOT be set on a closed ticket" / "Select time" / "Invalid due date".

### EC-021.10: Non-Manager Tries to Flag Overdue/Answered/Release
**Scenario**: A non-department-manager submits overdue/answered/unanswered/release/notdue.
**Expected**: "Perm. Denied…" or "You don't have permission to set the state"; no change.

### EC-021.11: Access Lost After Transfer/Edit/Assign
**Scenario**: The action moves the ticket out of the acting staff's reach.
**Expected**: The success message is shown but the agent is returned to the listing (ticket cleared) because `checkStaffAccess` now fails.

### EC-021.12: Unban an Address Banned by a Non-Removable Filter
**Scenario**: The requester e-mail was rejected by a filter rule rather than the editable banlist.
**Expected**: Warning "Email is not in the banlist"; nothing removed.

### EC-021.13: Lock Acquisition Failure on View
**Scenario**: The lock can't be obtained when opening the ticket (race / DB issue).
**Expected**: Non-fatal warning "Unable to obtain a lock on the ticket"; the ticket is still viewable.

### EC-021.14: Renew Lock No Longer Owned
**Scenario**: The browser tries to renew a lock now held by another agent (the original expired and was taken over).
**Expected**: Renew gives up (no retry); the agent's session can no longer reply until they re-acquire.

### EC-021.15: PDF Export Failure
**Scenario**: PDF generation fails.
**Expected**: "Internal error: Unable to export the ticket to PDF for print." on the view.

### EC-021.16: Empty / Placeholder Thread Body
**Scenario**: A thread entry body is the placeholder "-".
**Expected**: The view displays "(EMPTY)" rather than a dash.

### EC-021.17: Staff Member Is Also a Client (Inbound Reply Disambiguation)
**Scenario**: An inbound e-mail address matches both the ticket owner and a staff account.
**Expected**: The ticket owner always posts a Message; a non-owner staff e-mail posts a Note; e-mail from a system account is ignored (loop control). (Trigger owned by FS-041; rule stated here as the thread model owns the entry-type decision.)

### EC-021.18: Release via the Note-Form State Dropdown
**Scenario**: A department manager picks "Release (Unassign) Ticket" from the note-form state dropdown (`state=unassigned`).
**Expected**: The note posts, but the ticket is **not** unassigned because the internal state switch matches the misspelled `unassined`; the manager must use the dedicated "Release (unassign) Ticket" action from the More menu to actually unassign (KL-021.9).

### EC-021.19: Re-Replying an Already-Fetched Inbound E-mail
**Scenario**: The same inbound e-mail (same message-id) is processed twice against a thread entry.
**Expected**: The second attempt is treated as success without re-posting (idempotent on message-id), so the mail can be safely moved/deleted.

### EC-021.20: Auto-Response to a Bounce / System Sender
**Scenario**: A ticket is created/updated from a `mailer-daemon@`/`postmaster@` address, a system e-mail account, or a detected auto-reply.
**Expected**: No outbound auto-response is sent (BS-021.20); staff alerts may still fire.

### EC-021.21: Staff Creates a Ticket for a Requester at the Open-Ticket Limit
**Scenario**: A staff member opens a ticket for a requester who is already at the max-open-tickets ceiling.
**Expected**: Creation succeeds (limit bypassed for staff origin); the over-limit user notice is suppressed though an admin over-limit warning may still be logged.

### EC-021.22: Export With a Missing / Invalid Query Token
**Scenario**: `a=export` is requested without `h`, or with a token whose session query has expired.
**Expected**: A query-token error is raised and no CSV is produced; the literal error strings are owned by FS-020.10.

### EC-021.23: Preview a Ticket Without Access
**Scenario**: The inline preview AJAX is requested for a ticket the staff cannot access (or that does not exist).
**Expected**: HTTP 404 "No such ticket".

---

## Dependencies

| Dependency | Specification | Relationship |
|------------|---------------|--------------|
| Ticket queue, listing, dashboard, search, mass-process UI | **FS-020** | Hosts the entry script and submits `mass_process`; per-ticket actions return here |
| Public web-form ticket creation | **FS-011** | Shares `Ticket::create` (origin `web`); not owned here |
| Inbound e-mail ticket creation & reply matching | **FS-041** | Shares `Ticket::create`/`postMessage`/`postEmail`/`lookupByEmailHeaders` |
| External API ticket creation & cron overdue sweep | **FS-043** | Shares `Ticket::create` (origin `api`); cron invokes `checkOverdue` and lock `cleanup` |
| Canned responses & attachment storage/serving | **FS-022** | Reply/note consume canned responses and upload attachments |
| E-mail templates, outbound mailer, variable replacer | **FS-040** | Every alert/auto-response/reply e-mail uses these |
| Departments / teams / help topics / SLA / priorities | **FS-030 / FS-032** | Lookup objects, defaults, manager check, SLA grace period |
| Staff auth, group permission flags, department access | **FS-002 / FS-031** | All permission predicates and `checkStaffAccess` inputs |
| Banlist & inbound filter rejection | **FS-042** | Ban/unban actions and reply-blocking-when-banned |
| Reference data, enums, table schemas | **FS-091** | Status / thread-type / source / event-state enums; ticket/thread/lock/event schemas |
| Alert config flags & system settings | **FS-032 / FS-091** | `alert…ON…` flags, lock-time, auto-respond, strip-quoted-reply, max-open-tickets |
| PDF paper-size set | **FS-091.11** | The print-options paper sizes are **Letter, Legal, A4, A3** (4 values). Verified against `ticket-view.inc.php` option list + bundled FPDF page formats — **no `Ledger`**; FS-091.11's `Ledger` 5th value is unsupported by source (flagged for FS-091 correction, D09X-13) |

---

## Known Limitations

### KL-021.1: Reply Visibility Is Presentational; Action Authorization Is Server-Side But Coupled to the View
**Limitation**: The view hides buttons the staff cannot use, but the authoritative gate is the POST dispatcher. The two must be kept in sync; a mismatch only ever fails closed (the server rejects), never open.
**Impact**: Adding a new action requires touching both the view (button) and the dispatcher (gate).

### KL-021.2: Lock Expiry Is Computed at Page Load, Not Real-Time
**Limitation**: A lock's remaining time is derived from the load-time `expire − now`; the server does not continuously re-validate, relying on periodic AJAX renew/poll.
**Impact**: There is a window between expiry and the next poll where two agents could both believe they hold a usable lock; the reply-time re-check mitigates double replies but does not fully eliminate races.

### KL-021.3: `staff_id` Is Overloaded as Assignee and Closed-By
**Limitation**: A single ticket field stores both the currently-assigned staff member and the staff member who closed the ticket, disambiguated only by status.
**Impact**: A closed ticket's "assignee" history is lost (it shows the closer); reopening then reassigning rewrites the same field.

### KL-021.4: Clearing Overdue Does Not Annul the Overdue Event
**Limitation**: `clearOverdue` removes the flag but leaves the previously-logged `overdue` lifecycle event intact (unlike reopen, which annuls close).
**Impact**: Overdue-event statistics can overcount tickets that were flagged then cleared.

### KL-021.5: System Overdue Sweep Has No Escalation
**Limitation**: `checkOverdue` flags newly-overdue tickets but contains only a TODO for escalating already-overdue tickets after repeated grace periods.
**Impact**: There is no automatic re-escalation; overdue alerting fires once at marking time.

### KL-021.6: PDF Export Is Latin-1 Only
**Limitation**: The PDF exporter transcodes text to windows-1252; characters outside that range may be lost or mangled.
**Impact**: Tickets with non-Latin content do not print faithfully.

### KL-021.7: Notes Are Plain-Text / HTML-Escaped (No Rich Text)
**Limitation**: Thread titles and bodies are HTML-escaped on save (inline "DELME when rich-text supported" markers), so notes/replies are effectively plain text.
**Impact**: No formatting or inline images in the thread until rich text is added.

### KL-021.9: Note-Form "Release" State Is Inert Due to a Misspelled State Constant
**Limitation**: The internal `setState` switch tests for `unassined` (a typo) instead of `unassigned`, so selecting "Release (Unassign) Ticket" from the note-form state dropdown posts the note but never unassigns the ticket. Only the dedicated `process/release` action works.
**Impact**: Managers who rely on the note-form dropdown to release a ticket will see the note logged but the assignment unchanged.

### KL-021.10: Preview Lock Banner Keys Off the Wrong Owner
**Limitation**: The inline preview shows its "Ticket is locked by <name>" banner when the lock is owned by the **current viewer** rather than by someone else, so the warning effectively never fires for the case it was meant to cover (a lock held by another agent).
**Impact**: The preview does not reliably warn that another agent is holding the ticket; the authoritative lock check still happens on the full view and at reply time.

### KL-021.8: Reopen Defaults a Ticket to Unanswered Regardless of Prior State
**Limitation**: `reopen` sets the answered flag to 0 by default; a reopened ticket always returns as unanswered even if it had a prior staff response.
**Impact**: Reopened tickets may re-surface in "unanswered" counts.

---

## Future Considerations

- Decouple action authorization from view rendering via a single permission map.
- Real-time/locked-state push (WebSocket) instead of poll-based lock renewal to close the takeover race (KL-021.2).
- Separate assignee and closed-by fields (KL-021.3).
- Annul overdue events on clear, and add SLA escalation tiers to the overdue sweep (KL-021.4, KL-021.5).
- UTF-8-capable PDF export (KL-021.6) and rich-text thread entries (KL-021.7).
- Preserve answered state across reopen, or make the reopen-as-unanswered behavior configurable (KL-021.8).
