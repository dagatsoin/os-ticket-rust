# FS-032: Admin — System Settings, SLA Plans, Priorities & FAQ Categories

## Overview

The Admin System Settings, SLA, Priorities & Categories specification defines the
administration surface through which a privileged staff member configures the
help desk's global behavior. It owns four distinct but related areas:

1. **System / preference settings** — the seven-tab Settings panel
   (`settings.php`) that persists ~110 namespaced configuration key/value pairs
   governing system identity, ticket defaults, dates/times, authentication
   windows, attachments, autoresponders, alerts & notices, knowledge-base
   toggles, email routing, and site pages/logo.
2. **SLA (Service Level Agreement) plans** — the CRUD lifecycle for grace-period
   plans (`slas.php`) that drive a ticket's overdue computation, priority
   escalation, transient-override behavior, and per-plan overdue-alert
   suppression.
3. **Ticket priorities** — the priority reference set consumed by the ticket
   defaults tab (a fixed seed set of four priorities with urgency ranks,
   colors, and a public/internal flag).
4. **FAQ categories** — the CRUD lifecycle for knowledge-base article categories
   (`categories.php`) with a public/internal flag.

Settings persistence is mediated by a generic namespaced configuration store
(`Config` / `OsticketConfig` over the `config` table, namespace `core`), which
returns a database value when present, otherwise a hard-coded default, otherwise
a session-cached fallback. Each settings tab is validated and saved by a
dedicated update method; an invalid POST re-renders the form from the submitted
values rather than the stored ones.

**Scope boundaries.** This spec owns: the settings tab inventory and every field
on each tab (name, exact default, allowed range/options, effect, and validation);
the SLA plan CRUD, its overdue/grace semantics and deletion constraints; the
priority reference set as consumed here; and FAQ category CRUD. It does NOT own:
the database table schemas and enum canon (FS-091); the consuming behaviors
(ticket overdue alert delivery, email pipeline, autoresponder/alert *sending*,
the KB public rendering, site-page editing, logo file storage internals); those
are referenced as dependencies. The bootstrap of `$cfg`/`OsticketConfig` at
request start belongs to FS-001; this spec covers the *write* path and the
*per-field semantics* of the stored values.

---

## Functional Requirements

### FS-032.1: Settings Panel Entry & Tab Routing

**Description**: The system shall present a settings administration panel
(`settings.php`) organized into seven named tabs, routed by a `t` request
parameter, and accessible only to administrators.

**Acceptance Criteria**:
- The seven tabs and their `t` keys are, in order:

  | Order | `t` key | Tab label |
  |-------|---------|-----------|
  | 1 | `system` | System Settings |
  | 2 | `tickets` | Ticket Settings and Options |
  | 3 | `emails` | Email Settings |
  | 4 | `pages` | Site Pages |
  | 5 | `kb` | Knowledgebase Settings |
  | 6 | `autoresp` | Autoresponder Settings |
  | 7 | `alerts` | Alerts and Notices Settings |

- The active tab is resolved as: use `t` from the request if it names a known
  tab key, otherwise default to `system`.
- **Attachments settings tab (out-of-band)**: in addition to the seven tabs above,
  the admin **Attachments** system-settings screen is delivered by a standalone
  `staff/attachment.inc.php` partial (form action `admin.php?t=attach`) rather than
  by a `settings-{t}` partial, so it is not one of the seven `t`-routed tabs even
  though it is an admin system-settings screen. It persists the attachment-policy
  config keys `allow_attachments`, `allow_email_attachments`,
  `allow_online_attachments`, `allow_online_attachments_onlogin`,
  `email_attachments`, `max_file_size`, `upload_dir`, and `allowed_filetypes`
  (the master switch + sub-flags + size cap + storage dir + allowed-types list;
  the type/size enforcement rules are owned by FS-022.13, cross-referenced here).
- Each tab body is rendered by a matching partial named `settings-{t}` (e.g.
  `settings-system`, `settings-tickets`). The `system`, `tickets`, `emails`,
  `pages`, and `kb` partials independently re-assert the gate: they die with
  "Access Denied" unless the admin-context constant is defined AND the acting
  staff exists AND `isAdmin()` is true AND a `$config` array is present. The
  `alerts` and `autoresp` partials do NOT carry this guard — they rely solely on
  the entry-script (`admin.inc.php`) gate that ran before inclusion (see
  KL-032.8).
- The navigation marks the "settings" tab active with the URL
  `settings.php?t={target}`.

### FS-032.2: Settings Save Dispatch & Per-Tab Validation

**Description**: On a settings POST, the system shall dispatch to the update
method for the submitted tab (`t`), validate the tab's required fields, and on
success report "{tab label} Updated Successfully".

**Acceptance Criteria**:
- The dispatcher (`updateSettings`) branches on a case-insensitive `t` value to
  `updateSystemSettings`, `updateTicketsSettings`, `updateEmailsSettings`,
  `updatePagesSettings`, `updateAutoresponderSettings`, `updateAlertsSettings`,
  or `updateKBSettings`; an unrecognized `t` sets the error
  "Unknown setting option. Get technical support." and saves nothing.
