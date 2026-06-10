# FS-011: Public Ticket Submission (Web Form)

## Overview

The Public Ticket Submission specification defines the public-facing "Open a New Ticket" web form through which an end user (whether anonymous or signed into the client portal) creates a new support ticket. It owns the request handler that receives the submitted form, the markup and field-by-field behavior of the form itself, the validation rules applied to a web-sourced ticket, the help-topic-driven routing of the new ticket (department, priority, assignment, SLA), the gating controls that protect the public endpoint (CAPTCHA challenge, e-mail ban-list rejection, maximum-open-ticket throttle), the handling of file attachments submitted with the form, the autoresponse and alert e-mails fired on successful creation, and the success / "thank-you" and error responses shown back to the user.

This specification covers **only the public web-submission path** — ticket source `Web`. Ticket creation initiated by staff inside the control panel is owned by **FS-021** (staff ticket view & workflow); ticket creation from inbound e-mail is owned by **FS-041** (inbound e-mail pipeline); ticket creation through the external API is owned by **FS-043** (external API & cron). Those three paths share the same underlying ticket-creation routine (`Ticket::create`), so the shared validation and routing core is documented here from the web perspective, and the other source-specific behaviors are cross-referenced.

Persistence, enum sets (ticket source, status, reference types), and the full table/column schema are canonical to **FS-091** and are referenced rather than restated. File storage internals (content-addressed chunked storage) are owned by **FS-022**. E-mail templates and outbound delivery are owned by **FS-040**. The ban-list and ticket-filter rejection engine is owned by **FS-042**. Help topics, departments, teams, priorities, and SLA plans are configured under **FS-030 / FS-032**.

---

## Functional Requirements

### FS-011.1: New-Ticket Request Handler

**Description**: The system shall expose a public endpoint (`open.php`) that renders the new-ticket form on a non-POST request and processes a submitted form on a POST request. The endpoint declares the ticket source as the literal `Web` for every ticket it creates.

**Acceptance Criteria**:
- The endpoint boots through the public client request lifecycle (it includes `client.inc.php`), so the shared client header/footer, CSRF protection, and the current client session (`$thisclient`, if any) are available (see FS-001, FS-010).
- The endpoint defines the ticket source constant as the literal string `Web` and passes it to the ticket-creation routine.
- On a non-POST request, the endpoint renders the empty new-ticket form (FS-011.2).
- On a POST request, the endpoint collects the submitted values, applies the gating and validation rules (FS-011.5–FS-011.9), and attempts to create the ticket.
- Before passing the submitted values to creation, the handler force-sets `deptId` and `emailId` to `0` ("Just making sure we don't accept crap... only topicId is expected"), so a web submitter cannot pick the department or the receiving e-mail account directly — routing is driven exclusively by the chosen help topic. (Cross-ref FS-011.4.)
- The active navigation item is set to "new" so the client portal highlights the "Open a New Ticket" link.
- On successful creation the handler shows a success / thank-you response (FS-011.10); on failure it re-renders the form with inline error messages (FS-011.11).

### FS-011.2: New-Ticket Form Layout

**Description**: The system shall render an "Open a New Ticket" form (`open.inc.php`) collecting the requester's identity, the help topic, the subject, the message, and — conditionally — attachments, ticket priority, and a CAPTCHA challenge.

**Acceptance Criteria**:
- The page heading reads exactly **"Open a New Ticket"** with the lead-in text **"Please fill in the form below to open a new ticket."**
- The form posts (`method="post"`) to `open.php` with `enctype="multipart/form-data"` and carries a CSRF token field and a hidden field `a=open`.
- The form presents the following fields in order, with the required ones marked with a required indicator (a leading `*`):

  | Order | Field | Form name | Required | Control |
  |-------|-------|-----------|----------|---------|
  | 1 | Full Name | `name` | Yes | text input (size 30) — see FS-011.3 |
  | 2 | Email Address | `email` | Yes | text input (size 30) — see FS-011.3 |
  | 3 | Telephone | `phone` | No | text input (size 17) |
  | 3b | Ext. | `phone_ext` | No | text input (size 3) |
  | 4 | Help Topic | `topicId` | Yes | dropdown — see FS-011.4 |
  | 5 | Subject | `subject` | Yes | text input (size 40) |
  | 6 | Message | `message` | Yes | textarea (60 cols × 8 rows) with helper text "Please provide as much detail as possible so we can best assist you." |
  | 7 | Attachments | `attachments[]` | No | multi-file input — conditional (FS-011.7) |
  | 8 | Ticket Priority | `priorityId` | No | dropdown — conditional (FS-011.6) |
  | 9 | CAPTCHA Text | `captcha` | Conditional | text input (size 6) + challenge image — conditional (FS-011.5) |

- The form provides three buttons: **"Create Ticket"** (submit), **"Reset"** (clears the form), and **"Cancel"** (navigates to `index.php`).
- When the form re-renders after a failed submission, each field is repopulated with the submitter's previously entered values (HTML-escaped), and each field's inline error message (if any) is displayed beside it (see FS-011.11).

### FS-011.3: Identity Pre-Fill for Signed-In Clients

**Description**: When the submitter is a valid signed-in client, the system shall pre-fill and lock the identity fields from the client's session rather than accepting them from the form.

**Acceptance Criteria**:
- When a valid client session exists (`$thisclient->isValid()`), the form displays the client's name and e-mail as static text (not editable inputs) for the Full Name and Email Address fields, and pre-fills the Telephone and Ext. fields from the client's stored phone / phone-ext.
- When a client session object is present, the handler overrides the submitted `name` and `email` with the client's session name and e-mail before creation — the requester cannot impersonate a different identity than their authenticated one. (Note: the handler performs this identity override whenever a client-session object is present, whereas the form template's static-text rendering, the CAPTCHA suppression, and the post-success redirect additionally require the session to be **valid** (`isValid()`). In practice a present-but-invalid session is not reachable on this endpoint, so the two conditions coincide; the asymmetry is documented for completeness.)
- When no valid client session exists, the Full Name and Email Address fields render as editable text inputs that the anonymous submitter must complete.
- A signed-in client is never shown the CAPTCHA challenge (FS-011.5).

