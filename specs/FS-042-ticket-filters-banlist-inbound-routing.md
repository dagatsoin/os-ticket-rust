# FS-042: Ticket Filters, Banlist & Inbound Routing

## Overview

This specification defines the **ticket filter engine** and its special-case sibling, the **system banlist**. A *ticket filter* is an administrator-defined, ordered rule set that inspects every inbound ticket-creation request — regardless of channel (web form, inbound email, or external API) — and, when its match criteria are satisfied, either **rejects** the ticket outright or **mutates** the ticket-to-be-created (re-routing its department, priority, SLA, assignment; overriding the sender identity; suppressing auto-response; or attaching a canned auto-reply).

Filters are evaluated in administrator-controlled **execution order**; each filter targets either *all* channels or one specific channel, and email-targeted filters may additionally be scoped to a single inbound system email address. The first matching filter that is flagged "reject" aborts ticket creation; otherwise matching filters apply their actions in order, and a filter may declare "stop processing further on match" to short-circuit the remaining filters.

The **banlist** is implemented as a single reserved filter named `SYSTEM BAN LIST`: a reject-on-match, match-any filter whose rules are all of the form *sender email equals X*. It is surfaced to administrators through a dedicated banned-email-address interface (separate from the general filter editor) and is the target of the per-ticket "Ban Email" / "Unban Email" staff actions available from the ticket view.

This spec owns: filter and filter-rule data model and lifecycle (CRUD, mass enable/disable/delete); the match-criteria/operator vocabulary; the AND/OR (match-all / match-any) and stop-on-match semantics; the action set; the channel-target and email-id scoping; the banlist as a reserved filter and its dedicated interface; the per-ticket ban/unban staff actions; the fast pre-screen ban check; and the auto-response / auto-bounce header heuristics carried by the same class. The **intake channels** that *invoke* this engine (web submission, inbound email pipeline, API) own the surrounding ticket-creation flow and are referenced as dependencies. Canonical table/column schemas and enum sets are owned by **FS-091** and referenced, not restated.

---

## Functional Requirements

### FS-042.1: Filter Catalogue & Listing

**Description**: The system shall present administrators with a paginated, sortable list of all defined ticket filters.

**Acceptance Criteria**:
- The filter list is reachable only by an authenticated staff member whose admin flag is set; non-admin access is denied ("Access Denied").
- Each row shows: a selection checkbox, filter **Name** (linking to the editor), **Status** (`Active` / **`Disabled`**), execution **Order**, **Rules** count, **Target** (rendered via the target label map), **Date Added**, and **Last Updated**.
- The list is sortable by `name`, `status`, `order`, `rules`, `target`, `created`, or `updated`; unknown/absent sort key defaults to `name`. Sort direction is `ASC` (default) or `DESC`; each header link toggles direction.
- Results are paginated using the system page limit; a caption shows the "showing N filters" range, or "No filters found!" when empty.
- The list includes the reserved `SYSTEM BAN LIST` filter as a row, but selecting/opening it redirects to the dedicated banlist interface (see FS-042.11).

### FS-042.2: Create / Edit Filter

**Description**: The system shall allow an administrator to create a new filter or edit an existing one through a single form.

**Acceptance Criteria**:
- The editor collects: **Filter Name** (required), **Execution Order** (required, numeric, advertised range 1–99), **Stop processing further on match** (checkbox), **Filter Status** (Active / Disabled radio), **Target** (required; see FS-042.6), the **rule rows** (see FS-042.3), the **match-all / match-any** selector (see FS-042.4), the **action** fields (see FS-042.5), and free-text **Admin Notes**.
- On *create*, a successful save reports "Filter added successfully"; on *edit*, "Filter updated successfully".
- The editor is shown when an existing filter is being viewed, or when the request indicates "add"; otherwise the catalogue (FS-042.1) is shown.
- Editing the reserved `SYSTEM BAN LIST` filter via the general editor is not reachable — any attempt to open it redirects to the banlist interface.
- The rule region renders the existing rules plus **2 extra empty rule rows**, capped at a hard maximum of **25** rule rows.

### FS-042.3: Match Rules — Criteria, Operators & Values

**Description**: Each filter shall carry one or more match rules; each rule is a triple of *what* (criterion) / *how* (operator) / *value*.

