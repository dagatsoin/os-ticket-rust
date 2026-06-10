# FS-040: Email Accounts, Templates & Outbound Mail

## Overview

This specification covers the administrator-facing configuration of **email accounts** (the help-desk mailboxes the system sends from and — when enabled — fetches inbound mail into), the **email template sets** that drive every system-generated message (auto-responses, alerts, replies, notices), the **variable-substitution grammar** (`%{token}`) that personalizes those templates per ticket, and the **outbound mail composition** rules that assemble and dispatch a message (from-address selection, SMTP-vs-native routing, reply-threading headers, attachment encoding).

It owns the behavior of three control-panel pages — the Email Addresses manager (`emails.php`), the Email Templates manager (`templates.php`), and the per-message template editor — plus the supporting domain classes `Email` (`ost_email`), `EmailTemplateGroup` / `EmailTemplate` (`ost_email_template_group` / `ost_email_template`), the `Mailer` outbound composer, and the `VariableReplacer` substitution engine.

**Scope boundary**: This spec documents email accounts *behaviorally* — what fields they carry, the validation/connection checks performed at save time, and how an account is selected as the sender. The **execution of inbound mail fetching** (the periodic POP3/IMAP poll that turns mailbox messages into tickets) is owned by **FS-041 (Inbound email pipeline — fetch, pipe & parse)**; this spec only covers the account-level fetch *settings* and the one-shot connection validation done when those settings are saved. **Ticket filters** that route or reject inbound mail are owned by **FS-042**. The **system-wide email defaults** (default email, alert email, default SMTP account, default template set, admin email, autoresponder master switches) are persisted in the configuration store and are owned by **FS-032 (Admin — system settings)**; this spec references those config keys where they gate behavior here but does not own their editing UI. Canonical table/column schemas and enum sets are owned by **FS-091 (Reference data, enums & data model)**; this spec references column and enum names as behavioral facts but does not restate the full DDL.

---

## Functional Requirements

### FS-040.1: Email Account Listing

**Description**: The system shall present administrators a paginated, sortable list of all configured help-desk email accounts.

**Acceptance Criteria**:
- The list shows, per account: the display address (the `name <email>` form when a name is set, otherwise the bare email), the associated **New Ticket Priority**, the associated **New Ticket Department** (linked to the department editor), the created date, and the last-updated datetime.
- Columns **Email**, **Priority**, **Department**, **Created**, and **Last Updated** are individually sortable ascending/descending; default sort is **Email** ascending.
- The list is paginated using the system page limit; a "showing N emails" / "No emails found!" caption reflects the result count.
- Each row carries a selection checkbox feeding the bulk action (FS-040.4). The checkbox for the row that is the **system default email** is rendered disabled so it cannot be selected for deletion.
- An "Add New Email" action opens the create form (FS-040.2).
- Access is restricted to authenticated staff with the administrator flag; non-admins are denied (`Access Denied`).

### FS-040.2: Email Account Create / Edit Form

**Description**: The system shall provide a single form for creating a new email account and for editing an existing one, capturing the account's identity, ticket-routing defaults, mailbox fetch settings, outbound SMTP settings, and internal notes.

**Acceptance Criteria**:
- The form collects the following groups of inputs:
  - **Identity**: Email Address (required), Email Name / display name (required).
  - **New-ticket routing defaults**: New Ticket Priority, New Ticket Department.
  - **Auto-response override**: a "Disable new ticket auto-response for this email" checkbox (`noautoresp`) that overrides global and department settings for tickets arriving on this address.
  - **Login credentials**: Username and Password — labeled optional in general but required when mailbox fetching or authenticated SMTP is enabled.
  - **Mail Account (inbound fetch)**: enable/disable status, Host, Port, Protocol (POP or IMAP), Encryption (NONE or SSL), Fetch Frequency (minutes), Emails Per Fetch (max), and a **Fetched Emails** post-processing choice: *Move to <folder>* (archive), *Delete fetched emails*, or *Do nothing*.
  - **SMTP (outbound)**: enable/disable status, SMTP Host, SMTP Port, Authentication Required (yes/no), and an Allow Header Spoofing checkbox.
  - **Internal Notes**: free-text admin notes.
- On edit, the form is pre-filled from the stored account; the post-fetch radio is reconstructed from the stored state (`delete` if mail-delete is set, `archive` if an archive folder is set, otherwise none).
- On edit, the password field is left blank and a hint ("To change password enter new password above.") is shown; submitting a blank password preserves the existing stored password (FS-040.7).
- The create form defaults the auto-response toggles to enabled.
- Save invokes the create or update path (FS-040.3); validation errors are redisplayed inline against the offending fields and the user's submitted values are retained.

### FS-040.3: Email Account Save Validation & Provisioning

**Description**: When an email account is created or updated, the system shall validate the inputs, run live connection checks against any enabled mailbox/SMTP server, and persist the account only when all checks pass.

