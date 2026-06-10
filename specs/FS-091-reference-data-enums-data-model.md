# FS-091: Reference Data, Enums & Data Model

## Overview

This specification is the **canonical home** for the osTicket 1.7 logical data model and for every enumeration, reference set, configuration default, and seeded reference row used across the help desk. All other functional specs (FS-001 … FS-090) cross-reference the definitions here rather than restating them. When a term such as "ticket status", "ticket source", "priority urgency", "log level", "filter match field", or "default SLA grace period" appears in another spec, the authoritative literal values live in this document.

The application persists all state in a relational store accessed through procedural data-access helpers. Tables share a configurable name prefix (the installer placeholder is `%TABLE_PREFIX%`; the conventional default is `ost_`). The schema comprises **34 tables** created and seeded by the installer's `install-mysql.sql` stream (verified by counting the `CREATE TABLE` statements; see FS-091.15). This spec documents:

- The **entity-relationship model** — every table described as a logical entity (purpose, identity, key fields, relationships) rather than as SQL DDL.
- All **enumerations** with their exact literal values and human-readable labels.
- All **seeded reference rows** installed at first run (priorities with colors, default SLA, the default email-template set, default departments/help-topics/groups/team/pages/timezones).
- All **configuration keys** with their installed defaults and value ranges.
- The **identifier and numbering schemes** (notably the external ticket-number generator: random vs. sequential), and the timestamp/audit conventions.

> **Technology note**: This document is technology-agnostic. Field names, table names, enum literals, and default values are extracted because they are behavioral facts of the system, but no storage engine, language, or framework is prescribed. Where SQL column types appear in the source, they are translated into logical descriptions (identity, flag, code, timestamp, free text, etc.).

---

## Functional Requirements

### FS-091.1: Configurable Table Prefix

**Description**: The system shall store all persistent data in a set of tables sharing a single configurable name prefix, so that multiple installations can coexist in one database.

**Acceptance Criteria**:
- Every table name is formed by concatenating a prefix placeholder (`%TABLE_PREFIX%` in the install stream) with a fixed logical table name (e.g. `ticket`, `staff`, `config`).
- The prefix is chosen at install time and is conventionally `ost_`.
- All cross-table references (foreign-key-style integer columns) resolve within the same prefixed table set.
- The prefix is referenced application-wide via a single constant (`TABLE_PREFIX`); no table is addressed without it.

### FS-091.2: External Ticket Number Generation (Random vs. Sequential)

**Description**: The system shall assign each ticket a public-facing external number (`ticketID`) distinct from its internal record identifier (`ticket_id`), generated either randomly or sequentially according to a configuration flag.

**Acceptance Criteria**:
- Each ticket has two identifiers: an internal auto-incrementing record key (`ticket_id`) and a public external number (`ticketID`).
- A configuration flag `random_ticket_ids` (installed default `1` = enabled) selects the generation mode:
  - **Random mode** (`random_ticket_ids` = 1): the external number is a random numeric value of fixed length `EXT_TICKET_ID_LEN` = **6 digits**, produced by drawing a random integer in the inclusive range `100000`–`999999` (the length-padded `1…0` to `9…9` bounds for a 6-digit number).
  - **Sequential mode** (`random_ticket_ids` = 0): the external number is set equal to the internal record key (`ticketID = ticket_id`), assigned immediately after the row is created. A source comment notes sequential IDs are discouraged.
- In random mode the generator guarantees uniqueness of `ticketID` by re-rolling on collision: it queries for an existing ticket with the candidate number and recurses until an unused value is found.
- Even though `ticketID` is generated unique, the uniqueness constraint that the database enforces is the **composite** of `(ticketID, email)` — see BS-091.1. The application chooses to make `ticketID` globally unique for clarity.
- The external number is the value shown to clients and used for ticket lookup (FS-010), email subjects, and the `%{ticket.number}` template token.

### FS-091.3: Ticket Status, Source & Answered/Overdue Flags

**Description**: The system shall classify every ticket by an open/closed status, a creation source, and two independent boolean condition flags (answered, overdue).

**Acceptance Criteria**:
- A ticket's `status` is one of exactly two values: `open`, `closed` (installed default `open`). Convenience predicates `isOpen()` / `isClosed()` read this field.
- A ticket's `source` is one of exactly five values: `Web`, `Email`, `Phone`, `API`, `Other` (installed default `Other`).
- A ticket independently carries `isoverdue` (boolean flag, default off) and `isanswered` (boolean flag, default off); these are orthogonal to `status` (a ticket may be open-and-answered, open-and-overdue, etc.).
- Internal creation origins map to filter targets/sources via `origin2target`: `web`→`Web`, `email`→`Email`, `phone`→`Web`, `staff`→`Web`, `api`→`API` (see FS-091.9). Note that phone and staff origins normalize to the `Web` filter target.

### FS-091.4: Thread Entry & Attachment Reference Types

**Description**: The system shall record every ticket conversation entry and every attachment association under one of three entry types: Message, Response, or Note.

**Acceptance Criteria**:
- A thread entry's type (`thread_type`) is one of exactly three single-character codes: `M` (Message, from the client/requester), `R` (Response, from staff to the client), `N` (Note, internal staff-only).
- The same three-value code set (`M`, `R`, `N`) is reused as the attachment reference type (`ref_type`, default `M`), tying an attachment to the kind of thread entry it belongs to.
- The three thread-entry kinds are modeled as distinct logical sub-types (Message / Response / Note) over a shared thread-entry parent, but share one storage table distinguished by `thread_type`.

### FS-091.5: Priority Reference Set & Urgency Ordering

**Description**: The system shall maintain a priority catalog where each priority carries a machine tag, a display label, a display color, a numeric urgency rank, and a public-visibility flag.

**Acceptance Criteria**:
- Each priority record has: machine `priority` tag (unique), `priority_desc` display label, `priority_color` (hex color string), `priority_urgency` (numeric rank), and `ispublic` flag.
- **Lower `priority_urgency` number = more urgent** (urgency 1 is the most urgent).
- The catalog is seeded with four priorities (see Data Requirements → Seeded Priorities).
- A priority is offered to clients only when `ispublic` = 1; staff-only priorities (`ispublic` = 0) are filtered out of public pickers via `getPublicPriorities()`.
- The installed default priority for new tickets is `default_priority_id` = `2` (the `normal` priority).

### FS-091.6: SLA Plan Reference & Default Grace Period

**Description**: The system shall maintain service-level-agreement plans, each defining a grace period (in hours) and escalation/alert behavior, with one default plan installed.