### FS-011.4: Help-Topic Selection and Routing Effects

**Description**: The system shall populate the Help Topic dropdown from the public help topics and shall use the chosen help topic to derive the new ticket's department, priority, staff/team assignment, and SLA.

**Acceptance Criteria**:
- The dropdown's first option is a non-selectable prompt reading exactly **"— Select a Help Topic —"** with an empty value.
- The dropdown lists every **active, public** help topic (`isactive=1 AND ispublic=1`), ordered by name; sub-topics display as `Parent / Child`. (Help-topic configuration is owned by FS-030.)
- If no public help topics are configured, the dropdown falls back to a single option **"General Inquiry"** with value `0`.
- Help Topic is required for a web ticket: an empty selection produces the field error **"Select help topic"** (FS-011.8).
- When a valid help topic is selected, the new ticket inherits, in priority order:
  - **Department**: the help topic's department (the web handler forces the submitter's `deptId` to `0`, so the topic's department always governs unless the system default applies).
  - **Priority**: the help topic's priority, unless the submitter chose a priority on the form (FS-011.6) — the form-chosen priority takes precedence when present.
  - **Staff or Team assignment**: if the help topic has a default staff member, the ticket is auto-assigned to that staff member ("Auto Assignment"); otherwise, if it has a default team, the ticket is auto-assigned to that team.
  - **SLA plan**: the help topic's SLA plan, if set; otherwise the system default SLA.
  - **Autoresponse**: the help topic's autoresponse setting can suppress the new-ticket autoresponse for that topic (FS-011.9).
- If, after topic resolution, no department or priority has been determined, the system default department and system default priority apply.

### FS-011.5: CAPTCHA Challenge (Anonymous Submitters)

**Description**: When CAPTCHA is enabled and the submitter is not a valid signed-in client, the system shall present an image CAPTCHA challenge and reject the submission unless the entered text matches the challenge.

**Acceptance Criteria**:
- The CAPTCHA row is shown only when CAPTCHA is enabled **and** the submitter is anonymous (no valid client session). A valid signed-in client never sees the CAPTCHA row and is never CAPTCHA-validated.
- CAPTCHA is considered "enabled" only when the image-generation capability is available on the server **and** the administrator has turned on the CAPTCHA setting. (Cross-ref FS-032 for the admin setting; the capability dependency is a Known Limitation — see KL-011.1.)
- The challenge image is served by a separate image endpoint (`captcha.php`) — image generation and the session-stored expected-answer side effect are owned by FS-010.11. open.php consumes that session-stored answer (see below).
- On submit, validation is:
  - If the CAPTCHA field is empty → error **"Enter text shown on the image"**.
  - Else if the uppercased entered text's digest does not match the session-stored expected digest → error **"Invalid - try again!"**.
- CAPTCHA matching is case-insensitive (the challenge answer is uppercased on both generation and verification).
- When the form re-renders after a failed submission and the CAPTCHA field had no specific error, the field error is set to **"Please re-enter the text again"** to prompt a fresh attempt (a new challenge image is fetched on re-render).

### FS-011.6: Optional Priority Selection

**Description**: When the administrator allows requesters to choose a priority, the system shall present a Ticket Priority dropdown defaulting to the system default priority.

**Acceptance Criteria**:
- The Ticket Priority dropdown is shown only when the "allow priority change" setting is enabled **and** at least one priority exists. (Priority configuration is owned by FS-032; the priority enum/value set is canonical to FS-091.)
- When shown and the submitter has not chosen a priority, the dropdown pre-selects the system default priority.
- The submitter-chosen priority, when present, takes precedence over the help topic's priority (FS-011.4).
- When the priority dropdown is **not** shown, the ticket's priority is resolved entirely from the help topic / system default (FS-011.4); a web submitter cannot inject a priority that the form did not offer (the handler still validates `priorityId` as an optional integer).

### FS-011.7: Attachment Submission

**Description**: When online attachments are allowed, the system shall present a multi-file attachment input on the new-ticket form and shall validate and attach uploaded files to the ticket's initial message.

**Acceptance Criteria**:
- The attachment input is **shown** when **either**: online attachments are allowed and attachments are **not** restricted to signed-in clients only; **or** attachments are restricted to logged-in clients and the submitter is a valid signed-in client. (Two admin toggles: "allow online attachments" and "allow attachments on login only".)
- **The display gate and the processing gate differ** (BS-011.12): the handler formats and processes posted files based **only** on whether online attachments are allowed (`allowOnlineAttachments()`) and whether any files were posted — it does **not** re-check the "attachments on login only" restriction or the client's logged-in state. So a crafted anonymous POST carrying attachment files, on a system configured for "attachments on login only" (where the form would not have shown the input), is still accepted and the files are still validated and attached.
- Attachment formatting/validation is **skipped when an error has already been recorded** (e.g. a CAPTCHA failure): the handler formats the uploaded files only when no prior error exists for the submission. A submission that already failed CAPTCHA therefore does not have its files validated on that pass (the files must be re-supplied on the corrected resubmission, since the file input cannot be repopulated — see KL-011.6).
- When online attachments are allowed, no prior error exists, and files were posted, the handler formats and validates the uploaded files in "restricted" mode before creation. The type/size validation contract and its per-file error strings are owned by **FS-022**; the allowed file-type list and max-file-size cap values are owned by **FS-032**. (Web summary: each posted file is checked against the allowed file-type list and the maximum file size, and a PHP-level upload failure or "no file uploaded" empty slot is handled per FS-022.)
- Validated attachments are passed into creation as part of the ticket variables and are stored against the ticket's initial message (the message thread entry); attachment storage and de-duplication are owned by FS-022.
- File-level attachment errors are surfaced beside the Attachments field on re-render (FS-011.11). (Note: attachment validation errors are non-fatal to the form display but, like any error, block ticket creation — see EC-011.5.)