**Acceptance Criteria** (validation, in the order applied):
0. **Hidden-id integrity guard (edit only)**: when editing, the submitted hidden record id must match the account being edited; a mismatch produces a generic "Internal error. Get technical help." error (a tamper / stale-form guard) (BS-040.25).
1. **Email required & well-formed**: a valid email address is required.
2. **Email uniqueness**: the address must not already belong to another email account (BS-040.1), must not equal the system **admin email**, and must not be in use by any staff member.
3. **Name required**: a non-empty display name is required (HTML tags are stripped).
4. **Credentials when needed**: if mailbox fetching is enabled, or if SMTP with authentication is enabled, a Username is required; on create, a Password is required; a supplied password must encrypt successfully.
5. **Mailbox fetch fields (only when fetch enabled)**: the runtime must support IMAP; Host, Port, Protocol, numeric Fetch Frequency, numeric Emails-Per-Fetch, Department, and Priority are all required; a post-fetch action must be indicated, and if "archive" is chosen an archive folder name is required.
6. **SMTP fields (only when SMTP enabled)**: SMTP Host and SMTP Port are required.
7. **Host/userid combination uniqueness**: no two accounts may share the same mailbox host + username pair (BS-040.2).
- **Live mailbox check** (only when fetch enabled and field validation passed): the system attempts a connection with the supplied credentials; a failed login produces a protocol-settings error with the underlying connection error appended. If an archive folder is named, the system verifies that mailbox folder exists; an unknown folder produces a post-fetch error.
- **Live SMTP check** (only when SMTP enabled and field validation passed): the system opens an SMTP session with the supplied host/port/credentials; a failure produces an "Unable to log in. Check SMTP settings." error with the underlying message; on success the session is immediately disconnected.
- If priority or department were left unset, they default to the system default priority and default department respectively before persisting.
- **Side-effects on every successful save (create or update)**: the fetch operational counters are reset — `mail_errors` is set to 0 and `mail_lastfetch` is cleared to null — so a freshly (re)configured account starts a clean fetch cycle (BS-040.26).
- **Persisted defaults / coercions**: `mail_protocol` defaults to `POP` when blank; `mail_port`, `mail_fetchfreq`, `mail_fetchmax`, and `smtp_port` each default to 0 when blank/absent; the `noautoresp` and `smtp_spoofing` flags persist as 1 when their checkbox is present, else 0 (BS-040.26).
- **Post-fetch persistence mapping**: a `delete` choice persists `mail_delete=1` and clears the archive folder; an `archive` choice with a folder persists `mail_delete=0` and the folder; any other case persists `mail_delete=0` and clears the archive folder (BS-040.26).
- On successful create, a new account id is returned and a success message shown; on successful update, the account is reloaded and a success message shown. A failed insert/update yields "Unable to add/update email. Internal error". Any error aborts the save and redisplays the form.

### FS-040.4: Email Account Bulk Delete

**Description**: The system shall allow administrators to delete one or more selected email accounts, subject to in-use and default-account protections.

**Acceptance Criteria**:
- The bulk action requires at least one selected account; an empty selection is an error.
- Before deleting, the system checks whether any selected account is referenced by a department as either its inbound email or its autoresponse email; if so, the entire delete is refused with a "remove association first" error (BS-040.3).
- The **system default email** account is excluded from deletion (its checkbox is disabled, and the loop additionally skips it) (BS-040.4).
- The **configured alert email** account is also undeletable: the delete operation itself returns 0 (nothing removed) when invoked on the default email or the alert email account. Thus an alert-email account selected in the batch is silently left in place even though only the default account's checkbox is disabled (BS-040.4).
- When an account is deleted, any department row that pointed its `email_id` at the deleted account is reassigned to the system default email, and that department's `autoresp_email_id` is cleared to 0 (BS-040.5).
- The result message reports full success, a partial count ("N of M deleted"), or failure.
- Deleted accounts cannot be recovered (the confirmation dialog states this).

### FS-040.5: Email Template Set Listing

**Description**: The system shall present administrators a paginated, sortable list of all email template **sets** (template groups).

**Acceptance Criteria**:
- The list shows per set: Name, Status (Active / Disabled), In-Use (Yes / No), Date Added, and Last Updated.
- A set is **In-Use** when at least one department references it, or when it is the configured **system default template set** (BS-040.6).
- The system default template set is labeled "(System Default)" and its selection checkbox is disabled so it cannot be bulk-processed.
- Columns Name, Status, Created, and Updated are sortable; default sort is Name ascending.
- An "Add New Template" action opens the create form (FS-040.8).
- Bulk actions available: Enable, Disable, Delete (FS-040.9).
- Access is restricted to administrators.

### FS-040.6: Template Set Manage View (Message List)

**Description**: When an administrator opens a template set, the system shall list every message template defined in that set and every still-undefined message type, so each can be edited or implemented.

**Acceptance Criteria**:
- The set's editable attributes are shown: Name, Status (Active/Disabled), Language (fixed to English (US) in this version — see KL-040.3), and Admin Notes.
- The set lists each **implemented** message template by its human-readable name and description, each linking to the per-message editor (FS-040.7-template variant).
- The set lists each **unimplemented** message type (any of the canonical message types not yet present in this set) under an "Unimplemented Template Messages" heading, each linking to an "implement" action that creates that message in the set.
- The canonical set of message types and their names/descriptions is fixed; the literal catalog (code name → name → description) is owned by **FS-091.7** (BS-040.7 / Data Requirements — Template Type Catalog).

### FS-040.7: Per-Message Template Editing

**Description**: The system shall allow administrators to edit the subject line and body of any individual message template within a set, and to implement a previously-undefined message type into a set.

**Acceptance Criteria**:
- The editor captures a **Subject** (required) and a **Body** (required).
- On update, the subject and body of the existing message are saved.
- On **implement**, a new message row is created in the target set bound to the chosen message type's code name, with the submitted subject and body; the code-name and target set are required for a create.
- Subject and body may contain `%{token}` variables (FS-040.11); the editor does not resolve them — substitution happens at send time.
- A "Ticket Variables" reference (the variable catalog, FS-040.11) is available to administrators while editing.

### FS-040.8: Template Set Create (Clone-Based)

**Description**: The system shall create a new template set by cloning an existing set, so the new set starts with a full complement of message templates.

**Acceptance Criteria**:
- The create form collects: Name (required, unique — BS-040.8), Status (Active/Disabled, default Disabled), Language, a required "Template To Clone" selection (an existing set), and Admin Notes.
- On save, the new set row is inserted, then **every** message template belonging to the source set is copied (code name, subject, body) into the new set (BS-040.9).
- Name uniqueness is enforced; a duplicate name is rejected.
- After creation the administrator is taken to the new set's manage view where individual messages can be edited.

### FS-040.9: Template Set Bulk Actions (Enable / Disable / Delete)

**Description**: The system shall allow administrators to enable, disable, or delete selected template sets, subject to in-use protection.

**Acceptance Criteria**:
- **Enable**: selected sets are marked active; the result reports full or partial success.
- **Disable**: a set is disabled only if it is **not in use** (BS-040.10); in-use sets (including the default set) are skipped, and the message notes that in-use/default sets cannot be disabled.
- **Delete**: a set is deleted only if it is **not in use** and is **not the default set** (BS-040.11); when a set is deleted, any department referencing it has its template reference cleared to 0, and all of that set's message-template rows are removed (BS-040.12).
- A single-set update (via the set form) additionally refuses to disable a set that is in use ("Template in use cannot be disabled!").
- The bulk action requires at least one selection.
- Deleted sets cannot be recovered (confirmation dialog states this).