**Acceptance Criteria**:
- Each SLA plan has: `name` (unique), `grace_period` (whole hours), `isactive` flag, `enable_priority_escalation` flag, and `disable_overdue_alerts` flag.
- The `grace_period` defines how long after creation (or last response) a ticket may remain before it is flagged overdue.
- Exactly one SLA plan is seeded: **"Default SLA"** with `grace_period` = **48** hours, active, priority-escalation enabled, overdue alerts NOT disabled (see Data Requirements → Seeded SLA).
- A help topic, department, or ticket may each reference an SLA; the effective SLA is resolved from these in precedence order (explicit trump > department > topic > system default), defined canonically in **FS-021.13**, not here.

### FS-091.7: Email Template Group & Code-Name Set

**Description**: The system shall organize outbound email templates into named groups, where each group provides a fixed set of templates addressed by stable code-names.

**Acceptance Criteria**:
- A template group has a unique `name`, an `isactive` flag, and contains one template row per code-name.
- The canonical template code-name set (with descriptions) is fixed in the application as `all_names`:

  | Code-name | Display name | Purpose |
  |-----------|--------------|---------|
  | `ticket.autoresp` | New Ticket Auto-response | Autoresponse to user on new ticket |
  | `ticket.autoreply` | New Ticket Auto-reply | Canned auto-reply on new ticket (filter-driven; overrides normal auto-response) |
  | `message.autoresp` | New Message Auto-response | Confirmation when user appends a message |
  | `ticket.notice` | New Ticket Notice | Notice when staff opens a ticket on a user's behalf |
  | `ticket.overlimit` | Over Limit Notice | One-time notice when user hits max open tickets |
  | `ticket.reply` | Response/Reply Template | Used on staff response/reply to user |
  | `ticket.alert` | New Ticket Alert | Staff alert on new ticket |
  | `message.alert` | New Message Alert | Staff alert when user replies |
  | `note.alert` | Internal Note Alert | Staff alert on new internal note |
  | `assigned.alert` | Ticket Assignment Alert | Staff alert on assignment |
  | `transfer.alert` | Ticket Transfer Alert | Staff alert on transfer |
  | `ticket.overdue` | Overdue Ticket Alert | Staff alert on stale/overdue ticket |
  | `staff.pwreset` | Staff Password Reset | Password-reset link to staff (loaded from initial-data file, not seeded into the group) |

- One template group is seeded — **"osTicket Default Template"**, active — populated with twelve of the thirteen templates (all except `staff.pwreset`, which is loaded on demand from initial data). See Data Requirements → Seeded Email Templates.
- The installed default template group is `default_template_id` = `1`.
- A template body uses `%{variable}` token substitution; the token catalog is owned by FS-040.

### FS-091.8: Log Severity Levels

**Description**: The system shall record system log entries under one of three severity levels and shall suppress entries below the configured threshold.

**Acceptance Criteria**:
- A log entry's `log_type` is one of exactly three values: `Debug`, `Warning`, `Error`.
- A numeric `log_level` configuration controls the minimum severity persisted; the installed default `log_level` = `2`.
- A `log_graceperiod` configuration (installed default `12`, in months) governs automatic purging of aged log rows.
- Each log entry also records a free-text `title`, the `log` body, the originating `logger` name, the client `ip_address`, and creation/update timestamps.

### FS-091.9: Inbound Filter Match Fields & Operators

**Description**: The system shall let inbound mail/ticket filters test incoming requests against a fixed set of match fields using a fixed set of comparison operators.

**Acceptance Criteria**:
- The filter-rule **match field** (`what`) is one of exactly five storage values: `name`, `email`, `subject`, `body`, `header`. The administrator-facing picker (`getSupportedMatches`) exposes four of them: `name`→"Name", `email`→"Email", `subject`→"Subject", `body`→"Body/Text".
- The filter-rule **operator** (`how`) is one of exactly six storage values: `equal`, `not_equal`, `contains`, `dn_contain`, `starts`, `ends`. The administrator-facing picker (`getSupportedMatchTypes`) labels them: `equal`→"Equal", `not_equal`→"Not Equal", `contains`→"Contains", `dn_contain`→"Does Not Contain", `starts`→"Starts With", `ends`→"Ends With".
- A filter's **target** (`target`) — the request origin it applies to — is one of exactly four values: `Any`, `Web`, `Email`, `API` (default `Any`).
- A composite uniqueness constraint forbids two identical rules on the same filter: the tuple `(filter_id, what, how, val)` must be unique (see BS-091.2).

### FS-091.10: Page Type Set & Default Pages

**Description**: The system shall maintain editable site pages, each typed as landing, offline, thank-you, or other, with one default page of each functional type installed.

**Acceptance Criteria**:
- A page's `type` is one of exactly four values: `landing`, `offline`, `thank-you`, `other` (default `other`).
- Three pages are seeded and active: an **Offline** page (type `offline`), a **Thank you** page (type `thank-you`), and a **Landing** page (type `landing`).
- After seeding, three configuration keys are set to the ids of these pages: `landing_page_id`, `offline_page_id`, `thank-you_page_id`.

### FS-091.11: Staff Signature & Paper-Size Enumerations

**Description**: The system shall let each staff member choose a default signature mode and a default print paper size from fixed sets.

**Acceptance Criteria**:
- A staff member's `default_signature_type` is one of exactly three values: `none`, `mine`, `dept` (default `none`).
- A staff member's `default_paper_size` is one of exactly four values: `Letter`, `Legal`, `A4`, `A3` (default `Letter`). (Corrected in Phase 4 — `Ledger` had no source basis: `include/staff/ticket-view.inc.php:803` defines `array('Letter', 'Legal', 'A4', 'A3')` and the bundled FPDF PageFormats are `a3, a4, a5, letter, legal`.)

### FS-091.12: Email Protocol & Encryption Enumerations

**Description**: The system shall configure each mailbox account with an inbound mail protocol and an encryption mode from fixed sets.

**Acceptance Criteria**:
- A mail account's `mail_protocol` is one of exactly two values: `POP`, `IMAP` (default `POP`).
- A mail account's `mail_encryption` is one of exactly two values: `NONE`, `SSL`.
- Default fetch cadence and caps: `mail_fetchfreq` = 5 (minutes), `mail_fetchmax` = 30 (messages per fetch); SMTP defaults: `smtp_secure` = 1, `smtp_auth` = 1.

### FS-091.13: Ticket Event State Set

**Description**: The system shall append an immutable event row to a ticket's history each time a tracked lifecycle action occurs, tagged with a fixed event-state code.