### FS-011.8: Web-Ticket Field Validation

**Description**: The system shall validate the submitted ticket fields according to a web-source field set and shall reject creation if any required field is missing or any field is invalid.

**Acceptance Criteria**:
- Before validation, leading/trailing whitespace is trimmed from `email`, `phone`, `subject`, and `name`.
- The web-source required field set and their error messages are:

  | Field | Type | Required | Error message |
  |-------|------|----------|---------------|
  | `name` | string | Yes | "Name required" |
  | `email` | email | Yes | "Valid email required" |
  | `subject` | string | Yes | "Subject required" |
  | `message` | text | Yes | "Message required" |
  | `topicId` | int | Yes | "Select help topic" |
  | `priorityId` | int | No | "Invalid Priority" |
  | `phone` | phone | No | "Valid phone # required" |

  (`deptId`, `emailId`, `duedate`, and `source` are not part of the web required set — they belong to the staff/email/api sources; cross-ref FS-021/FS-041/FS-043.)
- Phone-extension rule: if a phone extension was entered, it must be numeric (error **"Invalid phone ext."**) and a phone number must also be present (error **"Phone number required"**) — an extension without a base number is rejected.
- If any required field is missing or any field is invalid and no higher-level error message has already been set, the form-level error is **"Missing or invalid data - check the errors and try again"**.
- Any validation error is fatal: creation does not proceed and the form is re-rendered with the inline errors (FS-011.11).
- The submitter's name and subject are sanitized (tags stripped) before being persisted on the ticket record. (The free-text message body is stored as the thread message; thread/message handling is owned by FS-021.)

### FS-011.9: Ticket Creation, Routing, and Reference Assignment

**Description**: On passing all gates and validation, the system shall create the ticket with the resolved routing, generate its public ticket reference, post the initial message, apply assignments and SLA, and log the creation event.