**Acceptance Criteria**:
- The **criterion** (`what`) selectable in the editor is exactly one of (value → label):
  - `name` → "Name" (ticket owner name)
  - `email` → "Email" (sender / owner FROM address)
  - `subject` → "Subject"
  - `body` → "Body/Text"
  - (The underlying data model additionally permits `header`; it is not offered in the editor's criterion list — see KL-042.5.)
- The **operator** (`how`) is exactly one of (value → label):
  - `equal` → "Equal"
  - `not_equal` → "Not Equal"
  - `contains` → "Contains"
  - `dn_contain` → "Does Not Contain"
  - `starts` → "Starts With"
  - `ends` → "Ends With"
- Each rule requires a non-empty **value**; the value is trimmed on save.
- A rule is accepted only if both criterion and operator are present and drawn from the allowed sets, and a value is supplied; partial rows produce per-row validation errors ("Invalid match selection", "Invalid match type selection", "Value required", "Incomplete selection").
- A rule of the form `email` + `equal` additionally requires a syntactically valid email value ("Valid email required for the match type").
- At least one rule must be defined; absence yields "You must set at least one rule."
- (Canonical operator/criterion enum sets are owned by FS-091; this spec is the canonical source for their *behavioral semantics*.)

### FS-042.4: Match Logic — Match-All vs Match-Any

**Description**: The system shall evaluate a filter's rules under one of two combination modes selected per filter.

**Acceptance Criteria**:
- **Match Any** (`match_all_rules = 0`, default): the filter matches as soon as *any single* rule matches; evaluation stops at the first matching rule.
- **Match All** (`match_all_rules = 1`): the filter matches only if *every* rule matches; evaluation stops (non-match) at the first failing rule.
- Comparisons are **case-insensitive** (both the incoming field value and the rule value are upper-cased before comparison). The editor labels this "(case-insensitive comparison)".
- Operator semantics: `equal` = exact string equality; `not_equal` = not exactly equal; `contains` = rule value occurs within the field; `dn_contain` = rule value does not occur within the field; `starts` = field begins with rule value; `ends` = field ends with rule value.
- A rule whose operator is not recognized is skipped (treated as neither match nor failure).

### FS-042.5: Filter Actions

**Description**: When a filter matches a ticket-to-be-created, the system shall apply that filter's configured actions to the pending ticket arguments.

**Acceptance Criteria**:
- **Reject Ticket** (`reject_ticket`): if set, the matching filter causes the ticket to be denied; *all other actions of this and all subsequent filters are ignored* and ticket creation is aborted (see FS-042.7). The editor warns "Reject Ticket (All other actions and filters are ignored)".
- **Set Department** (`dept_id`): if non-zero, overrides the pending ticket's owning department (overrides the help-topic/default department).
- **Set Priority** (`priority_id`): if non-zero, overrides the pending ticket's priority ("Overrides department's priority").
- **Set SLA Plan** (`sla_id`): if non-zero, overrides the pending ticket's SLA ("Overrides department's SLA").
- **Auto-assign** (`staff_id` *or* `team_id`): if a staff member is chosen, assign to that staff and clear any team; if a team is chosen, assign to that team and clear any staff; if "Unassigned", no auto-assignment. (The assignment selector is an overloaded value prefixed `s` for staff or `t` for team.)
- **Use Reply-To Email** (`use_replyto_email`): if set and the inbound message carries a Reply-To address, override the pending ticket's owner email with the Reply-To address, and (if present) override the owner name with the Reply-To name.
- **Disable Auto-Response** (`disable_autoresponder`): if set, suppress the ticket's auto-responder ("Override Dept. settings").
- **Canned Response** (`canned_response_id`): if chosen (from enabled canned responses), the pending ticket is flagged to auto-reply with that canned response.
- The editor advises that non-reject actions "Can be overridden by other filters depending on processing order."

### FS-042.6: Channel Targeting & Email-ID Scoping

**Description**: Each filter shall declare which intake channel(s) it applies to, and email-targeted filters may be scoped to a single inbound system email address.

**Acceptance Criteria**:
- The **Target** selection is exactly one of (value → label): `Any` → "Any", `Web` → "Web Forms", `API` → "API Calls", `Email` → "Emails".
- When the Target dropdown selects a *specific system email* (an option drawn from the configured inbound email accounts), the stored target becomes `Email` and the filter's `email_id` is set to that account; an "Emails" or "Any" target with no specific account leaves `email_id = 0` (applies to all inbound email accounts).
- Ticket origin is normalized to a target before evaluation: `web` → `Web`, `phone` → `Web`, `staff` → `Web`, `email` → `Email`, `api` → `API`.
- During evaluation, only filters whose target is `Any` *or* equals the ticket's normalized target are considered.
- For email-origin tickets, an `email_id`-scoped filter is considered only when `email_id = 0` (all accounts) or `email_id` equals the inbound account the ticket arrived on.
- The per-filter `email_id` scope guard inside the match test fires **only when the filter's stored target is exactly `Email`**. A filter whose target is `Any` but which carries a non-zero `email_id` does **not** honor that account scope during matching, so it can apply to tickets from any account/channel (see EC-042-14).
- The candidate-set query only adds the `email_id` scope clause when the *incoming* ticket actually carries a system email id; a web/api/phone ticket (no inbound account) is not filtered by `email_id` at the query layer, so account-scoped `Any`-target filters reach the in-memory test for those origins too.
- Target is required ("Target required"); an unknown/invalid target yields "Unknown or invalid target". A purely-numeric target value is interpreted as a specific system-email selection (stored target becomes `Email`, `email_id` = that number).

### FS-042.7: Filter Evaluation Pipeline (Inbound Routing)

**Description**: The system shall evaluate the applicable filter set against every inbound ticket-creation request and apply the resulting actions or rejection.

**Acceptance Criteria**:
- A filter run is constructed from the ticket **origin** and the incoming field set (sender email, name, subject, body/message, system email id, reply-to, reply-to-name); the relevant fields are trimmed and empties dropped.
- Only **active** filters whose target matches the normalized origin (and whose `email_id` scope matches, for email) are considered, in ascending **execution order**.
- The candidate filter set is built two ways depending on available data: when a sender email is present, a fast data-layer pre-narrowing query (the "quick list") returns only filters that *might* match (filters with an email/name/subject rule whose value occurs in the corresponding incoming field, **plus** filters carrying any negative-logic rule (`not_equal`/`dn_contain`), **plus** filters whose only rule criteria are ones not present in the incoming data, **plus** non-match-all filters with at least one un-considered rule criterion such as `body`); when no sender email is present, *all* active target-scoped filters are loaded. The in-memory `matches()` test (FS-042.3/FS-042.4) then makes the final decision over this candidate set (see KL-042.10).
- The action-application step is invoked **twice** on the pending ticket variables: once to detect a rejection (and abort if a reject filter matches), and again after validation to perform the mutations. Non-reject actions are therefore applied twice; this is idempotent for set-a-field actions but means the reply-to override is re-evaluated on both passes (see EC-042-13).
- The engine first determines the subset of filters that actually match (per FS-042.3/FS-042.4), preserving execution order.
- Iterating matching filters in order:
  1. If a matching filter has **Reject Ticket** set, evaluation halts and a rejection is returned identifying that filter; ticket creation is aborted with error "Ticket denied. Error #403" (errno 403) and a warning is logged naming the rejecting filter.
  2. Otherwise the filter's actions (FS-042.5) are applied to the pending ticket arguments.
  3. If the filter has **Stop processing further on match** set, iteration stops after applying its actions.
- A non-rejected ticket proceeds to creation carrying the cumulative (last-writer-wins per field, in execution order) mutations.

### FS-042.13: Filter Action Variable Mapping & Default Interaction

**Description**: When matching filters apply their actions, the system shall mutate the *pending ticket variable set* (not yet persisted), and the resulting values shall interact with help-topic / inbound-email-account defaults in a defined order.

**Acceptance Criteria**:
- Filter actions write into the pending ticket variables under these keys (camel-cased pending-ticket fields, distinct from the persisted `filter` columns): department → `deptId`; priority → `priorityId`; SLA → `slaId`; staff assignment → `staffId`; team assignment → `teamId`; disable-auto-response → `autorespond = false`; reply-to override → `email` (and `name` when a reply-to name is present); canned response → `cannedResponseId`.
- The filter's auto-assignment writes **only one** of `staffId`/`teamId`; the spec does not clear the *other pending var* (it set whichever the filter chose), so a help-topic default for the other slot can still apply afterward (see below).
- After filters run, help-topic defaults fill any pending field the filter left unset: department, priority, SLA, and topic-driven staff/team default are applied only when the corresponding pending var is empty (filter value wins over topic default because the filter wrote it first and the topic lookup uses "keep existing value if present" semantics).
- For email-origin tickets with no help-topic, the inbound system email account supplies department/priority/auto-response defaults only when the filter (and request) left them unset.
- A filter-assigned `staffId` causes auto-assignment to that staff with the note "Auto Assignment"; a filter-assigned `teamId` causes auto-assignment to that team; both may be applied if both ended up set (one from the filter, one from the help topic).
- A filter-set `cannedResponseId` causes the created ticket to post that canned reply and be left **un-answered**, and it **disables** the standard new-ticket auto-response.

### FS-042.14: Reply-Time Ban Enforcement

**Description**: Beyond blocking ticket creation and surfacing a banner, the system shall actively block a staff reply/response when the ticket owner's email is currently banned.

**Acceptance Criteria**:
- When a staff member submits a reply/response on a ticket whose owner email is currently banned (per the fast ban pre-screen), the submission is rejected with "Email is in banlist. Must be removed to reply." and no reply is posted.
- This enforcement is in addition to the ticket-view banner ("Email is in banlist! Must be removed before any reply/response", FS-042.12); the banner is informational, this check is enforcing.
- A staff-originated *new* ticket (origin `staff`, normalized to `Web`) is also subject to the fast ban pre-screen at open time; a banned owner address denies creation just as for web/email/api origins.

### FS-042.8: Fast Ban Pre-Screen (`isBanned`)

**Description**: Independently of the full filter pipeline, the system shall provide a fast check that determines whether a given sender email address is banned.

**Acceptance Criteria**:
- The ban pre-screen considers only filters that are: active, **reject-on-match**, **match-any** (`match_all_rules = 0`), unscoped (`email_id = 0`), and that own an active rule whose criterion is `email`.
- A rule of operator `equal` matches when the address exactly equals the rule value (case-insensitive); a rule of operator `contains` matches when the rule value occurs within the address. Other operators are ignored by the pre-screen.
- The check returns the id of the first banning filter, or false if none.
- The ban pre-screen is invoked at the front of ticket creation across channels (web/email/api/staff): a banned sender is denied with "Ticket denied. Error #403" (errno 403) and a "Banned email" warning is logged, before the full filter pipeline runs.
- The create-time ban pre-screen runs **only when the sender email is present and syntactically valid**; a missing or malformed sender email skips the pre-screen entirely (and ticket creation then fails its own required-email validation).
- The pre-screen narrows candidate rules at the data layer first (only rules whose stored value occurs within the sender address are loaded), then applies the operator test in memory; rules whose stored value is not a substring of the address are never considered (see KL-042.9).
- For the inbound email channel, a banned sender causes the source message to be reported as successfully handled (moved/deleted) without creating a ticket.
- The inbound-email and ticket-view ban checks have **no email-validity guard**; they call the pre-screen directly on the owner address.

### FS-042.9: Auto-Response & Auto-Bounce Header Detection

**Description**: The system shall expose heuristics (carried by the filter subsystem) that classify an inbound email as an automatic response or a delivery bounce based on its headers.

**Acceptance Criteria**:
- An email is classified as an **auto-response** when a recognized header begins with a recognized marker, including (header → marker(s)):
  - `Auto-Submitted` → "auto-replied"
  - `Precedence` → "auto_reply" / "bulk" / "junk" / "list"
  - `Subject` → "out of office" / "auto-reply:" / "autoresponse"
  - `X-Autoreply` → "yes"
  - `X-Auto-Response-Suppress` → "all" / "dr" / "rn" / "nrn" / "oof" / "autoreply"
  - `X-Autoresponse`, `X-Auto-Reply-From` → present (any value)
  - `X-AMAZON-MAIL-RELAY-TYPE` → "notification"
- An email is classified as an **auto-bounce** when (header → marker(s)):
  - `From` → "<mailer-daemon@mailer-daemon>" / "mailer-daemon" / "<>"
  - `Subject` → "delivery failure" / "delivery status" / "undeliverable:"
- A bounce also counts as an auto-response.
- Marker matching is case-insensitive and anchored at the start of the header value.
- These classifications are consumed by the inbound email pipeline (FS-041) — e.g., to suppress auto-replies and to recognize bounces — and by thread-entry rendering; this spec is the canonical source for the marker vocabulary.

### FS-042.10: Mass Filter Operations

**Description**: The system shall allow administrators to enable, disable, or delete multiple filters in one operation from the catalogue.

**Acceptance Criteria**:
- The catalogue offers **Enable**, **Disable**, and **Delete** mass actions over the checkbox-selected filter ids; at least one selection is required ("You must select at least one filter to process.").
- **Enable** sets the selected filters active; **Disable** sets them disabled; success/partial-success messages report the affected count ("Selected filters enabled/disabled", or "N of M ...").
- **Delete** removes each selected filter that exists *and is not the reserved `SYSTEM BAN LIST`* (the banlist cannot be mass-deleted here); deleting a filter also deletes its rules.
- Delete is gated behind a confirmation dialog warning that deleted filters and their rules cannot be recovered.

### FS-042.11: Banlist — Banned Email Address Interface

**Description**: The system shall provide a dedicated interface, separate from the general filter editor, for managing the banned-email-address list backed by the reserved `SYSTEM BAN LIST` filter.

**Acceptance Criteria**:
- The interface is reachable only by an admin staff member; it operates exclusively on the reserved `SYSTEM BAN LIST` filter, which is **auto-created on demand** if absent (execution order 99, active, match-any, reject-on-match, with the note "Internal list for email banning. Do not remove").
- Each banlist entry is a single rule of the form `email` + `equal` + *address*; the list shows: selection checkbox, **Email Address** (linking to its rule), **Ban Status** (`Active` / **`Disabled`**), Date Added, Last Updated.
- The list is searchable by query term (minimum length > 3 chars; "Term too short!" otherwise); an `@`-containing valid email searches by exact address, otherwise by substring.
- **Ban New Email**: adding requires a valid email ("Valid email address required"); the address must not already be present ("Email already in the ban list"); success reports "Email address added to ban list successfully".
- **Edit entry**: an entry's address (must remain a valid email), active status, and notes may be updated.
- **Mass operations**: Enable / Disable / Delete the selected banlist entries (the underlying rules), each behind a confirmation dialog; counts are reported.
- If the banlist filter itself is disabled, the interface warns it is **DISABLED** with a link to re-enable it via the filter editor; an empty list warns "System ban list is empty."

### FS-042.12: Per-Ticket Ban / Unban Staff Action

**Description**: From the ticket view, a permitted staff member shall be able to ban or unban the ticket owner's email address.

**Acceptance Criteria**:
- "Ban Email (<address>)" and "Unban Email (<address>)" actions are offered from the ticket-view actions menu, each behind a confirmation dialog.
- Both actions require the acting staff member to hold the **ban-emails** permission (group flag `can_ban_emails`); without it, "Perm. Denied. You are not allowed to ban emails" / "... remove emails from banlist."
- **Ban** adds the ticket owner's email as a banlist entry; if already present, "Email already in banlist"; on success, "Email (<address>) added to banlist".
- **Unban** removes the matching banlist entry; on success, "Email removed from banlist"; if it was not present, the action warns "Email is not in the banlist".
- When the current ticket's owner email is banned, the ticket view surfaces an error banner ("Email is in banlist! Must be removed before any reply/response") and the menu offers Unban only when the address is present as an explicit ban entry (i.e., removable); a sender banned merely by a *contains* / wildcard ban filter is flagged but not directly unbannable from the ticket.
- The container "More" actions menu that hosts Ban/Unban is itself shown when the staff member **either** holds `can_ban_emails` **or** is a manager of the ticket's department; however the individual Ban/Unban list items are rendered (and their POST actions accepted) only for staff who hold `can_ban_emails`. A department manager without `can_ban_emails` sees the More menu but not the Ban/Unban entries.
- The Ban entry is offered only when the owner is **not** currently banned; the Unban entry is offered only when the owner is currently banned **and** the address is an explicit removable entry (`unbannable`). When the owner is banned but not unbannable (banned via a non-exact rule), neither entry appears.
- The per-ticket Ban action internally adds the address as an `email`+`equal`+address rule on the reserved filter and records the acting staff name as the submitter argument (the submitter value is accepted but not persisted on the rule).

---

## Business Rules

- **BS-042-01 (Execution order is the precedence key)**: Filters are evaluated strictly in ascending `execorder`. Where two filters set the same ticket field, the later-ordered filter's value wins (last-writer-wins), except that a reject short-circuits everything.
- **BS-042-02 (Reject is absolute)**: The first matching reject-on-match filter aborts ticket creation; no subsequent filter — and no other action of the rejecting filter — is applied. Rejection denies the ticket with error #403 (errno 403) and logs a warning naming the rejecting filter.
- **BS-042-03 (Stop-on-match short-circuit)**: A matching filter flagged `stop_onmatch` applies its actions and then halts further filter processing for that ticket.
- **BS-042-04 (Match-any is the default combination mode)**: New filters default to match-any (`match_all_rules = 0`); match-all requires every rule to match.
- **BS-042-05 (Case-insensitive comparison)**: All rule comparisons upper-case both operands before comparing.
- **BS-042-06 (Channel scoping)**: A filter applies only when its target is `Any` or equals the ticket's normalized origin target (`web`/`phone`/`staff` → Web, `email` → Email, `api` → API). Email-scoped filters additionally require `email_id = 0` or a match on the arriving inbound account.
- **BS-042-07 (Email-equal rules require valid emails)**: A rule combining criterion `email` with operator `equal` is rejected unless its value is a syntactically valid email address.
- **BS-042-08 (At least one rule)**: A filter with no rules cannot be saved.
- **BS-042-09 (Rule cap)**: A filter may carry at most 25 rules (editor and save loop are both bounded at 25).
- **BS-042-10 (Unique rule per filter)**: Within a filter, a (criterion, operator, value) triple is unique; the banlist relies on this to prevent duplicate bans.
- **BS-042-11 (Mass-save replaces all rules)**: Saving a filter clears and re-inserts its entire rule set ("mass replace on each save"); rules are not patched individually through the filter editor.
- **BS-042-12 (Unique filter name)**: Filter names must be unique ("Name already in use").
- **BS-042-13 (Banlist is a reserved, special-cased filter)**: A filter named exactly `SYSTEM BAN LIST` (case-insensitive) is the banlist. It is reject-on-match, match-any, unscoped, and is auto-created on demand. It is excluded from the general filter editor (redirect) and from mass-delete on the catalogue.
- **BS-042-14 (Ban entry shape)**: Every banlist entry is precisely an `email` + `equal` + *address* rule on the reserved filter; bans are added/removed at rule granularity.
- **BS-042-15 (Ban pre-screen runs before the full pipeline)**: Across web/email/api channels, the fast ban check (FS-042.8) executes before the filter pipeline; a banned sender is denied immediately with error #403.
- **BS-042-16 (Ban pre-screen scope)**: The fast ban check considers only active, reject-on-match, match-any, unscoped (`email_id = 0`) filters with active `email` rules using `equal` or `contains` — i.e., it does not honor `not_equal`, `starts`, `ends`, `dn_contain`, channel scoping, or match-all bans.
- **BS-042-17 (Ban/unban permission)**: Per-ticket ban and unban require the `can_ban_emails` group permission.
- **BS-042-18 (Banned owner blocks replies)**: A ticket whose owner email is currently banned is flagged in the ticket view and must be unbanned before reply/response.
- **BS-042-19 (Auto-response/bounce markers are start-anchored, case-insensitive)**: Header classification matches markers only at the start of the (upper-cased) header value.
- **BS-042-20 (Create-time pre-screen requires a valid email)**: The fast ban pre-screen at ticket creation runs only when the sender email is present and syntactically valid; the inbound-email and ticket-view ban checks have no such guard and run on the raw owner address.
- **BS-042-21 (Actions apply to pending vars, filter value wins over topic/account defaults)**: Filter actions mutate the pending (unsaved) ticket variables; help-topic and inbound-account defaults backfill only fields the filters left unset, so a filter-set field beats the topic/account default.
- **BS-042-22 (Canned-response action suppresses auto-response and leaves ticket un-answered)**: A filter-set canned response posts that reply, disables the standard new-ticket auto-response, and marks the new ticket un-answered.
- **BS-042-23 (Reply blocked while owner banned)**: A staff reply/response is rejected at submit time when the ticket owner's email is currently banned ("Email is in banlist. Must be removed to reply."), independent of the informational banner.
- **BS-042-24 (Banlist filter may exist with zero rules)**: The reserved `SYSTEM BAN LIST` filter is created with an explicitly-supplied empty rule array, which bypasses the "at least one rule" requirement (BS-042-08) via the pre-built-rules path; the general editor's validation still requires ≥1 rule.
- **BS-042-25 (Actions applied twice per ticket)**: The filter action set is applied twice during ticket creation (rejection-detection pass, then mutation pass); set-field actions are idempotent, reply-to override is re-evaluated on both passes.
- **BS-042-26 (Email-id scope honored only for Email-targeted filters)**: The per-filter account-scope guard during matching fires only when the filter's target is exactly `Email`; `Any`-targeted filters ignore their own non-zero `email_id` while matching.

---

## Data Requirements

> Canonical table/column definitions and enum sets are owned by **FS-091**; summarized here for this domain's behavior.

- **Filter** (`filter` table): identity (`id`); ordering (`execorder`, default 99); activation (`isactive`); combination mode (`match_all_rules`); short-circuit (`stop_onmatch`); actions (`reject_ticket`, `use_replyto_email`, `disable_autoresponder`, `canned_response_id`, `priority_id`, `dept_id`, `staff_id`, `team_id`, `sla_id`); channel scope (`target` ∈ {`Any`,`Web`,`Email`,`API`}, `email_id` where 0 = all inbound accounts); unique `name` (≤ 32 chars); free-text `notes`; `created` / `updated` timestamps. Indexed on `target` and `email_id`.
- **Filter Rule** (`filter_rule` table): identity (`id`); parent (`filter_id`); criterion `what` ∈ {`name`,`email`,`subject`,`body`,`header`}; operator `how` ∈ {`equal`,`not_equal`,`contains`,`dn_contain`,`starts`,`ends`}; `val` (≤ 255 chars, trimmed); `isactive`; `notes`; `created` / `updated`. Unique constraint on (`filter_id`,`what`,`how`,`val`).
- **Banlist seed**: the install seeds one reserved filter `SYSTEM BAN LIST` (active, execorder 99, reject-on-match) plus a sample ban rule.
- **Permission**: the `can_ban_emails` flag on the staff **group** (FS-031) gates per-ticket ban/unban.
- **Referenced lookups** for action selectors: departments (FS-030), priorities & SLAs (FS-032), staff & teams (FS-030/FS-031), canned responses (FS-022), inbound system email accounts (FS-040).
- **Incoming evaluation fields** (constructed per ticket, not persisted): `email`, `name`, `subject`, `body`, `emailId`, `reply-to`, `reply-to-name`, and (for the header heuristics) raw email `headers`.

---

## User Flows / Interactions

1. **Define a routing filter**: Admin → Manage → Ticket Filters → *Add New Filter* → name, execution order, status, target (channel or specific email) → add rule rows (criterion/operator/value) → choose match-all/match-any → choose actions (reject, dept, priority, SLA, assign, reply-to, disable auto-response, canned response) → notes → Save. The filter takes effect on the next inbound ticket for its target channel.
2. **Inbound ticket arrives** (web/email/api): system normalizes origin → runs ban pre-screen (deny #403 if banned) → builds the active, target-scoped filter set in execution order → applies matching filters' actions (last-writer-wins; stop-on-match honored) → if a reject filter matches, deny #403 → otherwise create the (possibly re-routed) ticket.
3. **Ban from ticket view**: Staff with ban permission opens a ticket → actions menu → *Ban Email (<addr>)* → confirm → address added to `SYSTEM BAN LIST`. Future tickets from that address are denied #403.
4. **Unban from ticket view**: Staff opens a banned ticket (reply blocked banner shown) → *Unban Email (<addr>)* (offered only when the address is an explicit removable entry) → confirm → entry removed.
5. **Manage the banlist directly**: Admin → Emails → Banlist → search / *Ban New Email* / edit entry / mass enable-disable-delete. If the banlist filter is disabled, a warning links to re-enable it.
6. **Mass-manage filters**: Admin → Ticket Filters → select rows → Enable / Disable / Delete (delete confirmed; banlist excluded).

---

## Edge Cases

- **EC-042-1**: Opening the reserved `SYSTEM BAN LIST` through the general filter editor redirects to the dedicated banlist interface; it can never be edited as an ordinary filter.
- **EC-042-2**: The `SYSTEM BAN LIST` filter is excluded from catalogue mass-delete; selecting it for deletion silently skips it (reflected in the "N of M deleted" count).
- **EC-042-3**: An unrecognized operator on a rule is skipped during matching (counts as neither match nor failure), which can subtly change match-all outcomes.
- **EC-042-4**: A reject filter that also sets dept/priority/etc. has those actions ignored — reject wins and short-circuits before any action is applied.
- **EC-042-5**: An address banned only via a `contains` (substring/wildcard) ban rule is detected by the ticket view as banned, but is *not* directly unbannable from the ticket (no exact entry to remove); the staff member must edit the banlist.
- **EC-042-6**: The fast ban pre-screen ignores match-all bans, channel-scoped (`email_id ≠ 0`) bans, and the `not_equal`/`starts`/`ends`/`dn_contain` operators — such "bans" only take effect through the full pipeline at ticket creation, not the pre-screen, so behavior can differ between the pre-screen and the full evaluation.
- **EC-042-7**: For inbound email, a banned sender is reported as a *handled* message (moved/deleted) so the same email is not retried; no ticket and no client-facing rejection is produced.
- **EC-042-8**: A 403-rejected inbound email is permanently marked processed (its headers logged against ticket 0) so it is never re-processed.
- **EC-042-9**: Adding a ban that already exists is blocked at the UI ("Email already in the ban list") and additionally guarded by the per-filter unique (what,how,val) constraint.
- **EC-042-10**: Reply-To override only takes effect when the inbound message actually carries a Reply-To value; otherwise the original sender identity is retained.
- **EC-042-11**: Banlist search terms of length ≤ 3 are rejected ("Term too short!").
- **EC-042-12**: A disabled `SYSTEM BAN LIST` filter means no bans are enforced even though entries exist; the interface warns and links to re-enable.
- **EC-042-13**: Because the action set is applied twice during creation (rejection pass + mutation pass), a reply-to override is evaluated on both passes; this is harmless for set-field actions but is a redundant re-application worth being aware of.
- **EC-042-14**: A filter whose target is `Any` but which carries a non-zero `email_id` does not honor that account scope while matching (the scope guard only fires for `Email`-targeted filters), so such a filter applies across all accounts and channels — likely contrary to the admin's intent when an account was selected.
- **EC-042-15**: A ticket whose origin does not normalize to a known target (`web`/`phone`/`staff` → Web, `email` → Email, `api` → API; anything else → undefined) yields an empty/undefined target, so only `Any`-targeted filters are considered for that ticket.
- **EC-042-16**: The create-time ban pre-screen is skipped when the sender email is absent or invalid; such a ticket is still rejected by the normal required-valid-email validation, but a malformed-but-banned-looking address never reaches the ban check.
- **EC-042-17**: A self-loop guard outside this engine (the create path) suppresses auto-response for senders matching the system's own email accounts or `mailer-daemon@`/`postmaster@` addresses, independent of the filter `disable_autoresponder` action.
- **EC-042-18 (quickList over-inclusion is by design; `matches()` is authoritative)**: The `TicketFilter::quickList` short-list is a *backwards* SQL pre-filter — rather than proving a filter matches the incoming email, it eliminates filters that could *never* match and returns all the rest, deliberately accepting false positives. It pulls active, target-scoped, account-scoped filters that either have a sender-email/sender-name/subject rule that `LOCATE`s within the incoming values *or* are not email/name/subject-anchored at all (so body/header-only and `not_equal`/`dn_contain` filters are kept regardless). When `vars` carries no email it degrades to `getAllActive()` (every active in-scope filter). Consequently the returned set may include filters that ultimately do **not** match; correctness is **only** decided by each filter's `matches()` method (the full per-rule evaluation, BS-042-04/05). Callers must never treat quickList membership as a match — it is purely a performance pre-screen that narrows the candidate set before authoritative evaluation.

## Dependencies

- **FS-011 (Public ticket submission / web)** — invokes the ban pre-screen and filter pipeline during web ticket creation.
- **FS-041 (Inbound email pipeline)** — invokes the ban pre-screen and filter pipeline during email→ticket creation, and consumes the auto-response / auto-bounce heuristics (FS-042.9).
- **FS-043 (External API & cron)** — invokes the ban pre-screen and filter pipeline for API-originated tickets.
- **FS-021 (Staff ticket view & workflow)** — hosts the per-ticket Ban / Unban actions and the banned-owner reply-block banner.
- **FS-031 (Staff, groups)** — provides the `can_ban_emails` permission that gates per-ticket ban/unban.
- **FS-030 / FS-032 (Departments, teams, priorities, SLAs)** and **FS-022 (Canned responses)** and **FS-040 (Email accounts)** — provide the lookup values selected by filter actions and targeting.
- **FS-091 (Reference data & data model)** — canonical owner of the `filter` / `filter_rule` schemas and the criterion/operator/target enum sets referenced here.
- **FS-090 (Shared UI, pagination)** — list pagination and sorting infrastructure used by the filter and banlist tables.

## Known Limitations

- **KL-042.1**: The rule limit is a hard-coded 25 (the editor and save loop both stop at 25); there is no configurable maximum.
- **KL-042.2**: Filter matching is performed in application memory (not pushed to the data layer) and is case-insensitive only via full upper-casing; there is no Unicode-aware (multibyte) case folding — flagged in source as a known shortcoming.
- **KL-042.3**: The fast ban pre-screen and the full pipeline implement *different* subsets of ban semantics (see EC-042-6), so a "ban" expressed with operators other than `equal`/`contains`, or as match-all / channel-scoped, behaves inconsistently between the front-door pre-screen and creation-time evaluation.
- **KL-042.4**: The banlist's dedicated "ban rule" detail/add template (`banrule.inc.php`) referenced by the banlist controller is absent from this snapshot; the add/edit-single-entry path falls back to that missing template, so the single-rule add/edit view may not render in this build (list and mass operations are unaffected).
- **KL-042.5**: The `header` criterion exists in the data model but is not offered in the filter editor's criterion dropdown, so header-based match rules cannot be created through the UI (only via direct data manipulation).
- **KL-042.6**: There is no per-filter test/preview harness; filter correctness can only be observed by submitting real tickets.
- **KL-042.7**: Auto-assignment via a filter sets either staff or team but never both, and clears the other (in storage); there is no way to express "assign to team AND member" through a single filter action. Note however that at creation time a help-topic default can still supply the *other* slot, so a ticket can end up with both a filter-assigned and a topic-assigned actor.
- **KL-042.8**: In the fast ban pre-screen, the in-memory match test for the `equal` operator is mis-coded: its acceptance condition reduces to "the string-compare result is not null", which is effectively always true for any rule that survived the data-layer substring pre-filter. Consequently an `equal` ban rule behaves like a `contains` ban in the pre-screen — any address that has the rule value as a substring is treated as banned, not only an exact match. (The full pipeline's `matches()` does enforce true equality.)
- **KL-042.9**: The pre-screen's data-layer pre-filter only loads ban rules whose stored value occurs *as a substring* of the incoming address; a ban rule can therefore never match an address it is not a substring of, regardless of operator — there is no support for `starts`/`ends`/`not_equal`/`dn_contain` in the pre-screen.
- **KL-042.10**: The candidate-set "quick list" pre-narrowing query is a heuristic that aims to return a superset of potentially-matching filters; its SQL is intricate and conservatively includes negative-logic and un-considered-criterion filters. Final correctness still relies on the in-memory `matches()` test, but a filter incorrectly excluded by the heuristic would be silently skipped. When no sender email is present, the heuristic is bypassed and all active target-scoped filters are evaluated.
- **KL-042.11**: The reserved `SYSTEM BAN LIST` filter is created with both no `target` set in code (defaulting to the data-model default `Any`) and no `email_id`, matching the install-seeded row; it is therefore unscoped by both target and account.
- **KL-042.12**: After the filter row is saved, the subsequent rule-persistence step ignores its own errors (rules are best-effort re-inserted); a filter row can persist while one or more of its rules silently fail to save.
- **KL-042.13**: The banlist list view has source-level defects: its column-sort options map "Date Added" sorting to the *updated* timestamp (a duplicated array key overwrites the created mapping), and the sortable column header links point at `staff.php` rather than `banlist.php`. Sorting therefore behaves unexpectedly and header links navigate away from the banlist.
- **KL-042.14**: The "System ban list is empty." warning path in the banlist controller is effectively unreachable, because the banlist filter is auto-created on demand whenever the page loads, so the filter object is never null.
- **KL-042.15**: A filter `name` is limited to 32 characters and a rule `value` to 255 characters at the data layer; the editor does not advertise these limits, so over-length input is silently truncated by the store rather than validated.