**Acceptance Criteria**:
- A ticket event's `state` is one of exactly six values: `created`, `closed`, `reopened`, `assigned`, `transferred`, `overdue`.
- Each event records the acting `staff` name (default literal `SYSTEM` when system-generated), the contextual `staff_id` / `team_id` / `dept_id` / `topic_id` snapshot, an `annulled` flag (default off, used to void superseded events), and a `timestamp`.
- Event rows are append-only history (no primary key; indexed by `(ticket_id, state, timestamp)` and by `(timestamp, state)` for reporting).

### FS-091.14: File Type Code & Content-Addressed Storage

**Description**: The system shall store uploaded file bytes in content-addressed, chunked form, tagging each file with a single-character category code.

**Acceptance Criteria**:
- A file record carries a `ft` category code (single character, default `T`), the MIME `type`, the `size`, a content `hash`, the original `name`, and a creation timestamp.
- File bytes are stored as ordered chunks in a companion chunk entity keyed by `(file_id, chunk_id)`; the chunk size is fixed at `CHUNK_SIZE` = `500 * 1024` bytes (512,000 bytes).
- One seed file is installed (a 25-byte `osTicket.txt`) and linked to the seeded sample canned response as a demonstration attachment.

### FS-091.15: Installed Table Inventory (Exactly 34 Tables)

**Description**: The installer's schema stream shall create a fixed set of exactly thirty-four logical tables; this is the authoritative table inventory.

**Acceptance Criteria**:
- The installer stream (`install-mysql.sql`) contains exactly **34** table-creation statements; the schema therefore comprises **34 tables**, not 35.
- The complete inventory (logical names, prefix omitted): `api_key`, `faq`, `faq_attachment`, `faq_category`, `faq_topic`, `sla`, `config`, `department`, `email`, `filter`, `filter_rule`, `email_template_group`, `email_template`, `file`, `file_chunk`, `groups`, `group_dept_access`, `help_topic`, `canned_response`, `canned_attachment`, `session`, `staff`, `syslog`, `team`, `team_member`, `ticket`, `ticket_attachment`, `ticket_lock`, `ticket_email_info`, `ticket_event`, `ticket_priority`, `ticket_thread`, `timezone`, `page`.
- These correspond one-to-one to entities #1–#34 in the logical model below.
- No additional table is installed by the core stream; later database growth (e.g. via upgrade/migration streams or plugins) is out of scope for the install inventory.

---

## Business Rules

### BS-091.1: Ticket External Number Is Unique Per Email, Not Globally (Schema Constraint)

**Rule**: The database enforces uniqueness on the **composite** `(ticketID, email)`, allowing in principle the same external number for two different requester emails; the application layer nonetheless generates globally-unique external numbers (BS-091.4).

**Rationale**: The composite key supports a (legacy) model in which a number plus the requester email together identify a ticket for client lookup; the application chooses stricter global uniqueness for clarity and to prevent confusion.

**Examples**:
- Two tickets cannot share both the same `ticketID` and the same `email`.
- The random-id generator re-rolls on any existing `ticketID` regardless of email, so collisions never actually occur in practice.

### BS-091.2: Identical Filter Rules Are Forbidden On One Filter

**Rule**: Within a single filter, no two rules may share the same match field, operator, and value: `(filter_id, what, how, val)` is unique.

**Rationale**: Duplicate rules would be redundant and could distort "match all / match any" evaluation counts.

**Examples**:
- A filter already containing `email equal a@b.com` rejects a second identical `email equal a@b.com` rule.
- The same value with a different operator (`email contains a@b.com`) is allowed.

### BS-091.3: Lower Urgency Number Means Higher Priority

**Rule**: A priority's urgency rank orders priorities from most-urgent (rank 1) to least-urgent (higher numbers); escalation and sorting treat smaller `priority_urgency` as more important.

**Rationale**: The numeric rank gives a stable ordering independent of the display label or color.

**Examples**:
- `emergency` (urgency 1) outranks `high` (urgency 2), which outranks `normal` (urgency 3), which outranks `low` (urgency 4).
- The default `normal` priority (id 2, urgency 3) sits in the middle of the seeded set.

### BS-091.4: External Ticket Numbers Are Generated Unique By Re-Roll

**Rule**: In random-id mode, the external ticket number is a 6-digit random number re-generated until it does not collide with any existing ticket's external number.

**Rationale**: Uniqueness keeps client-facing references unambiguous despite random generation.

**Examples**:
- A candidate `482913` already in use causes a fresh draw.
- After installation switches to sequential mode, the external number equals the internal record id and increments monotonically.

### BS-091.5: Names Are Unique Within Their Reference Set

**Rule**: Many reference entities enforce a unique human-readable name within their table: department `dept_name`, SLA `name`, team `name`, group has no name-unique but priority `priority` tag, email-template-group implied by application, canned-response `title`, FAQ `question`, page `name`, help-topic `(topic, topic_pid)`, mailbox `email`, API key `apikey`, staff `username` are each unique.

**Rationale**: Unique names prevent ambiguous references and let the UI address entities by name.

**Examples**:
- A second department named "Support" is rejected.
- A help topic "Billing" may exist twice only under different parents (`topic_pid`), because uniqueness is on the `(topic, topic_pid)` pair.
- A staff `username` collision is rejected at save time.

### BS-091.6: Default Linking IDs Of Zero Mean "Unset / System Default"

**Rule**: Integer reference columns that default to `0` (e.g. `dept_id`, `sla_id`, `priority_id`, `topic_id`, `staff_id`, `team_id`, `tpl_id`, `email_id` when `0`) denote "no specific entity selected — fall back to the system default", not a real row with id 0.

**Rationale**: A sentinel zero lets routing/resolution code apply system-wide defaults (e.g. `default_dept_id`, `default_sla_id`) when a topic/department leaves a field unset.

**Examples**:
- A help topic with `sla_id` = 0 (the seeded "Billing" topic) inherits the system default SLA rather than referencing SLA #0.
- A ticket with `staff_id` = 0 and `team_id` = 0 is unassigned.
- The seeded "Support" department references `sla_id` = 0 at insert and resolves to the default SLA.

### BS-091.7: Installed Configuration Defaults Are Authoritative At First Run

**Rule**: At install time the configuration table is seeded with a fixed set of `(namespace='core', key, value)` rows; these installed values are the system defaults until an administrator changes them.

**Rationale**: A fresh install must be fully operable with sensible defaults before any admin configuration.

**Examples**:
- `max_page_size` = 25 governs default list pagination until overridden.
- `max_file_size` = 1048576 (1 MiB) caps uploads by default.
- `default_priority_id` = 2 selects `normal` for new tickets.