**Acceptance Criteria**:
- The new ticket is created with source `Web`, the resolved department, help topic, and priority (FS-011.4), the requester's name/e-mail/phone/extension, the requester's IP address (the request's remote address, unless an explicit IP was supplied — web submissions use the remote address), and a creation timestamp.
- A public ticket reference is generated:
  - When random IDs are enabled, a unique random numeric reference of the configured length is generated (regenerated on the rare collision so the reference is unique).
  - When random IDs are disabled, the ticket's sequential internal id is used as the public reference. (Ticket reference semantics are canonical to FS-091.)
- The initial subject is used as the title of the first message post; the message is posted to the ticket's thread (thread/message internals owned by FS-021).
- The SLA is applied to the ticket (resolved per FS-011.4).
- Auto-assignment is applied: to the help topic's default staff member ("Auto Assignment") and/or default team. (Ticket-filter-driven assignment is owned by FS-042; the web path applies the filter pipeline — see FS-011.13.)
- New-ticket autoresponse and staff alerts are fired (FS-011.12), subject to the suppression rules.
- If this creation causes the requester's open-ticket count to **equal** the maximum-open-ticket limit, the over-limit handler runs (FS-011.13, BS-011.4), with the requester e-mail and admin alert gated as described in BS-011.4.
- A `created` lifecycle event is logged against the ticket. (Event log schema owned by FS-091.)
- On success the handler returns the created ticket so the success/thank-you response (FS-011.10) can be shown.

### FS-011.10: Success / Thank-You Response

**Description**: On successful creation the system shall confirm the ticket to the requester without exposing the ticket number on screen, and shall route a signed-in client to their newly created ticket.

**Acceptance Criteria**:
- For a **signed-in valid client**, on success the handler regenerates the session and redirects the browser to the new ticket's view page (`tickets.php?id={publicRef}`). If the "show related tickets" setting is disabled, the client's session login key is reset to the newly created ticket so they see this ticket on return (multi-ticket portal behavior owned by FS-010).
- For an **anonymous submitter**, on success the page shows a thank-you body:
  - If the resolved help topic has a custom "thank-you" page configured, that page's body is shown; otherwise the system-wide thank-you page body is shown (if one is configured). (Custom pages owned by FS-033.)
  - Within the thank-you body, the ticket-number tokens (`%{ticket.number}`, `%{ticket.extId}`, `%{ticket}`) are replaced with a masked placeholder (`XXXXXX`) — **the ticket number is never rendered on the screen**, only delivered by e-mail, for security reasons (BS-011.6).
- If neither a topic-specific nor a system thank-you page is configured, the new-ticket form is re-rendered (a fresh blank form) rather than a thank-you page. (See EC-011.7.)
- The success confirmation message string available to the handler is **"Support ticket request created"**.

### FS-011.11: Error Re-Display

**Description**: On a failed submission the system shall re-render the form with the submitter's entered values preserved and the relevant error messages displayed inline.

**Acceptance Criteria**:
- When the submission fails (any gate or validation error), the form fields are re-populated from the submitted values, HTML-escaped.
- Each field with a specific error displays that error inline beside the field (name, email, phone, phone_ext, topicId, subject, message, attachments, priorityId, captcha).
- A top-level / form-level error message is shown; if the creation routine did not set a specific top-level error, the default top-level error is **"Unable to create a ticket. Please correct errors below and try again!"**.
- Required-field indicators (`*`) remain on the required fields.

### FS-011.12: New-Ticket Autoresponse and Staff Alerts

**Description**: On successful creation the system shall send a new-ticket autoresponse to the requester and a new-ticket alert to the relevant staff, each subject to configuration and loop-prevention suppression rules.

**Acceptance Criteria**:
- The effective autoresponse decision can be **overridden by a ticket-filter rule**: if the filter pipeline set an `autorespond` variable on the new-ticket variables, that value replaces the default autoresponse flag before the autoresponse/alert logic runs (BS-011.11).
- The new-ticket **autoresponse** to the requester is sent only when all of the following hold: autoresponse was requested (and not overridden off), an outbound e-mail account is resolvable, the system-wide "autorespond on new ticket" setting is on, the receiving department's "autorespond on new ticket" setting is on, **and the resolved e-mail template group provides a new-ticket autoresponse message template**. The autoresponse uses the department's template/e-mail when available, otherwise the system defaults; if neither a department template nor a system default template can be resolved, the entire autoresponse-and-alert step is abandoned (no autoresponse and no staff alert are sent). (Templates and outbound delivery owned by FS-040.)
- The autoresponse body is additionally augmented: the department's signature is appended only when the department is **public**; and when the "strip quoted reply" setting is on and a reply separator tag is configured, the separator tag is prepended to the body.
- A **canned auto-response** (when a canned-response id is supplied — not a typical web submission) replaces the new-ticket autoresponse and leaves the ticket unanswered.
- The new-ticket **staff alert** is sent only when: staff alerting was requested, an alert e-mail account is resolvable, the "alert on new ticket" setting is on, and a new-ticket alert template exists. Recipients are computed from: the admin (if "alert admin on new ticket" is on), department members (only if the ticket is unassigned and "alert dept members" is on), and the department manager (if "alert dept manager" is on). Each recipient is alerted at most once and only if currently available.
- Autoresponse loop prevention: the autoresponse is suppressed if the requester's e-mail is one of the system's own e-mail accounts, if the inbound message looks like an auto-response, or if the requester's address begins with `mailer-daemon@` or `postmaster@`. (These guards primarily matter for the e-mail source — cross-ref FS-041 — but are applied uniformly in the shared creation routine.)

### FS-011.13: Public-Endpoint Protections (Ban-List, Open-Ticket Limit, Filters)

**Description**: The system shall protect the public endpoint by rejecting banned e-mail addresses, throttling requesters who have reached the open-ticket ceiling, and applying the inbound ticket-filter pipeline.

**Acceptance Criteria**:
- **Ban-list and open-ticket-limit checks are gated on a present, valid e-mail**: the up-front ban-list and open-ticket-limit checks run only when the submitted `email` is non-empty **and** parses as a valid e-mail address. When the e-mail is empty or malformed, both up-front checks are skipped and the missing/invalid e-mail is caught later by field validation (FS-011.8) instead. (See BS-011.9.)
- **Ban-list rejection**: if the requester's e-mail is on the ban-list, creation is denied with the user-facing message **"Ticket denied. Error #403"** (error number 403) and a warning logged as `Banned email - {email}`. (Ban-list management owned by FS-042.)
- **Open-ticket limit (up-front block)**: when a maximum-open-tickets ceiling is configured (> 0) and the source is not staff, the requester is blocked **only if a client record already exists for their e-mail** (`Client::lookupByEmail`) **and** that client's current open-ticket count is greater than zero **and** is at least the ceiling. A first-time e-mail with no prior ticket history is never blocked up-front (it has no client record / zero open tickets). When blocked, creation is denied with the message **"You've reached the maximum open tickets allowed."** and a warning is logged. (The default-priority/limit settings are owned by FS-032. See BS-011.10.)
- **Reaching the limit on this ticket**: if this very creation brings the requester's open-ticket count up to **exactly** the ceiling (count `==` ceiling, checked via the freshly created ticket's client), the ticket is still created but the over-limit handler runs — see BS-011.4 for the precise gating of the requester e-mail and admin alert (both are conditional, not unconditional).
- **Ticket-filter pipeline**: the inbound ticket-filter rules are applied **twice** in the shared creation routine — first, immediately after field validation, to detect a rejecting filter; then, after the fatal-error check, a second time to perform the routing rewrites. If the first pass yields a rejection (a `RejectedException`), creation is denied with **"Ticket denied. Error #403"** (error number 403) and a warning logged as `Ticket rejected ( {email}) by filter "{name}"`. Non-rejecting filters may rewrite routing variables (e.g., department, priority, assignment, auto-assignment, and the autoresponse flag) before creation; in particular, a filter rule may set an `autorespond` variable that overrides the autoresponse decision (BS-011.11). (Filter engine owned by FS-042.)

---

## Business Rules

### BS-011.1: Web Submitter Cannot Choose Department or Receiving E-mail

**Rule**: A web (public) submitter may select only a help topic; the submitted department id and e-mail-account id are forced to `0` before creation, so routing to a department is derived from the help topic (or the system default), never chosen directly.

**Rationale**: Public users should not be able to steer a ticket into an arbitrary internal department or impersonate an inbound e-mail account; help topics are the sanctioned public routing mechanism. (Comment in handler: "Just making sure we don't accept crap... only topicId is expected.")

**Examples**:
- A submitter selecting "Billing" help topic lands in the department that "Billing" is configured to route to.
- A crafted POST containing `deptId=5` is ignored; the topic's department (or default) is used.

### BS-011.2: Help Topic Required for Web Tickets

**Rule**: Every web-sourced ticket must specify a valid help topic; the help-topic field is required for source `Web` (it is not required for the e-mail source, where the receiving account governs routing).

**Rationale**: Help-topic-driven routing is the only routing input a public user is allowed to give, so it must be present to route the ticket correctly.

**Examples**:
- Leaving the topic dropdown on "— Select a Help Topic —" yields the error "Select help topic" and no ticket is created.
- An e-mailed ticket (FS-041) has no topic field and is routed by its receiving e-mail account instead.

### BS-011.3: Form-Chosen Priority Overrides Help-Topic Priority

**Rule**: When the requester explicitly chooses a priority on the form (and priority selection is allowed), that priority is used; otherwise the help topic's priority applies; failing both, the system default priority applies.

**Rationale**: An explicit user choice is the strongest signal; the help topic provides a sensible default; the system default is the final fallback so a ticket always has a priority.

**Examples**:
- Priority dropdown hidden (setting off): a "Report Outage" topic with priority "High" yields a High ticket.
- Priority dropdown shown and submitter picks "Low" on a "High"-defaulted topic: the ticket is Low.