### FS-040.10: Template Resolution Fallback

**Description**: When the system needs a specific message template (e.g., the new-ticket auto-response), it shall resolve it from the active template set, falling back to packaged default content when the set has no row for that message type.

**Acceptance Criteria**:
- Given a message type's code name, the system first looks up a stored message row for that code name in the relevant set.
- If no stored row exists, the system loads the message from a packaged initial-data file located at `<i18n-dir>/<language>/templates/<code-name>.yaml`, where `<language>` is the set's language (always `en_US` in this version — KL-040.3). The file is a structured document that **must** define both a `subject` and a `body` key; the loader maps those two keys onto the resolved template's subject and body (BS-040.13).
- If the packaged file is missing either the `subject` or `body` key, an `InitialDataError` is raised (the file is treated as malformed) and no fallback template is produced (EC-040.12).
- If neither a stored row nor packaged default can be found, the failure is logged as a warning ("Template Fetch Error — Unable to fetch '`<name>`' template") and template fetch returns empty (no message is produced).

### FS-040.11: Variable Substitution Grammar

**Description**: The system shall substitute `%{token}` placeholders in template subjects and bodies (and canned responses) with live values resolved from the ticket and surrounding context at send time.

**Acceptance Criteria**:
- A token has the literal form `%{` + name + `}` where name matches an identifier that begins with a letter or underscore and is followed by **one or more** word characters, dots, or underscores (e.g., `%{ticket.number}`, `%{url}`, `%{ticket.dept.name}`). Because at least one trailing character is required, a single-character token name (e.g., `%{x}`) is NOT recognized as a token and is left untouched (BS-040.14).
- Token names are **dot-paths**: the first segment names an assigned **object** or a scalar **variable**; each further segment after a dot resolves against the previous result (nested traversal) (BS-040.14).
- Resolution of a dot segment against an object calls that object's accessor for the segment (the `get`+CapitalizedSegment convention); if the accessor returns another object, traversal continues into it; if it returns a scalar, that scalar is the value (BS-040.15).
- A bare object token (no dot suffix) resolves to that object's self-string representation when the object provides one (its `asVar` form); otherwise it resolves to empty (BS-040.16).
- An **unknown** object or variable token is **left unchanged** in the output and recorded as a substitution error (the original `%{...}` text remains) (BS-040.17).
- Substitution is applied uniformly to a single string or recursively across an array of strings (e.g., the `{subject, body}` pair of a resolved template) (BS-040.18).
- Every substitution context implicitly provides `%{url}` = the help desk's configured base URL (FQDN) (BS-040.19).
- Ticket-driven substitution implicitly provides `%{ticket}` bound to the current ticket, exposing the ticket's base variables and expandable sub-objects (topic, dept, staff, team).
- Certain alert recipients are personalized by a direct textual replacement of `%{recipient}` with the recipient's first name (or "Admin" for the admin recipient) after the general substitution pass (BS-040.20).
- The canonical variable catalog (base ticket variables, expandable objects, and context variables) is fixed and exposed to administrators as a reference (Data Requirements — Variable Catalog).

### FS-040.12: Outbound Mail Composition & From-Address Selection

**Description**: When sending any message, the system shall compose a MIME email with appropriate headers, choose the sending (from) address, and route the message via SMTP when configured or via the platform's native mail transport otherwise.

**Acceptance Criteria** (sender/account selection — BS-040.21):
- A send is performed through a `Mailer` bound to an originating email account. Account/transport selection at construction follows this precedence:
  1. If the supplied account has SMTP enabled, use that account's SMTP transport.
  2. Otherwise, if a **global default SMTP account** is configured and SMTP-enabled, use it; if that global account disallows header spoofing (or no specific account was supplied), the global account also becomes the sending identity.
  3. Otherwise, if no account was supplied, fall back to the **system default email** account as the sending identity.
- The **From** header is `"Display Name" <address>` of the resolved sending account (name falls back to the bare email when no name is set), unless an explicit from-address was set on the mailer.

**Acceptance Criteria** (composition & headers):
- The recipient, subject, and body are whitespace/newline-cleaned; the body is decoded from HTML entities to plain text (this version sends **plain text only** — see KL-040.1).
- Each outbound message gets a generated unique **Message-ID** incorporating a random code, a timestamp, and the sending account's address; an `X-Mailer: osTicket Mailer` header; and a `Date` header.
- A **Return-Path** is set to the sending account's address, or to the null path `<>` when the message is flagged no-bounce.
- Message-class headers are added per send options: **bulk** adds `Precedence: bulk`; **autoreply** adds `Precedence: auto_reply`, `X-Autoreply: yes`, `X-Auto-Response-Suppress: DR, RN, OOF, AutoReply`, and `Auto-Submitted: auto-replied`; **notice** adds `X-Auto-Response-Suppress: OOF, AutoReply` and `Auto-Submitted: auto-generated` (BS-040.22).
- Threading headers `In-Reply-To` and `References` are added when supplied so replies thread into the originating conversation.
- The body and headers are MIME-encoded with UTF-8 charset, base64 text encoding, and quoted-printable header encoding.
- **Line-ending selection**: the MIME line ending is chosen as follows (BS-040.28) — if a `MAIL_EOL` config constant is defined as a string, use it; otherwise, if a security hardening patch is detected (Suhosin extension loaded or its patch constant defined) AND the message is being sent without SMTP, force a single `\n`; otherwise let the MIME library use its platform default.
- Attachments are added either by stored-file reference (looked up and inlined by content) or by readable file path. A stored-file reference that cannot be looked up, or a file path that does not exist / is not readable, is silently skipped (the message still sends without that attachment).