### BS-091.8: Group → Department Access Is Seeded Fully Open

**Rule**: At install the group-to-department access table is populated with the **Cartesian product** of all seeded groups and all seeded departments, granting every seeded group access to every seeded department.

**Rationale**: A fresh install should let all three seeded staff groups work all departments until access is narrowed.

**Examples**:
- Three groups × two departments = six access rows installed.
- Adding a new department later does NOT auto-grant access; access rows are managed per group thereafter.

### BS-091.9: The System Ban List Is A Reserved Filter

**Rule**: A reserved filter named **"SYSTEM BAN LIST"** (highest exec order 99, reject-on-match) is installed and must not be removed; the email banlist mechanism (FS-042) appends its entries as rules on this filter.

**Rationale**: Email banning is implemented as a special always-present rejecting filter rather than a separate subsystem.

**Examples**:
- Banning `spam@x.com` adds an `email equal spam@x.com` rule under the SYSTEM BAN LIST filter.
- The seeded sample rule `email equal test@example.com` demonstrates the rule shape.

### BS-091.10: Default Templates / SLA / Pages Cannot Be Disabled While In Use

**Rule**: A reference entity that is currently the configured system default (e.g. the default email-template group) or is referenced by a department cannot be disabled or deleted until it is no longer in use.

**Rationale**: Disabling an in-use default would leave the system without a required resource (e.g. no template to render alerts).

**Examples**:
- The "osTicket Default Template" group cannot be disabled while it is `default_template_id` or while any department references it.
- Deletion of a template group reassigns referencing departments' `tpl_id` to 0 only when it is not in use.

### BS-091.11: Timestamps Are Recorded On Create And Update

**Rule**: Most reference and transactional entities carry `created` and `updated` timestamps; `created` is set once at insertion and `updated` is refreshed on every modification. The config table's `updated` auto-updates to the current time on change.

**Rationale**: Provides a lightweight audit of when records were last touched, used by sorting, log purging, and "last fetch/last error" tracking.

**Examples**:
- Editing a canned response refreshes its `updated` while preserving `created`.
- A mailbox records `mail_lastfetch` and `mail_lasterror` independently of `created`/`updated`.

### BS-091.12: Ticket Events Are Append-Only And Annullable, Not Deleted

**Rule**: Lifecycle events are appended to ticket history and never updated in place; a superseded event is marked `annulled = 1` rather than removed.

**Rationale**: Preserves a complete, auditable lifecycle trail (used by reporting and the ticket activity view).

**Examples**:
- Re-assigning a ticket annuls the prior `assigned` event and appends a new one.
- A `created` event persists for the life of the ticket.

---

## Data Requirements

### The 34-Table Logical Model

Tables are grouped by domain. Each is described as a logical entity: its identity, key attributes (in logical terms), and its relationships. Sentinel-zero references follow BS-091.6. The installer's `install-mysql.sql` stream contains exactly **34** `CREATE TABLE` statements (entities #1–#34 below); there is no 35th installed table (see FS-091.15 / KL-091.9).

#### Ticket Spine

1. **Ticket** (`ticket`) — the central work item. Identity: internal record key `ticket_id`; public external number `ticketID` (see FS-091.2). Attributes: requester `email`/`name`/`phone`/`phone_ext`, `subject` (default "[no subject]"), originating `ip_address`, `status` (open/closed), `source` (Web/Email/Phone/API/Other), `isoverdue`, `isanswered` flags, `duedate`, `reopened`, `closed`, `lastmessage`, `lastresponse`, `created`, `updated`. References: `dept_id`→Department, `sla_id`→SLA, `priority_id`→Priority, `topic_id`→Help Topic, `staff_id`→Staff and `team_id`→Team (assignment). Uniqueness: composite `(ticketID, email)`.

2. **Ticket Thread Entry** (`ticket_thread`) — one conversation entry. Identity: `id`; self-reference `pid` (parent entry). Attributes: `thread_type` (M/R/N), `poster`, `source`, `title`, `body`, `ip_address`, timestamps. References: `ticket_id`→Ticket, `staff_id`→Staff (for R/N). Logical sub-types: Message (M), Response (R), Note (N).

3. **Ticket Attachment** (`ticket_attachment`) — links a file to a thread entry. Identity: `attach_id`. Attributes: `ref_type` (M/R/N), timestamps. References: `ticket_id`→Ticket, `file_id`→File, `ref_id`→the thread entry.

4. **Ticket Lock** (`ticket_lock`) — exclusive edit lock. Identity: `lock_id`; unique on `ticket_id` (one lock per ticket). Attributes: `expire`, `created`. References: `ticket_id`→Ticket, `staff_id`→holding Staff.

5. **Ticket Email Info** (`ticket_email_info`) — maps inbound message-ids to thread entries for email threading. Attributes: `email_mid` (message id), raw `headers`. References: `message_id`→thread entry id.

6. **Ticket Event** (`ticket_event`) — append-only lifecycle log (FS-091.13). Attributes: `state` (created/closed/reopened/assigned/transferred/overdue), `staff` name (default SYSTEM), `annulled`, `timestamp`. Snapshot references: `ticket_id`, `staff_id`, `team_id`, `dept_id`, `topic_id`.

7. **Ticket Priority** (`ticket_priority`) — priority catalog (FS-091.5). Identity: `priority_id`; unique `priority` tag. Attributes: `priority_desc`, `priority_color`, `priority_urgency`, `ispublic`.

#### Routing & SLA

8. **Department** (`department`) — routing unit. Identity: `dept_id`; unique `dept_name`. Attributes: `dept_signature`, `ispublic`, `group_membership`, `ticket_auto_response`, `message_auto_response`, timestamps. References: `tpl_id`→Email Template Group, `sla_id`→SLA, `email_id`/`autoresp_email_id`→Email account, `manager_id`→Staff.

9. **Team** (`team`) — cross-department assignment group. Identity: `team_id`; unique `name`. Attributes: `isenabled`, `noalerts`, `notes`, timestamps. References: `lead_id`→Staff (team lead).

10. **Team Member** (`team_member`) — staff-to-team membership. Composite key `(team_id, staff_id)`; `updated` timestamp.

11. **Help Topic** (`help_topic`) — public/internal request category and routing default. Identity: `topic_id`; self-reference `topic_pid` (parent); unique `(topic, topic_pid)`. Attributes: `isactive`, `ispublic`, `noautoresp`, `notes`, timestamps. References: `priority_id`, `dept_id`, `staff_id`, `team_id`, `sla_id`, `page_id`.