### BS-011.4: Over-Limit Notice on Reaching the Ceiling

**Rule**: A requester who reaches (not exceeds) the maximum-open-ticket ceiling with the ticket they just created still gets that ticket, and the over-limit handler runs with the following gating:
- A warning is **always** logged to the system log (title `Max. Open Tickets Limit ({email})`).
- The handler is invoked with a "send notice" flag equal to `(autorespond is still true) AND (origin is not staff)`. If that flag is false, **or** the "send over-limit notice" setting (`overlimit_notice_active`) is off, the handler returns immediately after the warning log — **neither the requester e-mail nor the admin alert is sent**.
- Only when both the "send notice" flag is true **and** the "send over-limit notice" setting is on does the handler (a) e-mail the requester the over-limit auto-reply (using the department/template/e-mail resolution, falling back to system defaults) and (b) send an over-limit alert e-mail to the admin. The admin alert is **not** unconditional — it is gated behind the same condition as the user notice.

A requester who has **already** reached the ceiling before this submission is blocked outright (FS-011.13).

**Rationale**: The ceiling is a loop/flood control. Blocking only kicks in once the ceiling is already met; the courtesy notice on reaching it warns the requester that further tickets will be refused. The notice is suppressed when the autoresponse was already suppressed (loop control) or the source is staff, so internal/loop submissions do not generate user-facing or admin over-limit mail.

**Examples**:
- Ceiling = 3, requester had 2 open, creates a 3rd, `overlimit_notice_active` on, autorespond not suppressed: ticket created + requester e-mailed + admin alerted + warning logged.
- Ceiling = 3, requester had 2 open, creates a 3rd, `overlimit_notice_active` off: ticket created + warning logged only; no requester e-mail, no admin alert.
- Ceiling = 3, requester had 3 open, attempts a 4th: blocked up-front with "You've reached the maximum open tickets allowed." (no ticket created).

### BS-011.5: CAPTCHA Only Gates Anonymous Submissions

**Rule**: The CAPTCHA challenge is presented and enforced only for anonymous submitters when the feature is enabled; a valid signed-in client is never challenged. CAPTCHA matching is case-insensitive.

**Rationale**: A signed-in client is already authenticated (per-ticket session), so re-challenging them adds friction without benefit; the public anonymous endpoint is where automated abuse must be deterred.

**Examples**:
- Anonymous submitter with CAPTCHA enabled must type the shown text (any case).
- A logged-in client opening a second ticket sees no CAPTCHA row.

### BS-011.6: Ticket Number Is Never Displayed On-Screen at Creation

**Rule**: On the success/thank-you screen, all ticket-number tokens are masked to a placeholder (`XXXXXX`); the actual ticket number is delivered only via e-mail.

**Rationale**: Showing the ticket number on a public screen would let anyone observing (or an attacker who triggered the submission) learn a valid ticket reference, which combined with the requester e-mail grants portal access (per-ticket login model). E-mail delivery ties knowledge of the number to control of the mailbox.

**Examples**:
- An anonymous submission lands on the thank-you page reading "...ticket XXXXXX..." regardless of the real number.
- The requester learns the real number from the autoresponse e-mail (FS-011.12).

### BS-011.7: Identity Is Taken From the Session for Signed-In Clients

**Rule**: For a valid signed-in client, the requester name and e-mail are taken from the authenticated session and overwrite any submitted name/e-mail values; the identity fields render as static text.

**Rationale**: Prevents a logged-in client from opening a ticket under someone else's identity and keeps their ticket history coherent under their authenticated e-mail.

**Examples**:
- A client logged in as alice@example.com cannot open a ticket as bob@example.com even by editing the POST body.

### BS-011.8: Ban-List and Filter Rejections Return a Generic 403

**Rule**: Both a ban-list hit and a ticket-filter rejection return the same generic user-facing message "Ticket denied. Error #403" (error number 403); the specific reason is recorded only in the system log, not shown to the user.

**Rationale**: A generic rejection avoids telling an abuser exactly which control blocked them, while preserving an auditable internal record.

**Examples**:
- A banned address and an address rejected by a content filter both see "Ticket denied. Error #403".
- The log distinguishes "Banned email" from "Ticket rejected by filter '{name}'".

### BS-011.9: Ban-List and Limit Checks Require a Valid E-mail

**Rule**: The up-front ban-list and open-ticket-limit checks execute only when the submitted e-mail is both present and parses as a valid e-mail address; when the e-mail is empty or malformed, both checks are bypassed and the e-mail problem is instead reported by ordinary field validation.

**Rationale**: Both checks key on the e-mail address; without a parseable address neither a ban-list match nor an open-ticket count can be computed, so the checks are short-circuited and validation reports the bad e-mail as a normal field error.

**Examples**:
- A submission with a blank e-mail is never tested against the ban-list; it fails later with "Valid email required".
- A submission with a valid e-mail on the ban-list is rejected up-front with "Ticket denied. Error #403".

### BS-011.10: Up-Front Open-Ticket Block Depends on an Existing Client Record

**Rule**: The up-front open-ticket-limit block applies only when (a) the ceiling is configured greater than zero, (b) the source is not staff, (c) a client record already exists for the submitted e-mail, (d) that client's current open-ticket count is greater than zero, and (e) that count is at least the ceiling. A first-time e-mail (no prior client record, or zero open tickets) is never blocked up-front.

**Rationale**: The block is computed from the existing client's open-ticket count; an e-mail with no ticket history has no client record and a zero count, so it cannot trip the ceiling regardless of configuration. The "reach the ceiling on this very ticket" path (BS-011.4) is what catches the boundary case after the ticket is created.

**Examples**:
- Ceiling = 1, brand-new e-mail: first ticket is accepted (no prior record); the *next* attempt is blocked up-front because the client now has 1 open ticket.
- Ceiling = 3, e-mail with 3 open tickets: blocked up-front.