**Acceptance Criteria** (delivery & fallback — BS-040.23):
- When SMTP is selected, the SMTP connection is established (and cached per host:port:username within the request for reuse); the message is sent over it; success returns the generated Message-ID.
- If the SMTP send fails, the cached connection is dropped, the error is logged (without itself emailing an alert, to avoid loops), and the system falls back to the native mail transport.
- When no SMTP is configured (or after SMTP failure), the message is sent via the native transport; success returns the Message-ID, failure returns a false/failure result.

### FS-040.13: Specialized Send Wrappers

**Description**: The system shall provide convenience send paths that pre-apply the correct message-class options.

**Acceptance Criteria**:
- An **auto-reply** send applies the `autoreply` option set (FS-040.12 headers).
- An **alert/notice** send applies the `notice` option set.
- A low-level static send (used when no database/account context is available) sends as a no-bounce notice with an explicit from-address, via the native transport.

---

## Business Rules

### BS-040.1: Email Address Uniqueness
**Rule**: No two email accounts may share the same email address; additionally the address may not equal the configured admin email and may not belong to any staff member.
**Rationale**: The address is the routing key for inbound mail and the identity for outbound mail; collisions would misroute tickets and ambiguate sender identity. Reserving the admin and staff addresses prevents the help desk from impersonating or looping with internal accounts.
**Examples**: Creating a second `support@example.com` account is rejected ("Email already exists"). Using the admin's own address is rejected ("Email already used as admin email!").

### BS-040.2: Mailbox Host + Username Uniqueness
**Rule**: No two accounts may share the same mailbox host and username pair.
**Rationale**: Two accounts polling the same mailbox would double-process inbound mail.
**Examples**: Two accounts both set to fetch from `mail.example.com` with username `helpdesk` are rejected ("Host/userid combination already in use.").

### BS-040.3: Department Association Blocks Deletion
**Rule**: An email account that a department references (as its inbound email or its autoresponse email) cannot be deleted until the association is removed.
**Rationale**: A department with a dangling email reference would lose its inbound routing or autoresponder identity.

### BS-040.4: System Default Email and Alert Email Are Undeletable
**Rule**: At the model level, the account-delete operation refuses (returns 0, deletes nothing) when the account is **either** the configured system default email **or** the configured alert email. The bulk-delete page loop independently skips only the system default email (its checkbox is disabled), so a selected alert-email account is still protected because the underlying delete operation itself returns 0 for it.
**Rationale**: Outbound mail composition falls back to the default email as the system's last-resort sending identity, and the alert email is the configured sender for staff alerts; deleting either would break fallback or alert sending. The two-layer guard (page loop + model guard) ensures the alert email survives even though only the default email's checkbox is disabled in the list.
**Note**: A stricter consequence is that an alert-email account selected for bulk delete is silently NOT deleted (the loop attempts it, `delete()` returns 0, and the row stays); it does not surface a dedicated "alert email" error, only counting against the success total.

### BS-040.5: Deletion Reassigns Dependent Departments to Default
**Rule**: When an email account is deleted, any department that pointed its inbound email at it is reassigned to the system default email, and that department's autoresponse-email reference is cleared.
**Rationale**: Keeps departments mailable after their bound account is removed.

### BS-040.6: Template Set In-Use Definition
**Rule**: A template set is "in use" if any department references it OR it is the configured system default template set.
**Rationale**: Both conditions mean the set is actively driving live messages and must not be removed or disabled.

### BS-040.7: Fixed Template Type Catalog
**Rule**: The set of message types a template set may contain is a fixed catalog of code names, each with a fixed display name and description. The literal catalog table is owned canonically by **FS-091.7**; this rule owns the *behavior* — the resolver (FS-040.10) looks messages up by these exact code names, and `staff.pwreset` is resolved packaged-default-only (KL-040.4/KL-040.8).
**Rationale**: Each code name corresponds to a specific system event (new ticket, reply, assignment, etc.); the resolver looks messages up by these exact code names.