12. **SLA** (`sla`) — service-level plan (FS-091.6). Identity: `id`; unique `name`. Attributes: `isactive`, `enable_priority_escalation`, `disable_overdue_alerts`, `grace_period` (hours), `notes`, timestamps.

13. **Filter** (`filter`) — inbound routing/rejection rule set. Identity: `id`. Attributes: `execorder` (default 99), `isactive`, `match_all_rules`, `stop_onmatch`, `reject_ticket`, `use_replyto_email`, `disable_autoresponder`, `target` (Any/Web/Email/API), `name`, timestamps. Action references (applied on match): `canned_response_id`, `email_id`, `priority_id`, `dept_id`, `staff_id`, `team_id`, `sla_id`.

14. **Filter Rule** (`filter_rule`) — one match condition. Identity: `id`; unique `(filter_id, what, how, val)`. Attributes: `what` (name/email/subject/body/header), `how` (equal/not_equal/contains/dn_contain/starts/ends), `val`, `isactive`, timestamps. References: `filter_id`→Filter.

#### Staff & Authorization

15. **Staff** (`staff`) — staff user account. Identity: `staff_id`; unique `username`. Attributes: `firstname`, `lastname`, `passwd` (hash), `email`, `phone`/`phone_ext`/`mobile`, `signature`, `notes`, flags `isactive`/`isadmin`/`isvisible`/`onvacation`/`assigned_only`/`show_assigned_tickets`/`daylight_saving`/`change_passwd`, `max_page_size`, `auto_refresh_rate`, `default_signature_type` (none/mine/dept), `default_paper_size` (Letter/Legal/A4/A3), `created`, `lastlogin`, `passwdreset`, `updated`. References: `group_id`→Group, `dept_id`→Department, `timezone_id`→Timezone.

16. **Group** (`groups`) — staff permission role. Identity: `group_id`. Attributes: `group_enabled`, `group_name`, and the per-action permission flags `can_create_tickets`, `can_edit_tickets`, `can_post_ticket_reply`, `can_delete_tickets`, `can_close_tickets`, `can_assign_tickets`, `can_transfer_tickets`, `can_ban_emails`, `can_manage_premade`, `can_manage_faq`, `can_view_staff_stats`, `notes`, timestamps.

17. **Group Department Access** (`group_dept_access`) — which departments a group may work. Composite unique key `(group_id, dept_id)`.

18. **Session** (`session`) — persisted login session. Identity: `session_id`. Attributes: `session_data` (blob), `session_expire`, `session_updated`, `user_id` (staff id), `user_ip`, `user_agent`.

19. **Syslog** (`syslog`) — system log (FS-091.8). Identity: `log_id`. Attributes: `log_type` (Debug/Warning/Error), `title`, `log`, `logger`, `ip_address`, timestamps.

#### Email

20. **Email Account** (`email`) — inbound/outbound mailbox (FS-091.12). Identity: `email_id`; unique `email`. Attributes: `noautoresp`, inbound (`userid`, `userpass`, `mail_active`, `mail_host`, `mail_protocol` POP/IMAP, `mail_encryption` NONE/SSL, `mail_port`, `mail_fetchfreq`, `mail_fetchmax`, `mail_archivefolder`, `mail_delete`, `mail_errors`, `mail_lasterror`, `mail_lastfetch`), outbound (`smtp_active`, `smtp_host`, `smtp_port`, `smtp_secure`, `smtp_auth`, `smtp_spoofing`), `name`, `notes`, timestamps. References: `priority_id` (default `2` = `normal`), `dept_id`. SMTP defaults: `smtp_secure` = 1, `smtp_auth` = 1, `smtp_active` = 0, `smtp_spoofing` = 0.

21. **Email Template Group** (`email_template_group`) — named template set (FS-091.7). Identity: `tpl_id`. Attributes: `isactive`, `name`, `notes`, timestamps.

22. **Email Template** (`email_template`) — one rendered message template. Identity: `id`; unique `(tpl_id, code_name)`. Attributes: `code_name`, `subject`, `body`, timestamps. References: `tpl_id`→Email Template Group.

#### Content

23. **Canned Response** (`canned_response`) — reusable reply snippet. Identity: `canned_id`; unique `title`. Attributes: `isenabled`, `response`, `notes`, timestamps. References: `dept_id`.

24. **Canned Attachment** (`canned_attachment`) — file linked to a canned response. Composite key `(canned_id, file_id)`.

25. **FAQ** (`faq`) — knowledge-base article. Identity: `faq_id`; unique `question`. Attributes: `ispublished`, `answer`, `keywords`, `notes`, `created`/`updated` (date). References: `category_id`→FAQ Category.

26. **FAQ Category** (`faq_category`) — FAQ grouping. Identity: `category_id`. Attributes: `ispublic`, `name`, `description`, `notes`, dates.

27. **FAQ Attachment** (`faq_attachment`) — file linked to an FAQ. Composite key `(faq_id, file_id)`.

28. **FAQ Topic** (`faq_topic`) — links an FAQ to a help topic. Composite key `(faq_id, topic_id)`.

29. **Page** (`page`) — editable site page (FS-091.10). Identity: `id`; unique `name`. Attributes: `isactive`, `type` (landing/offline/thank-you/other), `body`, `notes`, timestamps.

#### Files

30. **File** (`file`) — content-addressed file record (FS-091.14). Identity: `id`. Attributes: `ft` category code (default T), MIME `type`, `size`, content `hash`, `name`, `created`.

31. **File Chunk** (`file_chunk`) — ordered byte chunk. Composite key `(file_id, chunk_id)`; `filedata` blob. Chunk size 512,000 bytes.

#### Platform

32. **Config** (`config`) — key/value settings (FS-091.7, BS-091.7). Identity: `id`; unique `(namespace, key)`. Attributes: `value`, auto-updating `updated` timestamp. All installed keys use namespace `core`.

33. **API Key** (`api_key`) — external integration credential. Identity: `id`; unique `apikey`. Attributes: `isactive`, `ipaddr` (IP-bound), `can_create_tickets` (default 1), `can_exec_cron` (default 1), `notes`, timestamps.

34. **Timezone** (`timezone`) — UTC-offset lookup. Identity: `id`. Attributes: `offset` (float hours), `timezone` (label). Seeded with 30 standard zones (see Seeded Timezones).

> **Table count**: Entities #1–#34 above are the complete set of installed tables. The installer creates exactly 34 tables (verified by counting `CREATE TABLE` statements in `install-mysql.sql`); there is no 35th table. Earlier drafts of this spec erroneously claimed 35 — corrected per FS-091.15 / KL-091.9.

### Enumerations (Canonical Literal Sets)