### BS-011.11: Filter Rules May Override the Autoresponse Flag

**Rule**: After the ticket-filter rewrite pass, if the new-ticket variables carry an `autorespond` value (set by a filter rule), that value replaces the autoresponse decision the caller passed in, before the autoresponse/alert and loop-prevention logic runs.

**Rationale**: Ticket filters are the inbound-routing customization point; allowing a rule to suppress (or force) the autoresponse lets administrators silence auto-replies for matching traffic (e.g. bulk/no-reply senders) without disabling the global setting.

**Examples**:
- A filter rule matching a no-reply pattern sets autorespond off; the matching ticket is created without a new-ticket autoresponse even though the global setting is on.

### BS-011.12: Attachment Processing Gate Differs From the Display Gate

**Rule**: The new-ticket form decides whether to *display* the attachment input from both the "allow online attachments" and "allow attachments on login only" toggles (plus the logged-in state), but the submission handler decides whether to *process* posted files from the "allow online attachments" toggle and the presence of files alone — it does not re-check the login-only restriction. A crafted POST can therefore submit attachments on a login-only configuration where the input was never shown.

**Rationale**: Documents an observed asymmetry between the view template's display condition and the handler's processing condition; it is a latent trust gap rather than an intended feature.

**Examples**:
- "Allow online attachments" on, "attachments on login only" on, anonymous crafted POST with files: the files are validated and attached even though the form would not have shown the input to an anonymous user.

---

## Data Requirements

> Table/column schema, the ticket-source / status / reference enum sets, and the configuration keys referenced below are **canonical to FS-091** (reference data & data model) and **FS-032** (system settings). They are summarized here only as they pertain to the web-submission form.

### Submitted Form Variables (source = `Web`)

| Variable | Meaning | Required (Web) | Notes |
|----------|---------|----------------|-------|
| `name` | Requester full name | Yes (unless signed-in client) | trimmed, tags stripped on persist; overridden from session for signed-in clients |
| `email` | Requester e-mail | Yes (unless signed-in client) | trimmed, validated as e-mail; key for ban-list / limit / portal login |
| `phone` | Telephone | No | validated as phone |
| `phone_ext` | Phone extension | No | numeric; requires a base phone number |
| `topicId` | Help topic id | Yes | drives dept/priority/assignment/SLA |
| `subject` | Ticket subject | Yes | tags stripped on persist; also used as initial message title |
| `message` | Ticket body | Yes | stored as the initial thread message |
| `priorityId` | Chosen priority | No | only when priority change allowed; overrides topic priority |
| `captcha` | CAPTCHA answer | Conditional | only for anonymous submitters when enabled |
| `attachments[]` | Uploaded files | No | only when attachments allowed; validated for type/size |
| `a` (hidden) | Action discriminator `open` | — | form-internal |
| CSRF token | — | — | required by the shared POST CSRF guard (FS-001/FS-002) |
| `deptId`, `emailId` | — | — | forced to `0` by the handler (BS-011.1) |

### Derived / Resolved Ticket Attributes

| Attribute | Source of value |
|-----------|-----------------|
| source | literal `Web` |
| dept_id | help topic's department → system default department |
| topic_id | submitted `topicId` (else `0`) |
| priority_id | form `priorityId` → topic priority → system default priority |
| sla_id | topic SLA → system default SLA |
| staff_id / team_id | topic default staff / team (auto-assignment); may be overwritten by filter rules |
| ip_address | request remote address (web) |
| public ticket reference | random numeric (configured length, unique) or sequential internal id |
| created / lastmessage | creation timestamp |

### Relevant Configuration Keys (owned by FS-032 / canonical in FS-091)

| Capability | Effect on this form |
|------------|---------------------|
| enable CAPTCHA (+ image capability) | shows/enforces the CAPTCHA challenge for anonymous submitters (FS-011.5) |
| allow priority change | shows the priority dropdown (FS-011.6) |
| allow online attachments / attachments on login only | shows the attachment input (FS-011.7) |
| max file size / allowed file types | validates uploaded attachments (FS-011.7) |
| max open tickets | open-ticket throttle (FS-011.13, BS-011.4) |
| send over-limit notice | whether the requester is e-mailed on reaching the ceiling (BS-011.4) |
| default priority / default department / default SLA / default template / default e-mail | fallbacks when topic does not supply them |
| autorespond on new ticket / alert on new ticket (+ admin/dept-member/dept-manager toggles) | autoresponse & alert behavior (FS-011.12) |
| use random ids | public ticket reference scheme (FS-011.9) |
| show related tickets | post-creation session-key behavior for signed-in clients (FS-011.10) |
| thank-you page id | system thank-you page fallback (FS-011.10) |

### Help Topic (read-only here; owned by FS-030)

Each public option exposes: topic id, display name (`Parent / Child` for sub-topics), department, priority, default staff/team, SLA, autoresponse flag, and an optional thank-you page. Only topics with `isactive=1 AND ispublic=1` appear.

---

## User Flows / Interactions

### Flow 1: Anonymous Submitter Creates a Ticket (CAPTCHA on)

1. The user opens `open.php`; the empty "Open a New Ticket" form renders with the help-topic dropdown populated from public topics and a CAPTCHA image.
2. The user fills in Full Name, Email, (optionally Telephone), selects a Help Topic, types a Subject and Message, (optionally attaches files / picks a priority), and enters the CAPTCHA text.
3. The user clicks "Create Ticket"; the form POSTs with the CSRF token.
4. The handler checks CAPTCHA, ban-list, open-ticket limit, validates fields, and applies the filter pipeline.
5. On success the ticket is created, routed by the help topic, the initial message is posted, the autoresponse e-mail (with the real ticket number) and staff alert are sent, and a `created` event is logged.
6. The thank-you page renders (topic-specific or system), with the ticket number masked to `XXXXXX`; the user retrieves the real number from the autoresponse e-mail.