### BS-040.8: Template Set Name Uniqueness
**Rule**: Template set names must be unique.
**Rationale**: The set is selectable by name (e.g., for cloning and as a department's template); duplicate names are ambiguous.

### BS-040.9: New Sets Are Cloned From an Existing Set
**Rule**: A new template set is always created by copying every message template from a chosen source set.
**Rationale**: Guarantees a new set is functionally complete from creation rather than missing message types.

### BS-040.10: Only Non-In-Use Sets May Be Disabled
**Rule**: A template set that is in use cannot be disabled (via either bulk disable or the set form).
**Rationale**: Disabling a live set would leave its events without a message source.

### BS-040.11: Default / In-Use Sets May Not Be Deleted
**Rule**: A template set may be deleted only if it is neither in use nor the system default set.
**Rationale**: Same as BS-040.10, escalated — deletion is irreversible.

### BS-040.12: Set Deletion Cascades to Messages and Clears Dept References
**Rule**: Deleting a set removes all of its message-template rows and resets to 0 any department reference to that set.
**Rationale**: Prevents orphaned message rows and dangling department template pointers.

### BS-040.13: Packaged-Default Fallback for Missing Messages
**Rule**: If a set has no stored row for a requested message type, the message is loaded from a packaged per-language structured initial-data file (`<language>/templates/<code-name>.yaml`) that defines a `subject` key and a `body` key; those two keys become the resolved template's subject and body. A file missing either key is rejected as malformed (`InitialDataError`) and yields no fallback.
**Rationale**: Newly created or partially-populated sets still produce sensible messages for every event type, while a structured contract guarantees both parts are present.
**Note**: The fallback file is parsed as a structured (YAML) document with explicit `subject`/`body` keys — it is NOT a free-form file whose first line is the subject and the remainder the body.

### BS-040.14: Tokens Are Dot-Path Traversals
**Rule**: A `%{a.b.c}` token resolves `a` as the root object/variable, then traverses `b`, then `c`, each segment resolving against the prior result.
**Rationale**: Enables expandable variables like `%{ticket.dept.name}` and `%{ticket.staff.name}` without flattening every combination.

### BS-040.15: Object Segment Resolution via Accessor Convention (with Generic Fallback)
**Rule**: Resolving a dot segment against an object first tries that object's accessor named `get` + the capitalized segment; an object result continues traversal, a scalar result terminates it. If no such accessor exists, the resolver falls back to a generic `getVar(<segment>)` accessor on the object when one is provided; a `false` return from that generic accessor terminates with an empty value, an object return continues traversal, and a scalar return terminates with that scalar.
**Rationale**: A uniform reflection convention lets any domain object expose template variables either via dedicated `get<Name>` accessors or a single generic `getVar` dispatcher, without a per-template lookup table.

### BS-040.16: Bare Object Token Uses Self-Representation
**Rule**: A token naming an object with no further path resolves to the object's own string representation when it provides one, else empty.
**Rationale**: Lets `%{ticket}` and `%{assignee}` render a sensible default string.

### BS-040.17: Unknown Tokens Are Preserved, Not Blanked
**Rule**: A token referencing an unknown object or variable resolves to a sentinel "unresolved" result; such tokens are excluded from the replacement map so the literal `%{...}` remains in the output text, and a substitution error is recorded.
**Rationale**: Leaving the literal `%{...}` makes authoring mistakes visible rather than silently dropping content.
**Resolution order & dedup notes**: Each distinct token is resolved at most once per pass — a memoized scalar variable wins first, then a registered object (traversed per BS-040.14/15), then a root-level scalar override; only if all fail is the token treated as unresolved. A token that occurs multiple times in the text is resolved once and every occurrence is replaced with that single value.

### BS-040.18: Substitution Applies Across String or Array Inputs
**Rule**: Substitution operates on a single string or recursively over an array of strings (e.g., the resolved `{subject, body}` pair).
**Rationale**: A single call personalizes both parts of a template at once.

### BS-040.19: Base URL Is Always Available
**Rule**: Every substitution context provides `%{url}` bound to the configured help-desk base URL.
**Rationale**: Nearly every template embeds links back to the ticket portal.

### BS-040.20: Recipient Personalization Is a Post-Pass
**Rule**: After the general substitution pass, alert bodies have `%{recipient}` directly replaced with each recipient's first name (or "Admin" for the admin recipient).
**Rationale**: One alert body is fanned out to multiple staff recipients, each personalized at delivery without re-running full substitution.

### BS-040.21: Sender / Transport Selection Precedence
**Rule**: Choose the per-account SMTP transport if the originating account has SMTP enabled; else the global default SMTP account if configured and enabled (which also becomes the identity when spoofing is disallowed or no account was given); else fall back to the system default email account as the identity.
**Rationale**: Honors per-mailbox SMTP while allowing a single global relay, and guarantees a valid sending identity even when none is supplied.
**"SMTP enabled" definition (BS-040.27)**: An account counts as SMTP-enabled only when its SMTP-active flag is set AND (authentication is NOT required OR a non-empty stored password is present). An account with SMTP active and authentication required but no stored password is therefore treated as NOT SMTP-enabled and is skipped in this precedence — selection falls through to the next tier.

### BS-040.22: Message-Class Headers Suppress Mail Loops
**Rule**: Auto-reply and notice/alert sends add precedence and auto-response-suppression headers (and auto-replies add `X-Autoreply`/`Auto-Submitted`).
**Rationale**: Marks machine-generated mail so remote autoresponders do not bounce replies back and create loops.

### BS-040.23: SMTP-First with Native Fallback
**Rule**: When SMTP is configured, send via SMTP and, on failure, log the error (without emailing) and retry via the native transport; otherwise send natively.
**Rationale**: Maximizes deliverability while never blocking on a misconfigured SMTP relay, and avoids alert loops when the mail subsystem itself is failing.

### BS-040.24: Blank Password on Edit Preserves the Stored Secret
**Rule**: Saving an account edit with an empty password field keeps the previously stored (encrypted) password; only a non-empty password re-encrypts and replaces it.
**Rationale**: Lets admins edit routing/SMTP settings without re-entering mailbox credentials, and avoids exposing the stored secret in the form.
**Mechanism note**: On edit the update path first decrypts the existing stored password into a working "current password" value; the effective password used for live connection checks and for any re-encryption is "the newly submitted password if non-empty, else the decrypted current password." A new encrypted value is only written to storage when a non-empty password was submitted.

### BS-040.25: Hidden Record-Id Integrity Guard
**Rule**: On edit, the submitted hidden record id must equal the id of the account being saved; a mismatch is rejected with a generic internal-error message before any other validation outcome is committed.
**Rationale**: Guards against tampered or stale edit forms that would otherwise write one account's data over another.

### BS-040.26: Save Normalizes Operational Fields and Defaults
**Rule**: Every successful account save resets fetch operational state (`mail_errors=0`, `mail_lastfetch=NULL`), coerces blank numeric fetch/SMTP fields to 0, defaults a blank protocol to `POP`, persists checkbox flags (`noautoresp`, `smtp_spoofing`) as 1/0 by presence, and maps the post-fetch choice to the `mail_delete` flag + archive-folder column (delete ⇒ 1/clear, archive+folder ⇒ 0/folder, otherwise ⇒ 0/clear).
**Rationale**: Keeps the stored row internally consistent and gives the fetch pipeline (FS-041) a clean, fully-defaulted record after every configuration change.

### BS-040.27: SMTP-Enabled Requires a Usable Credential
**Rule**: An account is "SMTP-enabled" for outbound transport selection only if its SMTP-active flag is set and either authentication is not required or a non-empty stored password exists.
**Rationale**: Prevents the mailer from selecting an SMTP transport that would immediately fail authentication for lack of a password, letting selection fall through to the next viable tier instead.

### BS-040.28: Outbound Line-Ending Resolution
**Rule**: The MIME line ending is the `MAIL_EOL` config constant when defined as a string; else a forced `\n` when a Suhosin-style hardening patch is present and the send is non-SMTP; else the MIME library default.
**Rationale**: Works around line-ending corruption introduced by certain PHP hardening patches on the native (non-SMTP) mail path while still honoring an explicit operator override.

### BS-040.29: Per-Request SMTP Connection Reuse with Persistent Sessions
**Rule**: SMTP transports are created with persistence requested and cached within the request, keyed by host + port + username; subsequent sends reusing the same key reuse the cached connection. A send failure evicts that cache entry so the next send reconnects.
**Rationale**: Avoids re-establishing an SMTP session for every message in a batch while ensuring a failed connection is not reused.

---

## Data Requirements

> Canonical column/enum definitions for `ost_email`, `ost_email_template_group`, and `ost_email_template` are owned by **FS-091**. The fields below are listed for behavioral reference.

### Email Account (per account)
- **Identity**: email address (unique), display name, internal notes.
- **Routing defaults**: new-ticket priority id, new-ticket department id, auto-response-disabled flag.
- **Credentials**: username, encrypted password (encrypted with a server secret keyed by the username).
- **Mailbox fetch**: active flag, host, protocol (`POP` | `IMAP`, default `POP`), encryption (`NONE` | `SSL`), port, fetch frequency (minutes, default 5), max emails per fetch (default 30), archive-folder, delete-after-fetch flag, plus operational fields (error count, last error, last fetch) maintained by the fetch pipeline (FS-041).
- **SMTP**: active flag, host, port, auth-required flag, header-spoofing-allowed flag.
- **Timestamps**: created, updated.

### Template Set (`EmailTemplateGroup`, per set)
- Name (unique), active flag, notes, created/updated timestamps, and a derived department-usage count.

### Message Template (`EmailTemplate`, per message in a set)
- Owning set id, **code name** (the message-type key, unique within a set), subject, body, created/updated timestamps.

### Template Type Catalog (fixed code names → name / description)

> The literal 13-row catalog (each `code name → display name → description`, including the seeded 12-message default group) is a static value-list owned canonically by **FS-091.7 (Reference data — email template type catalog & seed)**. This spec does **not** re-tabulate it; it references the code names as behavioral keys for the resolver (FS-040.6/FS-040.10) and substitution paths.
>
> Two behavioral facts about the catalog that this spec owns (because they govern resolution, not the value-list itself):
> - **`staff.pwreset`** is packaged-default-only — it is not part of the install-seeded message rows and is resolved exclusively via the packaged-default fallback (KL-040.4).
> - The `staff.pwreset` catalog entry carries a `default` pointer (`templates/staff.pwreset.txt`) that is **informational only**; the resolver always loads `<language>/templates/staff.pwreset.yaml` like every other fallback and never consults the `.txt` pointer (KL-040.8).

### Variable Catalog (the `%{token}` reference exposed to admins)

**Base ticket variables** (`%{ticket.<x>}`):
`id` (internal ID), `number` (external ticket number), `email`, `name` (full name), `subject`, `phone`, `status`, `priority`, `assigned` (assigned staff and/or team), `create_date`, `due_date`, `close_date`, `auth_token` (auto-login token), `client_link` (client ticket-view URL), `staff_link` (staff ticket-view URL).

**Expandable ticket objects** (traverse further with a dot): `%{ticket.topic}` (help topic), `%{ticket.dept}` (department, e.g. `%{ticket.dept.name}`), `%{ticket.staff}` (assigned/closing staff, e.g. `%{ticket.staff.name}`), `%{ticket.team}` (assigned/closing team).

**Context variables** (depend on the sending context): `%{message}` (incoming message), `%{response}` (outgoing response), `%{comments}` (assign/transfer comments), `%{note}` / `%{note.title}` / `%{note.message}` (internal note — expandable), `%{assignee}` (assigned staff/team), `%{assigner}` (staff assigning), `%{recipient}` (personalized per recipient — BS-040.20), `%{signature}`, `%{url}` (base FQDN — always present), `%{reset_link}` (password-reset link, password-reset context only), `%{staff.name}`.

### Default Template Set (seeded)
- A single set named **"osTicket Default Template"** is seeded active at install, populated with one message per seeded catalog code name (each shipping default subject/body using the `%{...}` tokens above). The seed group + its 12 code-name member list is owned by **FS-091.7**; this spec owns only the runtime consequence — the configuration store designates this seeded set as the **default template set** (config-owned by FS-032).

### Referenced configuration keys (owned by FS-032)
`default_email_id` (system default sending account), `alert_email_id` (alert sender), `default_smtp_id` (global default SMTP account), `default_template_id` (default template set), the admin email, default priority id, default department id. This spec reads these to gate deletion, fallback selection, and uniqueness checks but does not own their editing.

---

## User Flows / Interactions

### Flow 1: Add a Fetch-Enabled Mailbox
1. Admin opens Email Addresses → "Add New Email".
2. Enters address, name, picks new-ticket priority/department.
3. Enables Mail Account, enters host/port/protocol/encryption, fetch frequency, emails-per-fetch, username/password, and a post-fetch action (archive folder / delete / nothing).
4. Submits. The system validates fields, then live-connects to the mailbox (and verifies the archive folder if named).
5. On success the account is stored and listed; on failure the connection error is shown inline and the form retains entries.

### Flow 2: Enable Outbound SMTP on an Account
1. Admin edits the account, sets SMTP status to Enable, enters SMTP host/port and whether auth is required.
2. Leaves the password blank to keep the existing credential (or enters a new one).
3. Submits; the system opens a live SMTP session to validate, then disconnects and saves.

### Flow 3: Delete Email Accounts
1. Admin selects accounts in the list (the default account's checkbox is disabled) and clicks Delete.
2. The system checks for department associations; if any selected account is bound to a department it refuses the whole batch.
3. Otherwise it deletes the selected accounts, reassigning any dependent departments to the default email and clearing their autoresponse-email references; a success/partial message is shown.

### Flow 4: Create and Edit a Template Set
1. Admin opens Email Templates → "Add New Template".
2. Names the set, chooses a source set to clone, sets status, saves.
3. The new set is created with all messages copied; admin opens it and edits individual message subjects/bodies, using the Ticket Variables reference to insert `%{...}` tokens.
4. Implements any unimplemented message types as needed.

### Flow 5: Activate / Deactivate / Delete Template Sets
1. From the list, admin selects sets and chooses Enable, Disable, or Delete.
2. Disable/Delete skip any in-use or default set, reporting which were processed.

### Flow 6: A System Message Is Sent
1. A ticket event occurs (e.g., new ticket). The system resolves the relevant message type from the active template set (or packaged default).
2. `%{...}` tokens in subject and body are substituted from the ticket and context; `%{recipient}` is personalized per staff recipient.
3. The mailer selects the sending account/transport (per-account SMTP → global SMTP → default email), composes the MIME message with anti-loop headers, and sends via SMTP (falling back to native mail on failure).

---

## Edge Cases & Error Scenarios

### EC-040.1: IMAP Runtime Support Missing
**Scenario**: An admin enables mailbox fetching but the runtime lacks IMAP support.
**Expected**: Save fails with an error stating IMAP support must be enabled; the account is not persisted as fetch-enabled.

### EC-040.2: Live Mailbox Login Fails on Save
**Scenario**: Fetch credentials/host are wrong.
**Expected**: The connection attempt fails; the form shows a protocol-settings error with the underlying connection error and the save aborts (no broken account is stored).

### EC-040.3: Archive Folder Does Not Exist
**Scenario**: Admin chooses "archive" and names a non-existent mailbox folder.
**Expected**: The folder check fails and the post-fetch field shows an "Invalid or unknown mail folder" error; save aborts.

### EC-040.4: SMTP Login Fails on Save
**Scenario**: SMTP host/port/credentials are wrong.
**Expected**: The live SMTP check fails with "Unable to log in. Check SMTP settings." plus the underlying message; save aborts.

### EC-040.5: Deleting an Account Bound to a Department
**Scenario**: A selected account is a department's inbound or autoresponse email.
**Expected**: The whole delete batch is refused with a "remove association first" message; nothing is deleted.

### EC-040.6: Attempt to Delete or Disable the Default Template Set
**Scenario**: Admin selects the default set for disable/delete.
**Expected**: The default set is skipped (its checkbox is disabled and the action loop excludes it); the message notes in-use/default sets cannot be processed.

### EC-040.7: Disabling an In-Use Template Set via the Set Form
**Scenario**: Admin opens an in-use set and switches Status to Disabled.
**Expected**: The update is rejected with "Template in use cannot be disabled!".

### EC-040.8: Template References an Unknown Variable
**Scenario**: A template body contains `%{ticket.bogus}`.
**Expected**: The unknown token is left literally in the sent message and a substitution error is recorded; the rest of the message renders normally.

### EC-040.9: SMTP Send Fails at Delivery Time
**Scenario**: A configured SMTP relay is down when a message is actually sent.
**Expected**: The error is logged (without emailing an alert), the cached SMTP connection is dropped, and the message is retried via the native mail transport.

### EC-040.10: No Originating Account Supplied
**Scenario**: A send is requested with no specific account.
**Expected**: The mailer resolves the global default SMTP account or the system default email as the sending identity (per BS-040.21); the From header reflects that account.

### EC-040.11: Blank Password on Edit
**Scenario**: Admin edits an account and leaves password empty.
**Expected**: The stored encrypted password is preserved (BS-040.24); only a non-empty entry replaces it.

### EC-040.12: Malformed Packaged-Default Template File
**Scenario**: A set lacks a stored row for a requested message type, and the packaged-default `<language>/templates/<code-name>.yaml` file exists but does not define both a `subject` and a `body` key.
**Expected**: The loader raises an `InitialDataError` and returns no fallback template; combined with the missing stored row, the template fetch ultimately fails (logged warning) and no message is produced (BS-040.13, FS-040.10).

### EC-040.13: SMTP Account Configured but Missing Its Password
**Scenario**: An account has SMTP active with authentication required, but no stored password (e.g., created before a password was set, or password cleared).
**Expected**: The account is treated as NOT SMTP-enabled; the mailer skips it during transport selection and falls through to the next tier (global default SMTP, then default-email native send) (BS-040.27).

### EC-040.14: Attachment Source Unavailable at Send Time
**Scenario**: An outbound message references a stored file id that no longer resolves, or a file path that is missing or unreadable.
**Expected**: That attachment is silently skipped; the message is still composed and sent without it (FS-040.12).

---

## Dependencies

| Dependency | Specification | Relationship |
|------------|---------------|--------------|
| Inbound fetch execution (POP3/IMAP polling, mail-to-ticket parsing) | FS-041 | This spec owns fetch *settings* + the one-shot connect check at save; FS-041 owns the periodic fetch run that consumes them. |
| Ticket filters & banlist (autoreply/autoresponder gating, reply-to override, ban) | FS-042 | Filters select the `ticket.autoreply` template and toggle autoresponders that this spec's templates feed. |
| System settings / email & template defaults | FS-032 | Owns the config keys (`default_email_id`, `alert_email_id`, `default_smtp_id`, `default_template_id`, admin email, default priority/dept) that this spec reads. |
| Departments, priorities | FS-030 / FS-091 | Accounts bind a new-ticket department and priority; departments reference accounts and template sets. |
| Staff accounts & password reset | FS-031 / FS-002 | `staff.pwreset` template + `%{reset_link}`; staff-email uniqueness check on account create. |
| Ticket model & events (the substitution objects + send triggers) | FS-021 / FS-011 / FS-041 | Provides the `ticket` object and event hooks that resolve `%{ticket.*}` and trigger sends. |
| Canned responses (also use `%{...}` substitution) | FS-022 | Reuse the same VariableReplacer grammar documented here. |
| Cryptography (password encrypt/decrypt) | FS-003 | Account credentials are encrypted with a server secret keyed by username. |
| Reference data / schema / enums | FS-091 | Canonical `ost_email`, `ost_email_template_group`, `ost_email_template` DDL and the `POP`/`IMAP`, `NONE`/`SSL` enums; **FS-091.7** owns the email template type catalog (13 code names → name → description) + the seeded default group's 12-message list. This spec owns the resolver behavior over those keys. |
| Shared listing/pagination & CSRF | FS-090 / FS-001 | List pagination, sort, and POST CSRF protection. |

---

## Known Limitations

### KL-040.1: Plain-Text Only Outbound (HTML Stripped)
**Limitation**: Outbound messages are sent as plain text; HTML entities in the body are decoded and only a text MIME part is produced. The code notes "html support coming."
**Impact**: Rich formatting in templates is not rendered as HTML email; tokens and content must read sensibly as plain text.

### KL-040.2: Mailer From-Address Fallback Path Has a Self-Reference Bug
**Limitation**: In the constructor branch that falls back to the system default email when no account is supplied, the SMTP-info probe is read from the (null) supplied account rather than the resolved default account, so SMTP info is not actually picked up on that specific branch.
**Impact**: When relying purely on the default-email fallback (no account, no global SMTP), the message may be sent via native mail even if the default email had SMTP configured. Cosmetically/operationally minor because the global-SMTP branch is checked first.

### KL-040.3: Template Language Is Fixed to English (US)
**Limitation**: The set form exposes a Language selector, but it offers only English (US) and the set's language is hard-coded to `en_US`; packaged defaults are loaded only from the `en_US` directory.
**Impact**: No real multi-language template support in this version despite the UI affordance.

### KL-040.4: `staff.pwreset` Has No In-Set Editing UI
**Limitation**: The `staff.pwreset` message type is defined in the catalog and ships as a packaged-default file, but it is not part of the install-seeded message rows and is resolved via the packaged-default fallback; it is not surfaced in the per-set message list the same way as the others.
**Impact**: The staff password-reset email content is effectively edited only by changing the packaged default rather than per-set in the UI.

### KL-040.5: Staff View Partials Present and Verified Against Source
**Note**: The FS-040 template- and email-management view partials are **present** in this 1.7 source snapshot and have been read in full: `include/staff/templates.inc.php` (set listing — `@implements FS-040.5/.9`), `include/staff/template.inc.php` (set create/clone + manage view — `@implements FS-040.8/.6`), and `include/staff/tpl.inc.php` (per-message editor — `@implements FS-040.7/.11`), alongside `emails.inc.php`/`email.inc.php` and the shared `header.inc.php`/`footer.inc.php`. The entry scripts (`emails.php`, `templates.php`) and the domain classes (`Email`, `EmailTemplateGroup`, `EmailTemplate`, `Mailer`, `VariableReplacer`) are likewise intact.
**Impact**: The FS-040 UI-presentation acceptance criteria whose owning views are present — the sortable columns and default sort, the "showing N / No emails found" captions, the disabled default-account / system-default checkbox, the password-blank hint, the post-fetch radio reconstruction on edit, the Language selector, the "Supported Variables" reference link, the In-Use / "(System Default)" labels — are **verified against view source**, not inferred. (An earlier revision of this KL wrongly asserted the entire `include/staff/` partial tree was absent and marked these criteria unverifiable; that claim was false for this snapshot and has been corrected. One genuine rendering quirk surfaced by reading the views is recorded as KL-040.11.)

### KL-040.6: No Standalone Email Diagnostic / Test Tool in This Snapshot
**Limitation**: No `scp/emailtest.php` (a one-off send-test / diagnostic page) exists in this 1.7 snapshot. The only connectivity validation is the live mailbox/SMTP check performed at account save time (FS-040.3).
**Impact**: Administrators cannot send an ad-hoc test message from a dedicated tool; they validate connectivity implicitly by saving the account.

### KL-040.7: Substitution Errors Are Recorded but Not Surfaced
**Limitation**: Unknown-token substitution errors are collected by the replacer, but the calling send paths do not surface them to the administrator; the literal `%{...}` simply appears in the delivered message.
**Impact**: Authoring mistakes are only discovered by inspecting sent mail.

### KL-040.8: `staff.pwreset` Catalog `default` Pointer Is Inert
**Limitation**: The `staff.pwreset` catalog entry carries an extra `default` field pointing to `templates/staff.pwreset.txt`, but the packaged-default resolver ignores that pointer entirely and always loads `<language>/templates/<code-name>.yaml`. The `.txt` reference is a leftover that the resolution path does not consult.
**Impact**: Editing or relocating the referenced `.txt` file has no effect; the staff password-reset fallback is governed solely by the `.yaml` initial-data file like every other message type.

### KL-040.9: Per-Message "Code-Name Required" Validation Silently Misfires
**Limitation**: In the message-template save path, the implement (create) branch that should flag a missing code-name writes the error into a mistyped variable (`$errprs` instead of `$errors`), so the "Code name required" error is never actually registered. A create with a present template-group id but a missing code-name therefore does not raise that specific validation error.
**Impact**: A malformed implement request missing the code-name is not blocked by that check (in practice the controller always supplies the code-name from the unimplemented-message link, so the path is rarely exercised), but the intended guard is non-functional.

### KL-040.10: Template-Group In-Memory Cache Never Hits (Mistyped Property)
**Limitation**: The template-group method that lists its messages guards its in-memory cache on a mistyped property name (`$this->_tempates`) while populating the correctly-spelled one (`$this->_templates`), so the cache check always fails and the message list is re-queried from storage on every call within the request.
**Impact**: Purely a minor per-request efficiency loss (repeated identical queries when the set's message list is read multiple times); no functional or correctness difference.

### KL-040.11: Template-Set "In-Use" Column Header Is a Dead Sort Link
**Limitation**: In the template-set listing (`include/staff/templates.inc.php`), the **In-Use** column header emits a `sort=inuse` link, but `inuse` is not among the recognized sort keys (the sort map is `name`/`status`/`created`/`updated` only) and `$inuse_sort` is never assigned, so the click silently reverts to the default `name` sort. In-Use is a derived display-only value (BS-040.6) and is not a sortable column despite the header rendering as a link.
**Impact**: Clicking the In-Use header appears interactive but does nothing useful — it re-renders the list in the default Name order rather than grouping by usage. Mirrors the analogous team "Last Updated" dead-sort-link quirk (KL-030-12).

---

## Future Considerations
- HTML (multipart) outbound email with a rich template editor (KL-040.1).
- Real multi-language template sets keyed off recipient/locale (KL-040.3).
- A dedicated email diagnostic/test-send tool surfacing both connectivity and token-resolution errors (KL-040.6, KL-040.7).
- Per-message editing UI for `staff.pwreset` and other system-only templates (KL-040.4).
- Fix the default-email SMTP fallback self-reference (KL-040.2).