| Enum set | Owner field | Exact values | Default | Defined in |
|----------|-------------|--------------|---------|------------|
| Ticket status | `ticket.status` | `open`, `closed` | `open` | FS-091.3 |
| Ticket source | `ticket.source` | `Web`, `Email`, `Phone`, `API`, `Other` | `Other` | FS-091.3 |
| Thread entry type | `ticket_thread.thread_type` | `M`, `R`, `N` | (none) | FS-091.4 |
| Attachment ref type | `ticket_attachment.ref_type` | `M`, `R`, `N` | `M` | FS-091.4 |
| Ticket event state | `ticket_event.state` | `created`, `closed`, `reopened`, `assigned`, `transferred`, `overdue` | (none) | FS-091.13 |
| Filter match field | `filter_rule.what` | `name`, `email`, `subject`, `body`, `header` | (none) | FS-091.9 |
| Filter operator | `filter_rule.how` | `equal`, `not_equal`, `contains`, `dn_contain`, `starts`, `ends` | (none) | FS-091.9 |
| Filter target | `filter.target` | `Any`, `Web`, `Email`, `API` | `Any` | FS-091.9 |
| Origin→target map | (runtime) | `web`→Web, `email`→Email, `phone`→Web, `staff`→Web, `api`→API | — | FS-091.3 |
| Mail protocol | `email.mail_protocol` | `POP`, `IMAP` | `POP` | FS-091.12 |
| Mail encryption | `email.mail_encryption` | `NONE`, `SSL` | (none) | FS-091.12 |
| Log severity | `syslog.log_type` | `Debug`, `Warning`, `Error` | (none) | FS-091.8 |
| Page type | `page.type` | `landing`, `offline`, `thank-you`, `other` | `other` | FS-091.10 |
| Staff signature mode | `staff.default_signature_type` | `none`, `mine`, `dept` | `none` | FS-091.11 |
| Staff paper size | `staff.default_paper_size` | `Letter`, `Legal`, `A4`, `A3` | `Letter` | FS-091.11 |
| Email template code-names | `email_template.code_name` | (13-value set in FS-091.7) | — | FS-091.7 |

### Seeded Reference Rows

**Seeded Priorities** (4 rows; lower urgency = more urgent):

| priority (tag) | priority_desc | priority_color | priority_urgency | ispublic |
|----------------|---------------|----------------|------------------|----------|
| low | Low | `#DDFFDD` | 4 | 1 |
| normal | Normal | `#FFFFF0` | 3 | 1 |
| high | High | `#FEE7E7` | 2 | 1 |
| emergency | Emergency | `#FEE7E7` | 1 | 0 |

(Installed `default_priority_id` = 2 → `normal`. `emergency` is staff-only — `ispublic` = 0. Note `high` and `emergency` share the same color.)

**Seeded SLA** (1 row): `name` = "Default SLA", `grace_period` = 48 (hours), `isactive` = 1, `enable_priority_escalation` = 1, `disable_overdue_alerts` = 0.