### Flow 2: Signed-In Client Opens an Additional Ticket

1. A logged-in client opens `open.php`; the Full Name and Email render as static session text, phone fields pre-filled, and **no** CAPTCHA row appears.
2. The client selects a Help Topic, types a Subject and Message, optionally attaches files (if attachments-on-login is allowed), and submits.
3. The handler overrides name/email from the session, validates, and creates the ticket.
4. On success the session is regenerated and the browser is redirected to the new ticket's view (`tickets.php?id={ref}`); if "show related tickets" is off, the session login key is reset to this new ticket.

### Flow 3: Submission Fails Validation

1. The user submits with a missing Subject and an invalid e-mail.
2. The handler collects field errors ("Subject required", "Valid email required") and sets the form-level error.
3. The form re-renders with the entered values preserved (HTML-escaped), each error shown inline, a fresh CAPTCHA image (for anonymous users), and the top-level error "Unable to create a ticket. Please correct errors below and try again!" (or the routine's specific message).

### Flow 4: Banned / Filtered / Over-Limit Rejection

1. The user submits a valid-looking form.
2. The handler finds the e-mail on the ban-list (or a filter rejects it, or the open-ticket ceiling is already met).
3. No ticket is created; the form re-renders showing the generic rejection ("Ticket denied. Error #403") or the limit message ("You've reached the maximum open tickets allowed."); a warning is logged internally with the specific reason.

---

## Edge Cases & Error Scenarios

### EC-011.1: No Public Help Topics Configured

**Scenario**: No active public help topics exist.
**Expected Behavior**: The dropdown shows a single fallback option "General Inquiry" with value `0`. A submission with topic `0` still satisfies the required-int validation (value present), so the ticket routes to the system default department/priority. (See KL-011.3.)

### EC-011.2: Anonymous Submitter With Empty / Wrong CAPTCHA

**Scenario**: CAPTCHA enabled; the field is blank or mismatched.
**Expected Behavior**: Creation is blocked with "Enter text shown on the image" (blank) or "Invalid - try again!" (mismatch); on re-render with no specific CAPTCHA error, the prompt becomes "Please re-enter the text again" and a new challenge image loads.

### EC-011.3: Phone Extension Without a Phone Number

**Scenario**: The user enters an extension but no base phone number.
**Expected Behavior**: Validation fails with "Phone number required"; if the extension is non-numeric, "Invalid phone ext." is shown instead.

### EC-011.4: Crafted POST Tries to Set Department / E-mail / Priority Not Offered

**Scenario**: A hand-crafted POST includes `deptId`, `emailId`, or a `priorityId` for a priority the form never displayed.
**Expected Behavior**: `deptId` and `emailId` are forced to `0` (BS-011.1); `priorityId` is validated as an optional integer and, if the dropdown was hidden, simply contributes to the priority resolution chain or is overridden by topic/default — the submitter cannot pick a department or receiving account.

### EC-011.5: Attachment Type / Size Violation

**Scenario**: The user attaches a disallowed-type or oversized file.
**Expected Behavior**: The file-format routine sets a per-file error (type/size validation contract + error strings owned by FS-022); the error is shown beside the Attachments field and creation does not proceed until corrected.

### EC-011.6: Requester E-mail Is One of the System's Own Accounts / Mailer-Daemon

**Scenario**: The submitted e-mail matches a system e-mail account, looks like an auto-response, or begins with `mailer-daemon@` / `postmaster@`.
**Expected Behavior**: The ticket is still created, but the new-ticket autoresponse is suppressed (loop prevention); staff alerts may still fire per their settings.

### EC-011.7: Success but No Thank-You Page Configured

**Scenario**: A ticket is created (anonymous) but neither the help topic nor the system has a thank-you page.
**Expected Behavior**: Instead of a confirmation page, a fresh empty new-ticket form is re-rendered. The requester receives confirmation (with the real number) only via the autoresponse e-mail. (See KL-011.4.)

### EC-011.8: Random Ticket-Reference Collision

**Scenario**: A freshly generated random ticket reference collides with an existing one.
**Expected Behavior**: The reference generator detects the collision and regenerates until the reference is unique before insert.

### EC-011.9: Ticket Insert Succeeds but Sequential-ID Backfill Fails

**Scenario**: With random IDs disabled, the post-insert update that copies the sequential id into the public-reference column fails.
**Expected Behavior**: The system leaves the originally generated random reference in place so the ticket remains usable (documented as a TODO in source). (See KL-011.2.)

### EC-011.10: CAPTCHA Failure Suppresses Attachment Validation on the Same Pass

**Scenario**: An anonymous submitter fails the CAPTCHA on a submission that also carried files.
**Expected Behavior**: Because the handler formats/validates attachments only when no prior error exists, the CAPTCHA error short-circuits attachment processing on that pass; the submission fails on CAPTCHA, the file input cannot be repopulated (browsers do not echo file inputs), and the submitter must re-attach the files on the corrected resubmission. (See KL-011.6.)

### EC-011.11: Up-Front Limit Block vs. First-Time E-mail

**Scenario**: The open-ticket ceiling is configured but the submitter's e-mail has never been used before.
**Expected Behavior**: The up-front block does not fire (no client record / zero open tickets); the ticket is created. If the ceiling is `1`, the *next* submission from the same e-mail is blocked up-front. (See BS-011.10.)

---

## Dependencies

| Dependency | Specification | Relationship |
|------------|---------------|--------------|
| Client request lifecycle, CSRF, client session (`$thisclient`) | FS-001 / FS-002 / FS-010 | Boots the public endpoint; provides the signed-in client identity and CSRF guard |
| Public client portal — header/footer, ticket view, related tickets | FS-010 | Renders the form chrome; receives the post-creation redirect for signed-in clients |
| Staff ticket creation (`Ticket::open`, thread/message posting) | FS-021 | Shares the `Ticket::create` core; owns the message-thread internals and staff-source fields |
| Inbound e-mail ticket creation | FS-041 | Shares `Ticket::create`; owns the e-mail-source fields and autoresponse loop guards |
| External API ticket creation | FS-043 | Shares `Ticket::create`; owns the api-source path |
| Help topics, departments, teams | FS-030 | Provide the routing targets the web form selects via help topic |
| Priorities, SLA plans, system settings | FS-032 | Provide priority/SLA values and the configuration toggles gating the form |
| Ban-list & ticket-filter engine | FS-042 | Reject banned/filtered submissions; rewrite routing variables |
| E-mail templates & outbound delivery (autoresponse, alerts, over-limit notice) | FS-040 | Compose and send the new-ticket e-mails |
| Attachment storage (content-addressed, chunked) | FS-022 | Stores validated uploaded files against the initial message |
| Custom pages (thank-you pages) | FS-033 | Provide the thank-you page bodies |
| Reference data, enums & schema (ticket source/status/reference, table/column schema) | FS-091 | Canonical data model the ticket record conforms to |
| Validation & formatting infrastructure (e-mail/phone validation, tag stripping, file-size formatting) | FS-003 | Shared validators/formatters used during submission |

---

## Known Limitations

### KL-011.1: CAPTCHA Silently Disabled Without Image Capability

**Limitation**: CAPTCHA is treated as "enabled" only when both the admin setting is on **and** the server's image-generation capability is present; if the capability is missing, CAPTCHA is silently treated as disabled even though the administrator turned it on.

**Why It Exists**: The challenge image cannot be rendered without the image capability, so the gate degrades to "off" rather than erroring.

**Impact**: An administrator may believe CAPTCHA is protecting the public endpoint when, on a server lacking the image capability, anonymous submissions are accepted without any challenge. No on-screen warning surfaces this to the public form.

### KL-011.2: Sequential Public-Reference Backfill Has No Recovery Path

**Limitation**: When sequential (non-random) ticket references are configured, the public reference is first generated as a random number and then overwritten by the sequential internal id in a follow-up update; if that update fails, the random reference silently remains and there is no retry or reconciliation (flagged as a TODO in source).

**Why It Exists**: The implementation reuses the random-reference generator and patches in the sequential id post-insert for simplicity.

**Impact**: In the rare failure case, a ticket configured for sequential references ends up with a random one; the inconsistency is tolerated to keep the ticket usable.

### KL-011.3: "General Inquiry" Fallback Routes to System Defaults Only

**Limitation**: When no public help topics exist, the form offers only "General Inquiry" (topic id `0`), which carries no department/priority/SLA/assignment of its own; such tickets fall entirely to the system defaults.

**Why It Exists**: A placeholder option keeps the required help-topic field satisfiable even on a freshly configured system.

**Impact**: A system with no configured public topics provides no topic-based routing for public tickets; all such tickets land in the default department at default priority.

### KL-011.4: Anonymous Success Without a Thank-You Page Looks Like a No-Op

**Limitation**: If neither a topic-specific nor a system thank-you page is configured, a successful anonymous submission re-renders a blank form rather than any explicit success confirmation; the only confirmation is the autoresponse e-mail.

**Why It Exists**: The success view depends on a configured thank-you page; absent one, the handler falls through to re-rendering the form.

**Impact**: A misconfigured system can make a successful submission appear to have done nothing (no on-screen confirmation), risking duplicate submissions by confused users.

### KL-011.5: Generic 403 Hides the Rejection Reason From the User

**Limitation**: Ban-list and filter rejections both return the same opaque "Ticket denied. Error #403"; the user cannot tell why their ticket was refused, and there is no appeal/contact affordance on the form.

**Why It Exists**: Deliberate — to avoid leaking which control blocked an abuser (BS-011.8).

**Impact**: A legitimately mis-banned or mis-filtered user has no on-screen guidance; only an administrator inspecting the logs can diagnose the rejection.

### KL-011.6: Attachments Are Lost When Another Field Fails Validation

**Limitation**: Because the handler validates uploaded files only when no other submission error exists, and because the file input cannot be repopulated on re-render, a submission that fails on any other field (CAPTCHA, missing subject, invalid e-mail, etc.) silently discards the uploaded files — the submitter must re-select every attachment on each corrected resubmission.

**Why It Exists**: Attachment formatting is gated behind the no-error precondition, and HTTP file inputs are not echoed back by the browser for security reasons, so there is no mechanism to preserve uploads across a failed round-trip.

**Impact**: On forms with CAPTCHA (the common anonymous case), a submitter who mistypes the CAPTCHA loses their attachments and must re-add them, increasing friction and abandonment for legitimate users.

### KL-011.7: Over-Limit Admin Alert Cannot Be Sent Independently of the User Notice

**Limitation**: The over-limit admin alert and the over-limit user e-mail are governed by the same gate (the "send over-limit notice" setting plus the autorespond/non-staff condition). An administrator cannot be alerted that a requester reached the ceiling unless the user-facing over-limit notice is also enabled and eligible to send.

**Why It Exists**: The over-limit handler returns early when notice-sending is disabled, before reaching the admin-alert step; only the warning log is unconditional.

**Impact**: With the over-limit notice disabled, the only record of a requester reaching the ceiling is the system warning log — no proactive admin alert e-mail is generated.

---

## Future Considerations

- An on-screen success confirmation (with a "your ticket has been received" message and a link, but still without the number) for the case where no thank-you page is configured (KL-011.4).
- A configurable, non-image CAPTCHA / challenge mechanism (or an explicit admin warning) when the image-generation capability is unavailable (KL-011.1).
- Custom / arbitrary intake fields per help topic on the public form (the 1.7 form is fixed to name/email/phone/topic/subject/message + optional priority/attachments).
- A user-facing "contact us if you believe this is an error" affordance on the generic 403 rejection (KL-011.5).
- Client-side validation and progressive disclosure (e.g., showing topic-specific guidance) before submission.
- Reconciliation/retry for the sequential public-reference backfill failure (KL-011.2).