- A successful save reports `"{settingOptions[t]} Updated Successfully"` (the
  tab's human label).
- A failed save with no specific message reports the generic
  "Unable to update settings - correct errors below and try again".
- When the POST has validation errors, the form is re-populated from the
  submitted input (`Format::input($_POST)`); otherwise it is populated from the
  stored configuration (`Format::htmlchars($cfg->getConfigInfo())`).
- A persisted value is written only if it differs from the currently stored
  value (no-op writes are skipped); a key not yet present in the database is
  inserted, an existing key is updated with a refreshed `updated` timestamp.
- Each tab save is all-or-nothing within `updateAll`: it iterates the field map
  and aborts on the first write failure (returns false).

### FS-032.3: System Settings Tab Fields

**Description**: The System Settings tab shall expose general identity,
default-routing, page-size, logging, authentication-window, and date/time
configuration with the fields, defaults, and ranges below.

**Acceptance Criteria** — the tab persists exactly these fields:

**General**
- **Helpdesk Status** (`isonline`, radio): Online (`1`) / Offline (`0`).
  Default `0` (Offline). Offline mode disables the client interface and only
  allows admins into the Staff Control Panel.
- **Helpdesk URL** (`helpdesk_url`, text): required string. Default empty.
- **Helpdesk Name/Title** (`helpdesk_title`, text): required string.
  Default `osTicket Support Ticket System`.
- **Default Department** (`default_dept_id`, select): required int; options are
  public departments only (`ispublic=1`). Default `0`.
- **Default Email Templates** (`default_template_id`, select): required int;
  options are active email template groups. Default `1`.
- **Default Page Size** (`max_page_size`, select): values `5,10,…,50` in steps
  of 5. Default `25`.
- **Default Log Level** (`log_level`, select): `0` None/disabled, `3` DEBUG,
  `2` WARN, `1` ERROR. Default `2` (WARN).
- **Purge Logs** (`log_graceperiod`, select): `0` Never, else "After N Months"
  for N in 1..12. Default `12`.

**Authentication**
- **Password Change Policy** (`passwd_reset_period`, select): `0` None, else
  N in 1..12 ("Monthly" for 1, "Every N Months" otherwise). Default `0`.
- **Allow Password Resets** (`allow_pw_reset`, checkbox): enables the
  "Forgot my password" link. Code default `true`.
- **Password Reset Window** (`pw_reset_window`, text): required int, minimum 1;
  maximum minutes a reset token is valid. Code default `30`. (Consumed as
  seconds: stored minutes × 60.)
- **Staff Excessive Logins** (`staff_max_logins` 1..10, `staff_login_timeout`
  1..10 minutes): N failed attempts allowed before an M-minute lockout.
  Defaults `4` and `2`.
- **Staff Session Timeout** (`staff_session_timeout`, text): required int; idle
  minutes before re-login (0 disables). Default `30`. (Consumed as seconds.)
- **Client Excessive Logins** (`client_max_logins` 1..10,
  `client_login_timeout` 1..10): defaults `4` and `2`.
- **Client Session Timeout** (`client_session_timeout`, text): required int;
  idle minutes before re-login (0 disables). Default `30`.
- **Bind Staff Session to IP** (`staff_ip_binding`, checkbox): default `0`.

**Date & Time**
- **Time Format** (`time_format`, text): required PHP date-format string.
  Default ` h:i A`.
- **Date Format** (`date_format`, text): required. Default `m/d/Y`.
- **Date & Time Format** (`datetime_format`, text): required. Default
  `m/d/Y g:i a`.
- **Day, Date & Time Format** (`daydatetime_format`, text): required. Default
  `D, M j Y g:ia`.
- **Default Time Zone** (`default_timezone_id`, select): required int; options
  enumerated from the timezone table as "GMT {offset} - {name}". Default `0`.
- **Daylight Saving** (`enable_daylight_saving`, checkbox): default `0`.

- Each date/time field renders a live preview of the current GMT time formatted
  with the entered pattern, the stored timezone offset, and the DST flag.
- Required fields (`helpdesk_url`, `helpdesk_title`, `default_dept_id`,
  `default_template_id`, `staff_session_timeout`, `client_session_timeout`, the
  four format strings, `default_timezone_id`, `pw_reset_window`) each produce a
  named error on omission; `pw_reset_window` additionally rejects values < 1.

### FS-032.4: Ticket Settings & Options Tab Fields

**Description**: The Ticket Settings tab shall expose ticket-default routing,
behavioral toggles, and attachment policy.

**Acceptance Criteria** — the tab persists exactly these fields:

**Ticket defaults & behavior**
- **Ticket IDs** (`random_ticket_ids`, radio): `0` Sequential / `1` Random.
  Default `1` (Random; "highly recommended").
- **Default SLA** (`default_sla_id`, select, required): `0` None, else the SLA
  plan list. Each option label is formatted `"{name} ({grace_period} hrs -
  Active|Disabled)"` (from `SLA::getSLAs()`, ordered by name ascending); the
  list includes disabled plans, not just active ones.
- **Default Priority** (`default_priority_id`, select, required): enumerated
  from the priority set by `priority_id` → `priority_desc`. The select query has
  no ORDER BY, so options appear in the table's natural (insertion / id) order.
  Default `2` (Normal).
- **Maximum Open Tickets** (`max_open_tickets`, int, required): per email/user;
  `0` = unlimited. Default `0`.
- **Ticket Auto-lock Time** (`autolock_minutes`, int, required): minutes to lock
  a ticket on activity; `0` disables locking. Default `3`.
- **Web Tickets Priority** (`allow_priority_change`, checkbox): allow the user to
  set/override priority. Default `0`.
- **Emailed Tickets Priority** (`use_email_priority`, checkbox): use email
  priority header when available. Default `0`.
- **Show Related Tickets** (`show_related_tickets`, checkbox): default `1`.
- **Show Notes Inline** (`show_notes_inline`, checkbox): default `1`.
- **Clickable URLs** (`clickable_urls`, checkbox): default `1`.
- **Human Verification** (`enable_captcha`, checkbox): CAPTCHA on new web
  tickets; requires the GD image library. Default `0`.
- **Reopened Tickets** (`auto_assign_reopened_tickets`, checkbox): auto-assign
  to the last respondent. Default `1`.
- **Assigned Tickets** (`show_assigned_tickets`, checkbox): show on open queue.
  Default `1`.
- **Answered Tickets** (`show_answered_tickets`, checkbox): show on open queue.
  Default `0`.
- **Ticket Activity Log** (`log_ticket_activity`, checkbox): log activity as
  internal notes. Default `1`.
- **Staff Identity Masking** (`hide_staff_name`, checkbox): hide staff name on
  responses. Default `0`.

**Attachments**
- **Allow Attachments** (`allow_attachments`, checkbox, global master switch):
  default `0`.
- **Emailed/API Attachments** (`allow_email_attachments`, checkbox): default `0`.
- **Online/Web Attachments** (`allow_online_attachments`, checkbox) plus
  **authenticated-only** sub-flag (`allow_online_attachments_onlogin`,
  checkbox): defaults `0` and `0`.
- **Max. User File Uploads** (`max_user_file_uploads`, select 1..N): default
  empty. The upper bound N is the smaller of the platform
  `max_file_uploads` directive and the system default cap.
- **Max. Staff File Uploads** (`max_staff_file_uploads`, select 1..N): default
  empty.
- **Maximum File Size** (`max_file_size`, int, in bytes): default `1048576`
  (1 MiB). The form advisory shows the platform upload-size ceiling.
- **Ticket Response Files** (`email_attachments`, checkbox): email attachments
  to the user. Default `1`.
- **Accepted File Types** (`allowed_filetypes`, textarea): comma-separated
  extension list; `.*` accepts all (discouraged). Default `.doc, .pdf`. On save
  the value is lowercased and newlines are stripped.

- Attachment validation runs only when `allow_attachments` is checked, and then
  requires: the platform `file_uploads` directive enabled; a numeric
  `max_file_size`; a non-empty `allowed_filetypes`; and both upload-count caps
  present and not exceeding the platform `max_file_uploads` bound.
- CAPTCHA validation: if `enable_captcha` is checked, the GD library must be
  loaded and PNG image support available, else a field error is raised.

### FS-032.5: Email Settings Tab Fields

**Description**: The Email Settings tab shall configure default system/alert
email accounts, the admin contact address, inbound polling, quoted-reply
stripping, and the default outgoing (SMTP) account.

**Acceptance Criteria** — the tab persists exactly these fields:
- **Default System Email** (`default_email_id`, select, required): an email
  account id. Default `0`.
- **Default Alert Email** (`alert_email_id`, select, required): `0` = use the
  default system email; options exclude the default system email. Default `0`.
- **Admin's Email Address** (`admin_email`, email, required): default empty.
  Validated as an email and rejected if it matches an existing system email
  account ("Email already setup as system email").
- **Email Polling** (`enable_mail_polling`, checkbox): enable POP/IMAP polling.
  Default `0`.
- **Poll on auto-cron** (`enable_auto_cron`, checkbox): poll based on staff
  activity (not recommended). Default `0`.
- **Strip Quoted Reply** (`strip_quoted_reply`, checkbox): default `1`. If
  checked, a non-empty reply separator is required.
- **Reply Separator Tag** (`reply_separator`, text): default `-- do not edit --`.
- **Default Outgoing Email** (`default_smtp_id`, select): `0` = use the platform
  mail function; options are SMTP-active email accounts. Default `0`.

### FS-032.6: Site Pages, Logos, Autoresponder, Knowledge-Base & Alerts Tabs

**Description**: The remaining four settings tabs shall configure default site
pages and the client logo, the four autoresponder toggles, the two
knowledge-base toggles, and the full alerts-and-notices recipient matrix.

**Acceptance Criteria**:

**Site Pages tab (`pages`)** — persists:
- **Default Landing Page** (`landing_page_id`, required), **Default Offline
  Page** (`offline_page_id`, required), **Default Thank-You Page**
  (`thank-you_page_id`, required): each a page id filtered to its page type.
- **Client Logo** (`client_logo_id`): a radio chooses the system default logo
  (value `0`) or a previously uploaded custom logo; a file input uploads a new
  logo; delete checkboxes remove non-selected logos (with a JS confirmation
  dialog before submit). The currently selected logo cannot be deleted.
- **Logo image validation** (`AttachmentFile::uploadLogo`, BS-032.17): an uploaded
  logo must be a GIF/JPEG/PNG image whose **width-to-height aspect ratio is below
  3:1** (`$aspect_ratio = 3`). An image whose ratio is **≥ 3:1** is rejected with
  **"Image is too square. Upload a wider image"**; a non-GIF/JPEG/PNG image is
  rejected with **"Invalid image file type"**. The check requires the GD image
  extension; when GD is **not** loaded the dimension/ratio validation is skipped
  entirely and the file is stored as a logo (`ft='L'`) unvalidated (EC-032.15).

**Autoresponder tab (`autoresp`)** — persists four enable/disable radios:
- **New Ticket** (`ticket_autoresponder`, default `0`).
- **New Ticket by staff** (`ticket_notice_active`, default `0`) — notice when
  staff opens a ticket on a user's behalf.
- **New Message** (`message_autoresponder`, default `0`).
- **Overlimit notice** (`overlimit_notice_active`, default `0`) — ticket-denied
  notice on open-ticket-limit violation.
- This tab performs no field validation; it writes the four flags directly.

**Knowledgebase tab (`kb`)** — persists:
- **Knowledge base status** (`enable_kb`, checkbox, default `0`) — enables the
  client KB interface.
- **Canned Responses** (`enable_premade`, checkbox, default `1`) — enables
  canned responses on ticket reply.

**Alerts and Notices tab (`alerts`)** — persists a recipient matrix; each event
has an active radio (Enable/Disable) plus recipient checkboxes:

| Event | Active flag (default) | Recipient checkboxes (default) |
|-------|----------------------|-------------------------------|
| New Ticket | `ticket_alert_active` (0) | `ticket_alert_admin` (1), `ticket_alert_dept_manager` (1), `ticket_alert_dept_members` (0) |
| New Message | `message_alert_active` (0) | `message_alert_laststaff` (1), `message_alert_assigned` (1), `message_alert_dept_manager` (0) |
| New Internal Note | `note_alert_active` (0) | `note_alert_laststaff` (1), `note_alert_assigned` (1), `note_alert_dept_manager` (0) |
| Ticket Assignment | `assigned_alert_active` (1) | `assigned_alert_staff` (1), `assigned_alert_team_lead` (0), `assigned_alert_team_members` (0) |
| Ticket Transfer | `transfer_alert_active` (0) | `transfer_alert_assigned` (0), `transfer_alert_dept_manager` (1), `transfer_alert_dept_members` (0) |
| Overdue Ticket | `overdue_alert_active` (0) | `overdue_alert_assigned` (1), `overdue_alert_dept_manager` (1), `overdue_alert_dept_members` (0) |

- **System Alerts**: `send_sys_errors` is rendered as a checkbox that is both
  `checked` and `disabled` (it carries no value binding from `$config`). Because
  a disabled checkbox is never submitted, every alerts save writes
  `send_sys_errors = 0` regardless of the seeded default — see KL-032.3 for the
  consequence. `send_sql_errors` (default `1`) and `send_login_errors`
  (default `1`) are editable checkboxes persisted by presence (0/1).
  (`send_mailparse_errors`, default `1`, is stored at install but has no control
  on this tab and is not touched by the save.)
- The six active flags (`*_alert_active`) are radio Enable/Disable values written
  through verbatim from the POST (raw `1`/`0`), not by presence; only the
  recipient sub-flags are presence-based 0/1.
- For each of the six events, if the event's active flag is enabled, at least one
  of its recipient checkboxes must be selected, else a per-event error keyed on
  the event's `*_alert_active` field is raised ("Select recipient(s)") and the
  save aborts.
- **Transfer-alert error display defect**: the transfer recipient error is stored
  under the key `transfer_alert_active`, but the transfer-status row renders
  `$errors['alert_alert_active']` (a typo). As a result a "Select recipient(s)"
  error for the Ticket Transfer event is computed and blocks the save but is never
  shown next to its control (see KL-032.9).

### FS-032.7: Settings Persistence Semantics

**Description**: The system shall persist settings through a namespaced
key/value store that resolves reads in a fixed precedence order and supports
ephemeral session overrides.

**Acceptance Criteria**:
- Each setting belongs to a namespace; the core help-desk settings use namespace
  `core`. (SLA plans use namespace `sla.{id}` — see FS-032.10.)
- Reads (`get(key, default)`/`getInfo()`) resolve per the runtime config object —
  read-precedence cascade and the pre-namespace legacy single-row fallback are
  owned by **FS-001.5 (BS-016)**. This spec owns the `set()`/`persist()` write
  semantics below.
- `set(key, value)` writes through to the database (insert if absent, update if
  present); `persist(key, value)` sets a session-only value that is not written
  to the database.
- Declared in-code defaults exist for `allow_pw_reset` (`true`) and
  `pw_reset_window` (`30`); all other defaults originate from the install seed
  (see Data Requirements).

### FS-032.8: Ticket Priority Reference Set

**Description**: The system shall provide a fixed reference set of ticket
priorities, consumed by the Default Priority control and by ticket creation,
each with an urgency rank, display color, and a public/internal flag.

**Acceptance Criteria**:
- The seed priority set (4 rows: Low/Normal/High/Emergency with tag, color,
  urgency rank, public flag) is canonical in **FS-091.5**; this spec consumes it.
- Lower urgency rank = higher urgency (1 = Emergency is most urgent).
- A priority exposes: tag, description, color, urgency, and `isPublic()`.
- The priority list helper returns id → description pairs; a public-only variant
  filters to `ispublic=1` (used where end users may pick a priority).
- The Default Priority control on the Ticket Settings tab lists every priority
  by description; the system default is id `2` (Normal).
- There is no admin CRUD screen for priorities in this version; the set is
  seeded and read-only through the settings UI (see Known Limitations).

### FS-032.9: SLA Plan Listing & Mass Actions

**Description**: The system shall list SLA plans with sorting and pagination and
support enable/disable/delete on a selected set.

**Acceptance Criteria**:
- The list (`slas.php` → `slaplans.inc.php`) shows columns: selection checkbox,
  Name, Status (Active/Disabled), Grace Period (hrs), Date Added, Last Updated.
- The recognized sort keys are `name`→`sla.name`, `status`→`sla.isactive`,
  `period`→`sla.grace_period`, `date`→`sla.created`, `updated`→`sla.updated`;
  default sort is name ascending. Invalid sort/order values fall back to
  name/ASC. **Sort-link defect**: the "Date Added" column header links to
  `sort=created` (not the recognized `date` key), so clicking it silently falls
  back to name-ascending and the column is effectively un-sortable; likewise the
  active-sort CSS marker variables `$created_sort`/`$updated_sort` referenced by
  those headers are never assigned (see KL-032.10).
- The list paginates using the configured page limit.
- Mass actions (`do=mass_process`) operate on the `ids[]` selection; at least
  one plan must be selected, else "You must select at least one plan."
  - **Enable**: sets `isactive=1` on the selection; reports how many of the
    selected plans were enabled.
  - **Disable**: sets `isactive=0`; reports the count.
  - **Delete**: deletes each selected plan that is deletable (see FS-032.12);
    reports full success, partial ("N of M deleted"), or failure.
- An "Add New SLA Plan" link routes to the add form (`a=add`).
- The list partial gate checks `$thisstaff->isAdmin()` only (it does not
  short-circuit on a null `$thisstaff` the way the form partial does, which adds
  the `!$thisstaff` check); both still die "Access Denied" for non-admins behind
  the entry-script gate.

### FS-032.10: SLA Plan Create / Edit Form

**Description**: The system shall provide a create/edit form for an SLA plan
capturing name, grace period, status, escalation, transient behavior, overdue
alert suppression, and internal notes.

**Acceptance Criteria** — the form (`slaplan.inc.php`) captures:
- **Name** (`name`, text, required): unique across SLA plans.
- **Grace Period** (`grace_period`, text, required): numeric hours.
- **Status** (`isactive`, radio): Active (`1`) / Disabled (`0`). On the add form
  the default is Active.
- **Priority Escalation** (`enable_priority_escalation`, checkbox): escalate
  priority on overdue tickets. On the add form the default is enabled. Stored on
  the plan row (default `1`).
- **Transient** (`transient`, checkbox): the plan may be overridden on ticket
  transfer or help-topic change. Stored in the plan's own namespace
  (`sla.{id}`), not on the plan row.
- **Ticket Overdue Alerts** (`disable_overdue_alerts`, checkbox): when checked,
  overdue alert notices are suppressed for this plan, overriding the global
  alert setting. Stored on the plan row (default `0`).
- **Admin Notes** (`notes`, textarea): internal notes.
- Create routes through `do=add`; edit through `do=update` with an `id`. A
  successful create reports "SLA plan added successfully"; a successful update
  reports "SLA plan updated successfully".
- On create, after the row is inserted the `transient` flag is written into the
  new plan's namespaced config; on update, the `transient` flag is rewritten.

### FS-032.11: SLA Overdue Computation & Behavioral Hooks

**Description**: The system shall derive a ticket's SLA-driven due date from the
plan's grace period and expose plan-level flags that govern overdue alerting and
escalation.

**Acceptance Criteria**:
- A ticket's SLA due date is computed as the ticket creation time plus the plan's
  grace period in hours (null when the ticket has no SLA). When a ticket has no
  explicit manual due date, the SLA due date is its effective due date.
- A ticket becomes overdue when its effective due date is in the past; clearing
  or changing the due date / SLA recomputes the overdue flag.
- **Reopened tickets measure the grace period from the reopen time, not from
  creation.** The periodic overdue sweep (consumed by FS-021/FS-043) selects
  tickets with no manual due date where, for a never-reopened ticket, the elapsed
  time since `created` ≥ `grace_period × 3600` seconds, and for a reopened ticket
  the elapsed time since the `reopened` timestamp ≥ `grace_period × 3600`. The
  derived `sla_duedate` column (always `created + grace_period`) and the live
  per-ticket overdue check are therefore distinct from the reopened-aware sweep.
- `alertOnOverdue()` returns true only when the plan does NOT disable overdue
  alerts (i.e. `disable_overdue_alerts = 0`); this is how a plan overrides the
  global overdue-alert setting.
- `priorityEscalation()` reflects the plan's `enable_priority_escalation` flag.
- `isTransient()` reflects the plan's namespaced `transient` flag.
- `isActive()` reflects `isactive`.

> **Note**: The *delivery* of overdue alerts and the escalation action itself are
> owned by the ticket/email pipeline specs; this spec owns only the plan flags
> and the grace-period→due-date derivation that drive them.

### FS-032.12: SLA Plan Deletion Constraints & Reassignment

**Description**: When an SLA plan is deleted, the system shall refuse to delete
the configured default plan and shall re-home all references to the plan.

**Acceptance Criteria**:
- Deletion is refused (returns failure) when the plan is the configured system
  default SLA, or when no global config is available.
- On successful deletion the system:
  - clears the SLA on any departments referencing it (`sla_id = 0`);
  - clears the SLA on any help topics referencing it (`sla_id = 0`);
  - reassigns any tickets referencing it to the system default SLA id.

### FS-032.13: FAQ Category Listing & Mass Actions

**Description**: The system shall list FAQ categories with sorting and
pagination and support make-public / make-internal / delete on a selection.

**Acceptance Criteria**:
- The list (`categories.php` → `categories.inc.php`) shows columns: selection
  checkbox, Name, Type (Public/Internal), FAQs (count, linking to the FAQ list
  filtered by category), Last Updated.
- Sortable columns map to: name, type (`ispublic`), faqs (count), updated;
  default sort is name ascending.
- The list paginates using the configured page limit.
- Mass actions (`do=mass_process`) operate on `ids[]`; at least one category must
  be selected, else "You must select at least one category".
  - **Make Public**: sets `ispublic=1`; reports full/partial count.
  - **Make Internal**: sets `ispublic=0`; reports full/partial count.
  - **Delete**: deletes each selected category and its associated FAQs (see
    FS-032.16); reports full/partial/failure.
- An "Add New Category" link routes to the add form (`a=add`).

### FS-032.14: FAQ Category Create / Edit Form

**Description**: The system shall provide a create/edit form for an FAQ category
capturing its public/internal type, name, description, and internal notes.

**Acceptance Criteria** — the form (`category.inc.php`) captures:
- **Category Type** (`ispublic`, radio, required): Public (`1`, publish) /
  Private-internal (`0`).
- **Category Name** (`name`, text, required): trimmed and stripped of tags;
  minimum 3 characters; unique across categories.
- **Category Description** (`description`, rich textarea, required): stored after
  HTML-safety filtering.
- **Internal Notes** (`notes`, textarea): optional.
- Create routes through `do=create` (reports "Category added successfully");
  edit through `do=update` with an `id` (reports "Category updated
  successfully").
- A separate validate path performs the same field validation without writing
  (used for pre-flight checks).
- The category name is HTML-tag-stripped and trimmed *before* validation, so a
  name consisting only of tags/whitespace is treated as empty.
- The category description is HTML-safety-filtered on save (not on the
  validate-only path).

> **Note on lookup robustness**: `Category::lookup($id)` returns a `Category`
> object whenever `$id` is a non-zero numeric — it does NOT verify that a row
> actually loaded (unlike `SLA::lookup`/`Priority::lookup`, which re-check that
> the loaded id equals the requested id). A numeric-but-nonexistent category id
> therefore yields a hollow object rather than the "Unknown or invalid category
> ID." error, which only fires for non-numeric or zero ids (see KL-032.11).

### FS-032.15: FAQ Category Access Control

**Description**: The system shall gate FAQ category management behind the
FAQ-management permission rather than the admin flag.

**Acceptance Criteria**:
- `categories.php` redirects to the knowledge-base entry (`kb.php`) unless the
  acting staff has the manage-FAQ permission (`canManageFAQ()`).
- The category form partial dies "Access Denied" unless the staff-context
  constant is defined AND the staff has `canManageFAQ()`.
- The category list partial dies "Access Denied" unless the staff-context
  constant is defined AND a staff is present.

> **Note**: This differs from the settings and SLA screens, which require the
> stricter `isAdmin()` admin flag. The manage-FAQ permission is owned by the
> staff/group spec (FS-031).

### FS-032.16: FAQ Category Deletion Cascade

**Description**: When an FAQ category is deleted, the system shall also delete
the FAQ articles belonging to it.

**Acceptance Criteria**:
- Deleting a category removes the category row and then deletes every FAQ whose
  `category_id` matches the deleted category.
- The delete-confirmation dialog warns that deleted categories cannot be
  recovered, including associated FAQs.

> **Note**: The FAQ article model and KB rendering are owned by FS-050; this spec
> owns the category lifecycle and its delete cascade only.

---

## Business Rules

### BS-032.1: Settings Are Administrator-Only

**Rule**: The seven-tab settings panel and the SLA plan screens are accessible
only to staff whose admin flag (`isAdmin()`) is set. The entry scripts
(`settings.php`, `slas.php`) gate through the admin bootstrap (`admin.inc.php`)
before any partial is included; five of the seven settings partials (`system`,
`tickets`, `emails`, `pages`, `kb`) plus both SLA partials re-assert `isAdmin()`
independently as a defense-in-depth check. The `alerts` and `autoresp` partials
omit the in-partial re-assertion and depend on the entry-script gate alone
(KL-032.8).

**Rationale**: System-wide configuration and SLA policy affect every ticket and
user; only administrators may change them.

**Examples**:
- A non-admin staff member who reaches a settings partial sees "Access Denied".
- FAQ category management is the exception — it requires the manage-FAQ
  permission instead (BS-032.12).

### BS-032.2: Invalid Save Re-Displays Submitted Values

**Rule**: When a settings POST fails validation, the form re-renders from the
submitted input, not from the stored configuration, so the administrator sees
and can correct exactly what they entered.

**Rationale**: Re-loading stored values on error would discard the
administrator's partially-correct edits.

**Examples**:
- An admin blanks the Helpdesk URL and saves; the form returns showing the rest
  of their edits intact with a "Helpdesk URl required" error by the URL field.

### BS-032.3: No-Op Writes Are Skipped; Timestamps Track Real Changes

**Rule**: A configuration value is written to the store only when it differs from
the currently persisted value; a real change refreshes that key's `updated`
timestamp.

**Rationale**: Avoids unnecessary writes and keeps the last-modified timestamp
meaningful.

### BS-032.4: Checkbox Settings Persist as 0/1 by Presence

**Rule**: Checkbox-style settings are stored as `1` when present in the POST and
`0` when absent; radio Enable/Disable settings store the submitted `1`/`0`.

**Rationale**: Unchecked HTML checkboxes are not submitted, so presence is the
signal; this is applied uniformly across the system, tickets, emails,
autoresponder, KB, and alerts tabs.

### BS-032.5: An Alert Event Requires at Least One Recipient

**Rule**: For each of the six ticket alert events (new ticket, new message, new
note, assignment, transfer, overdue), enabling the event's active flag requires
selecting at least one recipient; otherwise the save is rejected with
"Select recipient(s)".

**Rationale**: An enabled alert with no recipients would silently send nothing —
a configuration error worth surfacing.

**Examples**:
- Enabling "New Ticket Alert" with none of Admin / Dept Manager / Dept Members
  checked is rejected.

### BS-032.6: Attachment Validation Is Conditional on the Master Switch

**Rule**: File-attachment field validation (numeric size, non-empty file-type
list, upload-count caps within platform bounds, platform `file_uploads`
directive enabled) is enforced only when "Allow Attachments" is checked.

**Rationale**: When attachments are globally disabled, the dependent fields are
irrelevant and need not be valid.

### BS-032.7: Admin Email Must Not Be a System Email

**Rule**: The administrator's contact email may not match the address of any
configured system email account.

**Rationale**: Reusing a system inbound address as the admin alert address would
create mail loops and ambiguous routing.

### BS-032.8: Strip-Quoted-Reply Requires a Separator

**Rule**: Enabling "Strip Quoted Reply" requires a non-empty reply separator tag.

**Rationale**: The stripper keys on the separator; without one it cannot find the
quoted boundary.

### BS-032.9: SLA Name and Grace Period Are Required and the Name Is Unique

**Rule**: An SLA plan requires a name and a numeric grace period (in hours); the
name must be unique across plans.

**Rationale**: The grace period drives overdue computation; a duplicate name
would make plan selection ambiguous.

### BS-032.10: A Ticket Is Overdue on Grace-Period Violation

**Rule**: A ticket's SLA due date is its creation time plus the plan's grace
period in hours; when the effective due date has passed, the ticket is overdue.

**Rationale**: This is the core SLA semantic — the grace period is the allowed
response window measured from creation.

**Examples**:
- A plan with a 48-hour grace period applied to a ticket created Monday 09:00
  makes that ticket overdue after Wednesday 09:00 (absent a manual due date).

> **Reopen exception**: for a ticket that has been reopened, the grace-period
> clock in the periodic overdue sweep restarts from the reopen timestamp rather
> than the original creation time, so the response window is measured from the
> reopen. (The persisted `sla_duedate` derived column still reflects
> `created + grace_period`; the reopened-aware measurement is applied by the
> sweep query owned by FS-021/FS-043.)

### BS-032.11: A Plan May Override Global Overdue Alerts and Be Transient

**Rule**: A plan with "Disable overdue alerts" suppresses overdue notices for its
tickets regardless of the global overdue-alert setting; a "Transient" plan may be
replaced when a ticket is transferred or its help topic changes.

**Rationale**: Lets specific SLAs opt out of noisy overdue alerts and lets
default/help-topic-driven SLAs yield to a department or topic SLA on routing.

### BS-032.12: The Default SLA Cannot Be Deleted; Deletion Re-homes References

**Rule**: The configured default SLA plan cannot be deleted; deleting any other
plan clears it from departments and help topics and reassigns its tickets to the
default SLA.

**Rationale**: The default SLA is a required fallback; orphaning tickets or
routing rows to a missing SLA would break overdue computation.

### BS-032.13: Priorities Are a Fixed, Urgency-Ranked Set

**Rule**: The four priorities (Low, Normal, High, Emergency) carry urgency ranks
4/3/2/1 (lower = more urgent); Emergency is internal-only (not public), the other
three are public.

**Rationale**: Urgency rank orders queues and escalation; the public flag
controls whether end users may select the priority.

### BS-032.14: FAQ Category Name Is Trimmed, ≥3 Chars, and Unique

**Rule**: An FAQ category requires a name (trimmed, tag-stripped, minimum 3
characters, unique across categories) and a description; the public flag
determines whether the category is eligible for publication.

**Rationale**: Prevents trivial/duplicate categories and ensures every category
is describable to readers.

### BS-032.15: A Public Category Publishes Only With Published Articles

**Rule**: A category marked Public is published to the client interface only if
it contains published FAQ articles.

**Rationale**: Empty public categories should not appear to end users.

> **Note**: The publication rendering rule is consumed/owned by FS-050; it is
> recorded here because the public/internal flag is set on this screen.

### BS-032.16: Deleting a Category Deletes Its FAQs

**Rule**: Deleting an FAQ category cascades to delete all FAQ articles in that
category; the deletion is irreversible.

**Rationale**: Articles cannot exist without a parent category in this model.

### BS-032.17: Client Logo Must Be a Wide (Sub-3:1) GIF/JPEG/PNG

**Rule**: An uploaded client logo is accepted only when it is a GIF, JPEG, or PNG
image whose width-to-height aspect ratio is **strictly less than 3:1**; a ratio of
3:1 or wider is rejected as "too square" (the threshold is the literal
`$aspect_ratio = 3` default), and any other image type is rejected as an invalid
type. The validation is performed only when the GD image extension is available;
without GD the image is stored as a logo without dimension/type validation.

**Rationale**: The client-portal logo region is a wide banner; a near-square image
renders poorly. The 3:1 floor enforces a landscape shape. Skipping validation when
GD is absent keeps logo upload functional on hosts lacking the image extension,
trading the shape guarantee for availability.

**Examples**:
- A 600×120 (5:1) PNG is accepted.
- A 300×120 (2.5:1) JPEG is rejected: "Image is too square. Upload a wider image".
- A 600×120 BMP is rejected: "Invalid image file type".
- On a host without GD, a 300×300 PNG is stored unvalidated as a logo.

---

## Data Requirements

### Core Configuration Keys & Install Defaults (namespace `core`)

The settings tabs read and write key/value pairs in the `core` namespace. The
table schema and the full key list are canonical in FS-091; the seeded defaults
relevant to this spec are:

| Key | Default | Owning tab |
|-----|---------|-----------|
| `isonline` | `0` | system |
| `helpdesk_title` | `osTicket Support Ticket System` | system |
| `helpdesk_url` | (empty) | system |
| `default_dept_id` | `0` | system |
| `default_template_id` | `1` | system |
| `max_page_size` | `25` | system |
| `log_level` | `2` | system |
| `log_graceperiod` | `12` | system |
| `passwd_reset_period` | `0` | system |
| `allow_pw_reset` | `true` (code default) | system |
| `pw_reset_window` | `30` (code default) | system |
| `staff_max_logins` | `4` | system |
| `staff_login_timeout` | `2` | system |
| `staff_session_timeout` | `30` | system |
| `staff_ip_binding` | `0` | system |
| `client_max_logins` | `4` | system |
| `client_login_timeout` | `2` | system |
| `client_session_timeout` | `30` | system |
| `time_format` | ` h:i A` | system |
| `date_format` | `m/d/Y` | system |
| `datetime_format` | `m/d/Y g:i a` | system |
| `daydatetime_format` | `D, M j Y g:ia` | system |
| `default_timezone_id` | `0` | system |
| `enable_daylight_saving` | `0` | system |
| `random_ticket_ids` | `1` | tickets |
| `default_sla_id` | `0` | tickets |
| `default_priority_id` | `2` | tickets |
| `max_open_tickets` | `0` | tickets |
| `autolock_minutes` | `3` | tickets |
| `allow_priority_change` | `0` | tickets |
| `use_email_priority` | `0` | tickets |
| `show_related_tickets` | `1` | tickets |
| `show_notes_inline` | `1` | tickets |
| `clickable_urls` | `1` | tickets |
| `enable_captcha` | `0` | tickets |
| `auto_assign_reopened_tickets` | `1` | tickets |
| `show_assigned_tickets` | `1` | tickets |
| `show_answered_tickets` | `0` | tickets |
| `log_ticket_activity` | `1` | tickets |
| `hide_staff_name` | `0` | tickets |
| `allow_attachments` | `0` | tickets |
| `allow_email_attachments` | `0` | tickets |
| `allow_online_attachments` | `0` | tickets |
| `allow_online_attachments_onlogin` | `0` | tickets |
| `max_user_file_uploads` | (empty) | tickets |
| `max_staff_file_uploads` | (empty) | tickets |
| `max_file_size` | `1048576` | tickets |
| `email_attachments` | `1` | tickets |
| `allowed_filetypes` | `.doc, .pdf` | tickets |
| `default_email_id` | `0` | emails |
| `alert_email_id` | `0` | emails |
| `admin_email` | (empty) | emails |
| `enable_mail_polling` | `0` | emails |
| `enable_auto_cron` | `0` | emails |
| `strip_quoted_reply` | `1` | emails |
| `reply_separator` | `-- do not edit --` | emails |
| `default_smtp_id` | `0` | emails |
| `landing_page_id`, `offline_page_id`, `thank-you_page_id`, `client_logo_id` | (set via pages tab) | pages |
| `ticket_autoresponder` | `0` | autoresp |
| `ticket_notice_active` | `0` | autoresp |
| `message_autoresponder` | `0` | autoresp |
| `overlimit_notice_active` | `0` | autoresp |
| `enable_kb` | `0` | kb |
| `enable_premade` | `1` | kb |
| (alert matrix keys — see FS-032.6 table for the 24 flags + `send_sys_errors`=1, `send_sql_errors`=1, `send_login_errors`=1, `send_mailparse_errors`=1) | | alerts |
| `overdue_grace_period` | `0` | (read-only; legacy global) |

### SLA Plan (`sla` row + `sla.{id}` namespace)

| Attribute | Type | Default | Notes |
|-----------|------|---------|-------|
| `id` | int | auto | |
| `name` | varchar(64), unique | (required) | |
| `grace_period` | int hours | `0` (form requires a value) | |
| `isactive` | 0/1 | `1` | |
| `enable_priority_escalation` | 0/1 | `1` | |
| `disable_overdue_alerts` | 0/1 | `0` | |
| `notes` | text | null | internal |
| `created`, `updated` | datetime | now | |
| `transient` | 0/1 (in namespace `sla.{id}`) | `0` | overridable on transfer/topic change |

Seed plan ("Default SLA" — active, escalation on, overdue alerts on, grace
period 48 hours) is canonical in **FS-091.6**. This spec's own overdue-computation
semantics live in FS-032.11.

### Ticket Priority (`ticket_priority`)

Fixed seed set of four rows — see FS-032.8 table (tag, description, color,
urgency, public). The full schema is canonical in FS-091.

### FAQ Category (`faq_category`)

| Attribute | Type | Notes |
|-----------|------|-------|
| `category_id` | int | auto |
| `name` | varchar, unique | trimmed, ≥3 chars |
| `ispublic` | 0/1 | Public/Internal |
| `description` | text | HTML-safety filtered |
| `notes` | text | internal |
| `created`, `updated` | datetime | |
| (derived) `faqs` | count | LEFT JOIN count of FAQ articles |

### Settings Tab Label Map

`system`→"System Settings", `tickets`→"Ticket Settings and Options",
`emails`→"Email Settings", `pages`→"Site Pages", `kb`→"Knowledgebase Settings",
`autoresp`→"Autoresponder Settings", `alerts`→"Alerts and Notices Settings".

---

## User Flows / Interactions

### Flow 1: Change a System Default
1. Admin opens Settings (defaults to the System tab).
2. Admin edits the Helpdesk Title and Default Department, then saves.
3. The system validates required fields, writes only changed keys, and reports
   "System Settings Updated Successfully".

### Flow 2: Configure the Alert Matrix
1. Admin opens the Alerts and Notices tab.
2. Admin enables "New Ticket Alert" but leaves all three recipients unchecked.
3. On save the system rejects with "Select recipient(s)" by the New-Ticket event
   and re-displays the submitted state.
4. Admin checks "Admin Email" and re-saves; the matrix persists.

### Flow 3: Add an SLA Plan
1. Admin clicks "Add New SLA Plan".
2. Admin enters a unique name, a grace period of 24 (hours), leaves Status Active
   and Escalation on, optionally marks Transient.
3. On save the plan row is inserted and the transient flag is written to the
   plan's namespace; the system reports "SLA plan added successfully".

### Flow 4: Delete SLA Plans (with default protection)
1. Admin selects several plans on the list, including the default SLA, and clicks
   Delete.
2. Non-default plans are deleted (departments/topics cleared, tickets reassigned
   to the default SLA); the default SLA is skipped.
3. The system reports a partial result ("N of M deleted") because the default was
   refused.

### Flow 5: Make FAQ Categories Public
1. A staff member with manage-FAQ permission opens FAQ Categories.
2. They select two internal categories and click "Make Public".
3. The categories flip to public; each will publish to clients only once it has
   published articles.

### Flow 6: Choose a Custom Client Logo
1. Admin opens the Site Pages tab and uploads a new logo file, or selects a
   previously uploaded one.
2. Admin checks Delete on an unused logo and saves.
3. A confirmation dialog appears (because a delete checkbox is set); on confirm,
   the selected logo is set and the unused logo file is removed.

---

## Edge Cases & Error Scenarios

### EC-032.1: Unknown Settings Tab
**Scenario**: A settings request carries an unrecognized `t`.
**Expected**: The active tab resolves to `system`; a POST with an unknown `t`
yields "Unknown setting option. Get technical support." and saves nothing.

### EC-032.2: Date-Format Live Preview With No Timezone
**Scenario**: Admin enters date/time format strings before selecting a timezone.
**Expected**: The preview renders against the stored offset (0 when unset) and
the DST flag; the format strings themselves are still required to save.

### EC-032.3: Attachment Caps Exceed Platform Limit
**Scenario**: With attachments enabled, the admin sets a user/staff upload count
above the platform `max_file_uploads` bound.
**Expected**: A field error "Invalid selection. Must be less than {bound}"
blocks the save.

### EC-032.4: CAPTCHA Without GD
**Scenario**: Admin enables Human Verification on a platform lacking GD / PNG
support.
**Expected**: A field error ("The GD extension required" or
"PNG support required for Image Captcha") blocks the save.

### EC-032.5: Duplicate SLA / Category Name
**Scenario**: Admin enters a name already used by another SLA plan or FAQ
category.
**Expected**: "Name already exists" / "Category already exists" — the save is
rejected.

### EC-032.6: Too-Short Category Name
**Scenario**: Admin enters a 1–2 character category name.
**Expected**: "Name is too short. 3 chars minimum".

### EC-032.7: Deleting the Default SLA
**Scenario**: The selected delete set is exactly the default SLA.
**Expected**: Nothing is deleted; the action reports failure ("Unable to delete
selected SLA plans").

### EC-032.8: Editing a Plan/Category by an Invalid Id
**Scenario**: The request carries an `id` that does not resolve.
**Expected**: For SLA — a non-numeric/zero id (or one that fails the
`getId()==$id` re-check) sets the entry-script error, whose text reads "Unknown
or invalid API key ID." (a copy artifact; see KL-032.4). For Category — only a
non-numeric/zero id raises "Unknown or invalid category ID."; a numeric but
non-existent id slips past `Category::lookup` (which does not re-check the loaded
row) and produces a hollow object rather than the error (see KL-032.11).

### EC-032.9: Mass Action With Empty Selection
**Scenario**: A mass enable/disable/delete/make-public is submitted with no
`ids[]`.
**Expected**: "You must select at least one plan." / "You must select at least
one category".

### EC-032.10: Partial Mass-Delete Result
**Scenario**: Some selected items delete and others fail (e.g. default SLA among
them).
**Expected**: "N of M selected … deleted" is reported rather than full success.

### EC-032.11: Admin Email Equals a System Email
**Scenario**: Admin sets the admin email to an address already used as a system
inbound email.
**Expected**: "Email already setup as system email" blocks the save.

### EC-032.12: Logo Save With No Checkboxes
**Scenario**: Admin saves the Site Pages tab without checking any delete
checkbox.
**Expected**: The form submits directly. The confirm dialog intercepts whenever
*any* checkbox in the form is checked (the JS predicate is "any checked
checkbox", not specifically a delete checkbox); on the Site Pages tab the only
checkboxes are the `delete-logo[]` ones, so the effect is delete-only, but the
predicate is broader than "a delete checkbox is set".

### EC-032.13: Uploaded Logo Without Selecting It
**Scenario**: Admin uploads a new logo file but leaves the radio on a different
(or the system-default `0`) logo.
**Expected**: The upload is stored as a system logo file, but `client_logo_id`
persists the radio's `selected-logo` value (system default stores `false`); the
newly uploaded file is selectable on the next render. The upload and the
selection are independent actions on the same save.

### EC-032.14: Selecting the System Default Logo
**Scenario**: Admin chooses the system-default logo radio (value `0`) and saves.
**Expected**: `client_logo_id` is written as `false` (non-numeric/zero
`selected-logo` collapses to `false`), so the client interface falls back to the
bundled default logo image.

### EC-032.15: Logo Upload Without the GD Extension
**Scenario**: Admin uploads a client logo on a host where the GD image extension
is not loaded.
**Expected**: `AttachmentFile::uploadLogo` skips the dimension/aspect-ratio/type
validation entirely and stores the file as a logo (`ft='L'`) — so a near-square or
non-image-typed file that would be rejected on a GD-enabled host is accepted here.
The 3:1 aspect-ratio and GIF/JPEG/PNG guarantees (BS-032.17) hold only when GD is
present (KL-032.13).

---

## Dependencies

| Dependency | Specification | Relationship |
|------------|---------------|--------------|
| Request bootstrap, `$cfg`/`OsticketConfig` instantiation, admin shell, nav | FS-001 | Provides the `$cfg` singleton and admin page chrome; this spec owns the write path |
| Staff auth, `isAdmin()`, `canManageFAQ()` permission | FS-002 / FS-031 | Gates the settings/SLA screens (admin) and the category screens (manage-FAQ) |
| Validation & formatting helpers (`Validator::process`, `Format::*`) | FS-003 | Field validation, HTML-escaping, date preview, file-size formatting |
| Departments / help topics (default dept, SLA re-homing) | FS-030 | Default-department options; SLA delete clears `sla_id` on these |
| Email accounts & templates (default system/alert/SMTP email, default template) | FS-040 | Option sources for the system/emails tabs; admin-email uniqueness check |
| Email/alert *delivery*, autoresponder *sending*, overdue alert dispatch | FS-040 / FS-041 / FS-021 | Consumes the flags this spec persists |
| Ticket overdue/escalation behavior, due-date computation | FS-021 | Consumes SLA grace period, escalation, transient, alert flags |
| Site pages & logo file storage | FS-033 / FS-022 | Page-id options and logo attachment files for the pages tab |
| Knowledge base / FAQ articles, public publication rendering | FS-050 | Consumes `enable_kb`, the public flag, and the category delete cascade |
| Pagination | FS-090 | SLA and category list paging |
| Config table, priority/SLA/category schemas & enums | FS-091 | Canonical table/column/enum definitions referenced here |

---

## Known Limitations

### KL-032.1: No Admin UI for Ticket Priorities
**Limitation**: Priorities are a fixed four-row seed set with no create/edit/
delete screen; only the Default Priority selector reads them. Changing colors,
tags, urgency, or adding priorities requires direct data manipulation.

### KL-032.2: Two Distinct Permission Gates Across This Spec
**Limitation**: Settings and SLA screens require the admin flag (`isAdmin()`),
while FAQ category management requires the manage-FAQ permission
(`canManageFAQ()`). A manage-FAQ staff member who is not an admin can manage
categories but not settings — an intentional but easily-confused split.

### KL-032.3: `send_sys_errors` Is Rendered "Always On" but Stored as 0
**Limitation**: The `send_sys_errors` checkbox on the Alerts tab is rendered
`checked` and `disabled` to imply system-error alerts are always on. Because a
disabled checkbox is never submitted, every alerts-tab save actually persists
`send_sys_errors = 0`. The discrepancy is harmless in practice: no consumer reads
`send_sys_errors` (there is no `alertONSysError()` accessor), so the stored value
is effectively dead and system-error alerting is governed elsewhere. The UI's
"always on, can't disable" presentation and the stored `0` therefore disagree but
neither has a behavioral effect through this config key.

### KL-032.4: Copy-Paste Error Text on the SLA Entry Script
**Limitation**: When an SLA `id` fails to resolve, the entry script sets the
error "Unknown or invalid API key ID." (text copied from the API-key screen)
rather than an SLA-specific message. Cosmetic; the create/update flows use the
correct "Unknown or invalid SLA plan." message.

### KL-032.5: `overdue_grace_period` Is a Legacy Global With No UI
**Limitation**: A global `overdue_grace_period` config key (default `0`) exists
and is read by `getGracePeriod()`, but there is no control for it on any settings
tab — per-plan SLA grace periods superseded it. It is effectively dead unless set
directly.

### KL-032.6: Autoresponder & Some Email/KB Saves Skip Field Validation
**Limitation**: The autoresponder tab (and the simpler KB toggles) write their
flags with no field validation beyond the presence checks; malformed values
would be coerced rather than rejected. Low risk given the controls are
radios/checkboxes.

### KL-032.7: Session-Override Settings Are Ephemeral
**Limitation**: `persist()` writes a session-only override that shadows the
database value for the current session without saving; some derived values (e.g.
timezone offset) are session-cached this way and are not visible as stored
config. This is by design for derived/compatibility values but means not every
effective setting is in the `config` table.

### KL-032.8: Two Settings Partials Lack the In-Partial Admin Re-Check
**Limitation**: The `alerts` and `autoresp` settings partials do not repeat the
`isAdmin()` "Access Denied" guard that the other five partials carry; they rely
solely on the entry-script (`admin.inc.php`) gate. This is safe given the only
inclusion path is through that gated entry script, but it is an inconsistent
defense-in-depth posture across the seven tabs.

### KL-032.9: Transfer-Alert Recipient Error Is Never Displayed
**Limitation**: When the Ticket Transfer alert is enabled with no recipients, the
save is correctly blocked with a "Select recipient(s)" error keyed on
`transfer_alert_active`, but the transfer-status row renders the misspelled key
`$errors['alert_alert_active']`, so the message never appears next to its
control. The administrator sees a blocked save with no visible reason on that
row.

### KL-032.10: SLA "Date Added" Column Is Not Sortable
**Limitation**: The SLA plan list's "Date Added" header links to `sort=created`,
which is not among the recognized sort keys (`date` maps to `sla.created`), so
the link silently falls back to name-ascending. The corresponding active-sort CSS
marker variables (`$created_sort`, `$updated_sort`) referenced by those headers
are also never assigned. Cosmetic / minor usability.

### KL-032.11: `Category::lookup` Does Not Verify the Row Loaded
**Limitation**: Unlike `SLA::lookup` and `Priority::lookup`, `Category::lookup`
returns a `Category` object for any non-zero numeric id without re-checking that
a row was actually fetched. A numeric-but-nonexistent category id thus yields a
hollow object instead of the "Unknown or invalid category ID." error, and a
subsequent update would operate against an internally-zero id. Low risk because
ids are normally produced by the list links.

### KL-032.12: Logo File Input Name/Read Mismatch
**Limitation**: The Site Pages logo file input is named `logo[]` (array form),
while the save reads `$_FILES['logo']` and passes it to `AttachmentFile::format`.
The single-file upload path works in practice, but the array-style field name and
scalar read are inconsistent and would not cleanly support multiple logo files.

### KL-032.13: Logo Validation Is GD-Gated, Not Enforced Without It
**Limitation**: The logo aspect-ratio and image-type checks (BS-032.17) run only
when the GD extension is loaded; `AttachmentFile::uploadLogo` short-circuits to an
unvalidated store when GD is absent (EC-032.15). On a GD-less host an admin can
therefore install a near-square or non-image-typed "logo" that the spec's shape
rule would otherwise reject. The validation is thus a best-effort guard, not a
hard guarantee independent of the runtime.

---

## Future Considerations

- A first-class priority management screen (CRUD, reorder, custom colors).
- Expose `overdue_grace_period` or remove the dead key (KL-032.5).
- Allow disabling system-error alerts, or surface a recipient override
  (KL-032.3).
- Per-SLA business-hours / calendar support (the current grace period is a flat
  hour count from creation).
- Consolidate the admin-vs-manage-FAQ gate or document it in-product (KL-032.2).
- Replace the copied SLA error string (KL-032.4).
- Optional "move FAQs to another category" on category delete instead of the
  hard cascade (the code carries a TODO to this effect).