**Seeded Departments** (2 rows): "Support" (`sla_id` = 0 → default SLA, signature "Support Dept", public, auto-responses on) and "Billing" (`sla_id` = the default SLA's id, signature "Billing Dept", public, auto-responses on).

**Seeded Help Topics** (2 rows): "Support" (active, public, dept = first department, SLA = default SLA) and "Billing" (active, public, dept = first department, `sla_id` = 0).

**Seeded Groups** (3 rows): "Admins" (notes "overlords"; all permissions including delete/ban/premade/faq), "Managers" (same broad permissions), "Staff" (no delete, no ban, no manage-premade, no manage-faq). All three enabled. The seed `INSERT` does not set `can_post_ticket_reply` or `can_view_staff_stats`, so all three groups take the column defaults for those two flags: `can_post_ticket_reply` = 1 (granted to all) and `can_view_staff_stats` = 0 (denied to all, including Admins/Managers). The Staff group keeps `can_create_tickets`/`can_edit_tickets`/`can_close_tickets`/`can_assign_tickets`/`can_transfer_tickets` = 1. Group↔department access seeded as the full Cartesian product (BS-091.8).

**Seeded Team** (1 row): "Level I Support" (enabled, alerts on, no members seeded).

**Seeded Filter / Ban List** (1 filter + 1 sample rule): "SYSTEM BAN LIST" (exec order 99, reject-on-match, active) with one demo rule `email equal test@example.com` (BS-091.9).

**Seeded Email Template Group** (1 group + 12 templates): "osTicket Default Template" (active) containing the twelve seeded templates listed below (the thirteenth, `staff.pwreset`, is loaded from initial data on demand). Installed `default_template_id` = 1.

| code_name | seeded subject |
|-----------|----------------|
| ticket.autoresp | `Support Ticket Opened [#%{ticket.number}]` |
| ticket.autoreply | `Support Ticket Opened [#%{ticket.number}]` |
| ticket.notice | `[#%{ticket.number}] %{ticket.subject}` |
| ticket.alert | `New Ticket Alert` |
| message.autoresp | `[#%{ticket.number}] Message Added` |
| message.alert | `New Message Alert` |
| note.alert | `New Internal Note Alert` |
| assigned.alert | `Ticket #%{ticket.number} Assigned to you` |
| transfer.alert | `Ticket Transfer #%{ticket.number} - %{ticket.dept.name}` |
| ticket.overdue | `Stale Ticket Alert` |
| ticket.overlimit | `Open Tickets Limit Reached` |
| ticket.reply | `[#%{ticket.number}] %{ticket.subject}` |

**Seeded Canned Responses** (2 rows): "What is osTicket (sample)?" and "Sample (with variables)" (the latter demonstrating `%{ticket.name}`, `%{ticket.number}`, `%{ticket.create_date}`, `%{ticket.dept.name}` tokens). The sample with-variables canned response is linked to the seeded sample file via canned_attachment.

**Seeded File** (1 row): MIME `text/plain`, size 25, name `osTicket.txt`, with one seeded chunk containing the literal bytes "Canned attachments rock!\n".

**Seeded Pages** (3 rows): "Offline" (type offline), "Thank you" (type thank-you), "Landing" (type landing) — all active; their ids are written into `offline_page_id`, `thank-you_page_id`, `landing_page_id`.

**Seeded Timezones** (30 rows): offset → label pairs from `-12.0` (Eniwetok, Kwajalein) through `0.0` (Western Europe Time, London, Lisbon, Casablanca) to `+12.0` (Auckland, Wellington, Fiji, Kamchatka), including half-hour offsets `-3.5` (Newfoundland), `+3.5` (Tehran), `+4.5` (Kabul), `+5.5` (Bombay, Calcutta, Madras, New Delhi), `+9.5` (Adelaide, Darwin).

### Installed Configuration Keys & Defaults (namespace `core`)

The following keys are seeded at install. Each is namespaced `core`; `(namespace, key)` is unique. Selected ranges/notes noted where the source constrains them.

| Key | Installed default | Notes / range |
|-----|-------------------|---------------|
| isonline | 0 | helpdesk online flag |
| enable_daylight_saving | 0 | |
| staff_ip_binding | 0 | bind staff session to IP |
| staff_max_logins | 4 | failed-login lockout count |
| staff_login_timeout | 2 | minutes lockout |
| staff_session_timeout | 30 | minutes |
| passwd_reset_period | 0 | days; 0 = never |
| client_max_logins | 4 | |
| client_login_timeout | 2 | minutes |
| client_session_timeout | 30 | minutes |
| max_page_size | 25 | list rows per page |
| max_open_tickets | 0 | per requester; 0 = unlimited; integer, required |
| max_file_size | 1048576 | bytes (1 MiB) |
| max_user_file_uploads | (empty) | per-message client upload cap |
| max_staff_file_uploads | (empty) | per-message staff upload cap |
| autolock_minutes | 3 | ticket edit auto-lock; integer, required |
| overdue_grace_period | 0 | hours added before overdue |
| alert_email_id | 0 | →Email account (0 = unset) |
| default_email_id | 0 | →Email account |
| default_dept_id | 0 | →Department |
| default_sla_id | 0 | →SLA |
| default_priority_id | 2 | →Priority (normal) |
| default_template_id | 1 | →Email Template Group |
| default_timezone_id | 0 | →Timezone |
| default_smtp_id | 0 | →Email account |
| allow_email_spoofing | 0 | |
| clickable_urls | 1 | |
| allow_priority_change | 0 | client may set priority |
| use_email_priority | 0 | honor email X-Priority |
| enable_kb | 0 | knowledge base on |
| enable_premade | 1 | canned responses on |
| enable_captcha | 0 | |
| enable_auto_cron | 0 | web-triggered cron |
| enable_mail_polling | 0 | |
| send_sys_errors | 1 | |
| send_sql_errors | 1 | |
| send_mailparse_errors | 1 | |
| send_login_errors | 1 | |
| save_email_headers | 1 | |
| strip_quoted_reply | 1 | |
| log_ticket_activity | 1 | |
| ticket_autoresponder | 0 | |
| message_autoresponder | 0 | |
| ticket_notice_active | 0 | |
| ticket_alert_active | 0 | + _admin 1, _dept_manager 1, _dept_members 0 |
| message_alert_active | 0 | + _laststaff 1, _assigned 1, _dept_manager 0 |
| note_alert_active | 0 | + _laststaff 1, _assigned 1, _dept_manager 0 |
| transfer_alert_active | 0 | + _assigned 0, _dept_manager 1, _dept_members 0 |
| overdue_alert_active | 0 | + _assigned 1, _dept_manager 1, _dept_members 0 |
| assigned_alert_active | 1 | + _staff 1, _team_lead 0, _team_members 0 |
| auto_assign_reopened_tickets | 1 | |
| show_related_tickets | 1 | |
| show_assigned_tickets | 1 | |
| show_answered_tickets | 0 | |
| show_notes_inline | 1 | |
| hide_staff_name | 0 | |
| overlimit_notice_active | 0 | |
| email_attachments | 1 | |
| allow_attachments | 0 | |
| allow_email_attachments | 0 | |
| allow_online_attachments | 0 | |
| allow_online_attachments_onlogin | 0 | |
| random_ticket_ids | 1 | 1 = random 6-digit; 0 = sequential (FS-091.2) |
| log_level | 2 | min severity persisted (FS-091.8) |
| log_graceperiod | 12 | months before log purge |
| upload_dir | (empty) | |
| allowed_filetypes | `.doc, .pdf` | comma list |
| time_format | ` h:i A` | |
| date_format | `m/d/Y` | |
| datetime_format | `m/d/Y g:i a` | |
| daydatetime_format | `D, M j Y g:ia` | |
| reply_separator | `-- do not edit --` | |
| admin_email | (empty) | |
| helpdesk_title | `osTicket Support Ticket System` | |
| helpdesk_url | (empty) | |
| schema_signature | (empty) | set during install/upgrade |
| landing_page_id | (set to seeded Landing page id) | post-page-seed |
| offline_page_id | (set to seeded Offline page id) | post-page-seed |
| thank-you_page_id | (set to seeded Thank-you page id) | post-page-seed |

### Identifier, Numbering & Audit Conventions

- **Internal vs. external ticket identity**: internal `ticket_id` is an auto-incrementing record key; the public `ticketID` is generated per FS-091.2 (random 6-digit or sequential).
- **External ticket-number length**: `EXT_TICKET_ID_LEN` = 6 (random mode draws an integer in 100000–999999).
- **Sentinel zero**: integer reference columns default to 0 meaning "unset / use system default" (BS-091.6).
- **Name uniqueness**: enforced per reference set (BS-091.5).
- **Timestamps**: `created` set once; `updated` refreshed on change; config `updated` auto-updates (BS-091.11). Mailboxes additionally track `mail_lastfetch` / `mail_lasterror`; staff track `lastlogin` / `passwdreset`.
- **Append-only history**: ticket events are appended and annulled, never updated/deleted (BS-091.12).
- **Content addressing**: files are keyed by content `hash` and stored in 512,000-byte chunks (FS-091.14).
- **Config addressing**: all settings are `(namespace, key)` pairs; installed keys use namespace `core`.

---

## User Flows / Interactions

Not applicable. This specification defines reference data, enumerations, and the logical data model only. The user-facing flows that consume these definitions are specified in their respective domain documents (see Dependencies). Administrators edit the mutable reference rows (priorities, SLAs, templates, departments, help topics, pages, config) through the admin screens defined in FS-030 through FS-033 and FS-040.

---

## Edge Cases & Error Scenarios

### EC-091.1: External Ticket-Number Collision (Random Mode)
When a freshly drawn 6-digit external number already exists, the generator discards it and re-draws, recursing until an unused value is found. With a 900,000-value space this can degrade as ticket volume approaches saturation (see KL-091.1).

### EC-091.2: Switch From Random To Sequential After Tickets Exist
Toggling `random_ticket_ids` from 1 to 0 does not renumber existing tickets; only newly created tickets receive `ticketID = ticket_id`. The two schemes can therefore coexist within one installation's ticket history.

### EC-091.3: Sequential Assignment Update Failure
In sequential mode the external number is set by a follow-up update after insert. A source comment flags that the failure path is unhandled ("RETHINK what happens if this fails"); on failure the ticket may retain its insert-time external value rather than the intended sequential one.

### EC-091.4: Reference To Sentinel-Zero Entity
A reference column holding 0 must be resolved to the corresponding system default (e.g. `default_sla_id`, `default_dept_id`). If the system default is itself 0/unset, resolution falls through to application hard-coded fallbacks (e.g. `DEFAULT_PRIORITY_ID` = 1).

### EC-091.5: Disabling An In-Use Default
Attempting to disable or delete the default email-template group (or any template still referenced by a department) is rejected with an "in use cannot be disabled" error (BS-091.10).

### EC-091.6: Duplicate Reference Name
Saving a department/SLA/team/canned-response/page/staff with a name already taken is rejected at validation time (BS-091.5); help topics permit the same `topic` under different parents.

### EC-091.7: Duplicate Filter Rule
Adding a rule identical to an existing one on the same filter is rejected by the `(filter_id, what, how, val)` uniqueness constraint (BS-091.2).

### EC-091.8: Removal Of The System Ban List
The reserved "SYSTEM BAN LIST" filter must not be removed; its notes explicitly warn against deletion. Removing it would break the email-banning mechanism (FS-042).

### EC-091.9: Log Level Suppression
Entries below the configured `log_level` are not persisted; lowering the level mid-operation begins capturing lower-severity entries without backfilling earlier ones. Aged entries are purged per `log_graceperiod` (12 months default).

### EC-091.10: Same-Color Priorities
The seeded `high` and `emergency` priorities share color `#FEE7E7`; color alone does not distinguish them — urgency rank (2 vs. 1) and the public flag do.

---

## Dependencies

- **FS-001 (App bootstrap & shared request lifecycle)** — loads the `config` table into the runtime `$cfg` settings object; defines `TABLE_PREFIX`, `EXT_TICKET_ID_LEN`, and other constants consumed here.
- **FS-002 (Staff authentication & access control)** — consumes Group permission flags, Group↔Department access, Staff, and Session entities.
- **FS-003 (Crypto/validation/formatting + logging/timezone)** — owns the runtime semantics of the Syslog severity levels and the Timezone lookup; references the log-level config.
- **FS-010 / FS-011 (Client portal & ticket submission)** — consume ticket external-number lookup, public priorities/help-topics, page types.
- **FS-021 (Staff ticket workflow)** — consumes ticket status/source/flags, thread entry types, event states, lock, priority, SLA.
- **FS-022 (Canned responses & attachments)** — consumes Canned Response, Canned Attachment, File, File Chunk.
- **FS-030 / FS-031 / FS-032 / FS-033 (Admin config)** — edit Departments, Teams, Help Topics, Staff, Groups, SLA, Priority, Pages, Logs, and the config keys catalogued here.
- **FS-040 / FS-041 / FS-042 (Email pipeline & filters)** — consume Email account, Email Template Group/Template, Filter, Filter Rule, the SYSTEM BAN LIST, and the origin→target map; the `%{...}` token catalog lives in FS-040.
- **FS-043 (API & cron)** — consumes the API Key entity and its IP-binding / capability flags.
- **FS-050 (Knowledge base / FAQ)** — consumes FAQ, FAQ Category, FAQ Attachment, FAQ Topic.
- **FS-060 / FS-061 (Installer & upgrader)** — the installer applies the seed rows and config defaults defined here; the upgrader migrates this schema forward and maintains `schema_signature`.

> This document is the canonical source for all of the above; consumers cross-reference FS-091.N / BS-091.N rather than restating enum literals or defaults.

---

## Known Limitations

### KL-091.1: Random External Numbers Saturate The 6-Digit Space
With only 900,000 possible 6-digit values, an installation with hundreds of thousands of tickets sees rising collision/re-roll cost in random mode; the length is fixed by `EXT_TICKET_ID_LEN` = 6 and not configurable at runtime.

### KL-091.2: Sequential-Mode Failure Path Is Unhandled
The post-insert sequential-number update has no recovery on failure (acknowledged by an in-source TODO), so a failed update can leave a ticket with an unintended external number.

### KL-091.3: Composite Ticket Uniqueness Is Weaker Than Application Behavior
The schema only enforces `(ticketID, email)` uniqueness; global `ticketID` uniqueness is an application convention (re-roll on collision), not a database guarantee — a direct data import bypassing the application could create duplicate external numbers across different emails.

### KL-091.4: Help-Topic `staff_id`/`team_id` Index Anomaly
The ticket table indexes `team_id` using the `staff_id` column (`KEY team_id (staff_id)`), a likely copy-paste anomaly in the schema; team-id-only lookups are therefore not separately indexed.

### KL-091.5: Email Template `staff.pwreset` Is Not Seeded Into The Group
Twelve of the thirteen catalogued templates are seeded; `staff.pwreset` is loaded lazily from an initial-data file and absent from the seeded group, so a fresh group has no password-reset template until that path runs.

### KL-091.6: No Foreign-Key Enforcement
Inter-table references are plain integer columns with helper indexes, not enforced foreign keys; referential integrity (and the sentinel-zero convention) is maintained by application code only, so orphaned references are possible after manual data edits.

### KL-091.7: Single Configuration Namespace In Core
All installed settings use the `core` namespace; the namespace dimension of the `(namespace, key)` key exists for future/plugin partitioning but is unused by the core install.

### KL-091.8: Two Seeded Priorities Share A Color
`high` and `emergency` are installed with the same `#FEE7E7` color, so any UI relying on color alone cannot distinguish them; they differ only by urgency rank and public flag.

### KL-091.9: The Core Install Creates 34 Tables, Not 35
Earlier prose described the schema as "35 tables", but the installer's `install-mysql.sql` stream contains exactly 34 `CREATE TABLE` statements (verified count). The authoritative inventory is FS-091.15. The off-by-one arose from double-counting the `email_template_group` / `email_template` pair; both are real, distinct tables already enumerated as entities #21 and #22, so the true total is 34.

### KL-091.10: `ticket_email_info` Index Name Misnames Its Column
The `ticket_email_info` table declares `KEY message_id (email_mid)` — the index is *named* `message_id` but actually indexes the `email_mid` column, not the `message_id` reference column. As a result the `message_id` → thread-entry lookup column is not separately indexed, while `email_mid` (used for inbound message-id threading) is. This is a likely naming slip in the schema, analogous to the `team_id`/`staff_id` index anomaly (KL-091.4).
