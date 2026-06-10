# FS-030: Admin — Departments, Teams & Help Topics

## Overview

This specification defines the three routing-and-organization configuration objects an administrator manages from the staff control panel's admin area: **Departments**, **Teams**, and **Help Topics**. Together they determine how incoming tickets are categorized, routed to a responsible organizational unit, auto-assigned to a staff member or team, prioritized, SLA-bound, and which email identity / template / auto-response governs the conversation.

- **Departments** are the primary organizational unit that owns tickets. A department carries an outbound email identity, an email template group, an SLA plan, a manager, a visibility flag (public vs. private/internal), per-department auto-response overrides, an optional outgoing signature, and a list of staff **Groups** allowed to access its tickets. Every ticket belongs to exactly one department (transfer changes it).
- **Teams** are cross-department assignment groups with a lead, a roster of member staff, an enabled/disabled status, an assignment-alerts override, and admin notes. A help topic or a staff action can assign a ticket to a team rather than an individual.
- **Help Topics** are the public/internal catalog of ticket subjects the requester picks at submission time. Each topic maps to a default department, priority, optional SLA override, optional auto-assignment target (staff or team), optional thank-you page, an auto-response override, public/internal visibility, an enabled/disabled status, an optional parent topic (one level of nesting), and admin notes.

All three are administered through three parallel entry scripts (`departments.php`, `teams.php`, `helptopics.php`) that share an identical structure: a **list view** (sortable columns, checkbox-driven bulk actions) and an **add/edit form view**, dispatched by the same `do` (`update` / `create` / `mass_process`) command pattern. Each create/edit performs server-side validation and reports field-level errors; deletions enforce object-specific safety constraints and perform downstream cleanup (re-homing tickets, members, and dependent topics).

This spec owns the management UI and rules for these three objects. The **runtime consumption** of these objects during ticket creation/routing (FS-011 web submission, FS-021 staff workflow, FS-041 inbound email) and the canonical data-model/enum reference (FS-091) are referenced as dependencies, not restated.

> All access to these admin pages requires an authenticated staff session with the admin flag set (`$thisstaff->isAdmin()`); the constant `OSTADMININC` must be defined (set when the page boots through the admin include). Every view partial begins with `if(!defined('OSTADMININC') || !$thisstaff || !$thisstaff->isAdmin()) die('Access Denied');`. Staff authentication, CSRF, and the admin gate are owned by FS-002.

---

## Functional Requirements

### FS-030.1: Admin access gate & navigation context

**Description**: The system shall restrict all department/team/help-topic management to admin-flagged staff and surface each under its navigation tab.

**Acceptance Criteria**:
- `departments.php`, `teams.php`, and `helptopics.php` each first include the admin bootstrap (`admin.inc.php`), establishing the authenticated admin session, CSRF protection, and the `$cfg` config singleton.
- Each view partial independently re-checks `OSTADMININC` is defined and `$thisstaff->isAdmin()` is true, printing exactly `Access Denied` and halting otherwise. The form partials (`department.inc.php` / `team.inc.php` / `helptopic.inc.php`) and the team list partial use the full guard `if(!defined('OSTADMININC') || !$thisstaff || !$thisstaff->isAdmin())`; the **department** and **help-topic** list partials use the shorter `if(!defined('OSTADMININC') || !$thisstaff->isAdmin())` (no explicit `!$thisstaff` term). In practice the admin bootstrap has already guaranteed an authenticated admin before any partial loads, so the shorter guard is equivalent in the supported flow.
- `departments.php` and `teams.php` set the active navigation tab to `staff`; `helptopics.php` sets it to `manage`.
- Each script renders the staff header, then either the list partial or the single-object form partial, then the staff footer.

### FS-030.2: Page routing — list vs. form

**Description**: The system shall choose between the list view and the single-object add/edit form based on request parameters.

**Acceptance Criteria**:
- For each object, if a valid `id` is supplied (resolving to an existing object) **or** the request carries `a=add`, the single-object **form** partial is rendered (`department.inc.php` / `team.inc.php` / `helptopic.inc.php`).
- Otherwise the **list** partial is rendered (`departments.inc.php` / `teams.inc.php` / `helptopics.inc.php`).
- If `id` is supplied but does not resolve to an existing object, the error `Unknown or invalid <department|team|help topic> ID.` is set and the list view is shown.
- The form is in **edit mode** when a valid object is loaded and `a` is not `add`; otherwise it is in **add mode** (a present-but-add request always wins as add).
- After a successful create, the request's `a` flag is cleared so the user is returned to the list view (with the success message), not the empty add form.

### FS-030.3: Department list view

**Description**: The system shall list all departments with sortable columns, a usage count, and bulk-action controls.

**Acceptance Criteria**:
- Columns: a select checkbox, **Name**, **Type** (Public / **Private**), **Users** (count of staff whose home department is this department), **Email Address** (the department's outbound email identity), **Dept. Manager**.
- The default department is annotated with ` (Default)` after its name, and its select checkbox is rendered disabled (it cannot be bulk-selected).
- The **Users** count, when greater than zero, links to the staff directory filtered by department; the Email cell links to the email-account editor; the Manager cell links to the manager's staff profile.
- Sortable columns and their sort keys: `name` (dept name), `type` (ispublic), `users` (staff count), `email` (email name then address), `manager` (manager name). Default sort is `name` ascending; clicking a column header toggles ASC/DESC.
- A caption reads `Showing 1-N of N departments` (or `No departments found!`).
- An **Add New Department** link routes to `departments.php?a=add`.
- Bulk-action buttons: **Make Public**, **Make Private**, **Delete Dept(s)**, each gated by a JS confirmation dialog (see FS-030.13). There is no pagination on this list (all departments are shown).

### FS-030.4: Department add/edit form

**Description**: The system shall provide a form to create or edit a department with all routing, email, auto-response, access, and signature settings.

**Acceptance Criteria**:
- Fields:
  - **Name** (required text).
  - **Type**: radio Public / Private (Internal). Defaults to Public on add.
  - **Email** (required select): the outbound department email identity, chosen from configured email accounts; option `0` is `— Select Department Email —`.
  - **Template** (required select): the email template group; option `0` is `— System default —`; only active template groups are listed.
  - **SLA** (select): option `0` is `— System default —`; lists all SLA plans.
  - **Manager** (select): option `0` is `— None —`; lists all staff (by last, first name).
  - **Group Membership** (checkbox): "Extend membership to groups with access" — when enabled, alerts/notices extend to all staff in groups granted access (see FS-030.16).
  - **Auto Response Settings** (override block): **New Ticket** checkbox "Disable new ticket auto-response for this Dept."; **New Message** checkbox "Disable new message auto-response for this Dept."; **Auto Response Email** select whose first usable option is `— Department Email (Above) —` (value `0`), then each configured email account.
  - **Department Access**: a checkbox per staff group ("Check all groups allowed to access this department"). Each group shows its member count (linking to the filtered staff list when > 0). An always-shown note states the department manager and primary members always have access independent of group selection or assignment.
  - **Department Signature**: optional free-text area; help text states the signature is offered as a choice on ticket reply for **public** departments only.
- The form posts to `departments.php` with hidden `do` = `create` or `update`, plus `a` and `id`.
- On validation failure, previously entered values are re-displayed (HTML-escaped) and field-level error messages appear inline next to the offending field.
- Edit mode pre-loads the department's stored values, including its currently allowed group IDs (pre-checked).
- Buttons: submit (`Create Dept` / `Save Changes`), **Reset**, **Cancel** (returns to `departments.php`).

### FS-030.5: Department creation & update

**Description**: The system shall persist a new or edited department after validating its fields and synchronizing its allowed-group access list.

**Acceptance Criteria**:
- On `create`: validate (FS-030.14) then insert; on success, set the allowed-group access rows from the submitted `groups[]` and show `<Name> added successfully`.
- On `update`: validate then update; on success, synchronize the allowed-group access rows and reload, showing `Department updated successfully`.
- Updating allowed groups is idempotent: submitted group IDs are inserted (ignoring duplicates) and any previously granted group not in the submitted set is removed; submitting no groups removes all group access rows.
- The stored `name` and `signature` are stripped of HTML tags before saving.
- `created` is stamped on insert; `updated` is stamped on every save.
- If a department's manager is referenced for "group membership" alerting, the manager and primary members always retain access regardless of the group list.
- **No-change update guard**: a department update is considered successful only when the underlying row is actually modified (the save requires a nonzero affected-row count). Because `updated` is always set to the current timestamp, a save normally counts as a modification; however a re-save with no field changes within the same one-second tick can register zero affected rows and is then reported as `Unable to update <Name> Dept. Error occurred` rather than success (see EC-030-15, KL-030-11).

### FS-030.6: Department deletion & re-homing

**Description**: The system shall delete a department only when safe and re-home all dependent records to the default department.

**Acceptance Criteria**:
- A department may be deleted only if **all** are true: the config singleton is available, it is **not** the system default department, and it has **zero** home-department staff (its Users count is 0). Otherwise deletion is refused (returns 0, no rows removed).
- On a successful single delete, the system re-homes dependents to the default department:
  - All tickets whose department is the deleted one are moved to the default department.
  - All staff whose home department is the deleted one are moved to the default department (a safety net; deletion is only allowed at zero staff).
  - All help topics whose default department is the deleted one are repointed to the default department.
  - All group-access rows for the deleted department are removed.
- Deleted departments cannot be recovered.

### FS-030.7: Department bulk actions

**Description**: The system shall support bulk Make Public / Make Private / Delete over a checkbox selection of departments.

**Acceptance Criteria**:
- At least one department must be selected, else `You must select at least one department`.
- If the **default** department is among the selection, the entire batch is refused with `You can not disable/delete a default department. Remove default Dept. and try again.`
- **Make Public**: sets all selected to public. Full success → `Selected departments made public`; partial → `N of M selected departments made public`; total failure → `Unable to make selected department public.`
- **Make Private**: sets selected (excluding the default, which is force-skipped) to private. Messages mirror Make Public (`... made private` / partial / `Unable to make selected department(s) private. Possibly already private!`).
- **Delete**: first counts staff across the whole selection; if any selected department has staff, the **entire** batch is refused with `Departments with staff can not be deleted. Move staff first.` Otherwise each non-default department is deleted (with re-homing per FS-030.6). Full → `Selected departments deleted successfully`; partial → `N of M selected departments deleted`; none → `Unable to delete selected departments.`
- An unknown bulk action yields `Unknown action - get technical help`.

### FS-030.8: Team list view

**Description**: The system shall list all teams with sortable columns, a member count, and bulk-action controls.

**Acceptance Criteria**:
- Columns: select checkbox, **Team Name**, **Status** (Active / **Disabled**), **Members** (count), **Team Lead**, **Created**, **Last Updated**.
- The Members count, when > 0, links to the staff list filtered by team; the lead cell links to the lead's staff profile.
- Sort keys: `name`, `status` (isenabled), `members`, `lead` (lead name), `created`. Default `name` ascending; headers toggle ASC/DESC.
- The **Last Updated** column header renders a sort link (`sort=updated`) but `updated` is **not** a recognized sort key for teams; clicking it falls back to the default `name` sort and does not actually sort by update time (see KL-030-12). (The Created cell shows date only; the Last Updated cell shows date-and-time.)
- Caption reads `Showing 1-N of N teams` (or `No teams found!`).
- An **Add New Team** link routes to `teams.php?a=add`.
- Bulk-action buttons: **Enable**, **Disable**, **Delete**, each gated by a confirmation dialog. No pagination.

### FS-030.9: Team add/edit form

**Description**: The system shall provide a form to create or edit a team, manage its lead, member removal, and notes.

**Acceptance Criteria**:
- Fields:
  - **Name** (required text).
  - **Status**: radio Active / Disabled. Defaults to Active on add. Help text: "Disabled team won't be available for ticket assignment or alerts."
  - **Team Lead** (select): `— None —` (value `0`) plus the team's current members; only existing members can be chosen as lead. Adding members is not done here.
  - **Assignment Alerts** (checkbox): "Disable assignment alerts for this team (override global settings.)".
  - **Team Members** (edit mode only): a list of current members, each with a **Remove** checkbox. Help text states that to **add** members, the admin edits the target member's staff profile (membership is added from the staff side, not here).
  - **Admin Notes**: internal free-text, "viewable by all admins."
- The form posts to `teams.php` with hidden `do`, `a`, `id`.
- On validation failure, submitted values are re-displayed and the name error shows inline.
- Buttons: submit (`Create Team` / `Save Changes`), **Reset**, **Cancel** (returns to `teams.php`).

### FS-030.10: Team creation, update & member removal

**Description**: The system shall persist a team, remove members marked for removal, and protect lead integrity.

**Acceptance Criteria**:
- On `create`: validate (FS-030.15) then insert; on success show `<team> added successfully`.
- On `update`: if the current lead is among the members marked for removal, the lead is reset to none (`lead_id` = 0) before saving. Save the team, then delete the team-member rows for every staff ID in the removal set, then reload. Success → `Team updated successfully`.
- The lead is persisted only on update (a brand-new team has no members yet, so no lead can be chosen at create time).
- `created` stamped on insert; `updated` stamped on every save.
- **No-change update guard**: like a department, a team update returns success only when the underlying row is actually modified (nonzero affected-row count); a no-op re-save within the same one-second tick is reported as `Unable to update the team. Internal error` (see EC-030-15, KL-030-11). The team member-removal step runs only after a successful save.
- Unlike `name` on a department, the team `name` and `notes` are **not** HTML-stripped on save (the raw submitted text is persisted; it is HTML-escaped only at render time).

### FS-030.11: Team deletion & ticket release

**Description**: The system shall delete a team and release any tickets assigned to it.

**Acceptance Criteria**:
- Deletion requires an authenticated staff context and a valid team ID; it removes exactly the one team row (refusing if not exactly one row is affected).
- After the team row is removed: all its team-member rows are deleted, and every ticket whose assigned team is the deleted team has its team assignment cleared (team set to 0 → effectively unassigned-by-team).
- Unlike departments, **teams have no "in use" guard** — a team may be deleted even while it owns staff members and tickets; those associations are released as part of the delete.
- Deleted teams cannot be recovered.

### FS-030.12: Team bulk actions

**Description**: The system shall support bulk Enable / Disable / Delete over a checkbox selection of teams.

**Acceptance Criteria**:
- At least one team must be selected, else `You must select at least one team.`
- **Enable**: sets selected teams enabled. Full → `Selected teams activated`; partial → `N of M selected teams activated`; failure → `Unable to activate selected teams`.
- **Disable**: sets selected teams disabled. Full → `Selected teams disabled`; partial → `N of M selected teams disabled`; failure → `Unable to disable selected teams`.
- **Delete**: deletes each selected team (releasing members and tickets per FS-030.11). Full → `Selected teams deleted successfully`; partial → `N of M selected teams deleted`; none → `Unable to delete selected teams`.
- Unknown action → `Unknown action. Get technical help!`.

### FS-030.13: Help topic list view

**Description**: The system shall list help topics with sortable, paginated columns and bulk-action controls.

**Acceptance Criteria**:
- Columns: select checkbox, **Help Topic** (parent/child shown as `Parent / Child`), **Status** (Active / **Disabled**), **Type** (Public / **Private**), **Priority**, **Department**, **Last Updated**.
- The Department cell links to the department editor.
- Sort keys: `name`, `status` (isactive), `type` (ispublic), `dept` (department name), `priority`, `updated`. Default `name` ascending; headers toggle ASC/DESC.
- The list is **paginated** (page size = the system `PAGE_LIMIT`); a page navigator and `Showing X-Y of Z help topics` caption are shown. (Departments and teams are not paginated.)
- An **Add New Help Topic** link routes to `helptopics.php?a=add`.
- Bulk-action buttons: **Enable**, **Disable**, **Delete**, each gated by a confirmation dialog.

### FS-030.14: Help topic add/edit form

**Description**: The system shall provide a form to create or edit a help topic with routing, priority, SLA, page, auto-assignment, and auto-response options.

**Acceptance Criteria**:
- Fields:
  - **Topic** (required text).
  - **Status**: radio Active / Disabled. Defaults to Active on add.
  - **Type**: radio Public / Private/Internal. Defaults to Public on add.
  - **Parent Topic** (optional select): `— Select Parent Topic —` plus all **top-level** topics (those with no parent). Selecting a parent makes this a child topic; one level of nesting is supported.
  - **New ticket options** block:
    - **Priority** (required select): a leading placeholder `— Select Priority —` (value `""`), then ticket priorities ordered by urgency descending.
    - **Department** (required select): a leading placeholder `— Select Department —` (value `""`), then all departments ordered by name — the default department a ticket on this topic is routed to.
    - **SLA Plan** (select): `— Department's Default —` (value `0`) plus all SLA plans; help text: "(Overrides department's SLA)". (This literal help text is misleading — at runtime the department SLA actually takes precedence over the topic SLA; see BS-030-23 and FS-021.13.)
    - **Thank-you Page** (select): `— System Default —` plus active thank-you pages; help text: "(Overrides global setting. Applies to web tickets only.)".
    - **Auto-assign To** (select): `— Unassigned —` (value `0`), then an optgroup of active staff members (values prefixed `s`), then an optgroup of enabled teams (values prefixed `t`). A single control chooses either a staff member or a team.
    - **Ticket auto-response** (checkbox): "Disable new ticket auto-response for this topic (Overrides Dept. settings)."
  - **Admin Notes**: internal free-text.
- The form posts to `helptopics.php` with hidden `do`, `a`, `id`.
- On validation failure, submitted values re-displayed; field errors inline.
- Buttons: submit (`Add Topic` / `Save Changes`), **Reset**, **Cancel** (returns to `helptopics.php`).

### FS-030.15: Help topic creation & update

**Description**: The system shall persist a help topic after validation, decoding the combined auto-assignment control into staff-or-team.

**Acceptance Criteria**:
- On `create`: validate (FS-030.17) then insert; success → `Help topic added successfully`.
- On `update`: validate then update + reload; success → `Help topic updated successfully`.
- The topic text is trimmed and HTML-stripped before validation and storage.
- The single **Auto-assign To** value is decoded as: a value beginning with `s` → assign to that staff member (team cleared); a value beginning with `t` → assign to that team (staff cleared); anything else → no auto-assignment (both cleared). Only digits are kept from the chosen value when resolving the target ID.
- `created` stamped on insert; `updated` stamped on every save.
- **No no-op guard on topic update**: unlike department and team saves, a help-topic update returns success whenever the query executes — it does **not** require a nonzero affected-row count. Saving a topic with no field changes therefore still reports `Help topic updated successfully` (contrast EC-030-15 / KL-030-11). This is an intentional behavioral difference between the three objects.
- `notes` is persisted as submitted (not HTML-stripped); only the `topic` text is trimmed and HTML-stripped.

### FS-030.16: Help topic deletion & dependent cleanup

**Description**: The system shall delete a help topic and clean up its children, ticket references, and FAQ links.

**Acceptance Criteria**:
- A help topic is deleted with no "in-use" guard. After the topic row is removed:
  - Any child topics whose parent was the deleted topic have their parent cleared (parent set to 0 → promoted to top-level).
  - Every ticket referencing the deleted topic has its topic reference cleared (topic set to 0).
  - All FAQ-to-topic association rows for the deleted topic are removed.
- Deleted topics cannot be recovered.

### FS-030.17: Help topic bulk actions

**Description**: The system shall support bulk Enable / Disable / Delete over a checkbox selection of help topics.

**Acceptance Criteria**:
- At least one topic must be selected, else `You must select at least one help topic`.
- **Enable**: sets selected active. Full → `Selected help topics enabled`; partial → `N of M selected help topics enabled`; failure → `Unable to enable selected help topics.`
- **Disable**: sets selected disabled. Full → `Selected help topics disabled`; partial → `N of M selected help topics disabled`; failure → `Unable to disable selected help topic(s)`.
- **Delete**: deletes each selected topic (with cleanup per FS-030.16). Full → `Selected help topics deleted successfully`; partial → `N of M selected help topics deleted`; none → `Unable to delete selected help topics`.
- Unknown action → `Unknown action - get technical help.`.

---

## Business Rules

### Departments

- **BS-030-01 — Department name required, minimum length, unique.** Name must be present, at least **4** characters, and not collide with another department's name. Errors: `Name required` / `Name is too short.` / `Department already exists`.
- **BS-030-02 — Email identity required.** A department must select an outbound email account (`Email selection required` otherwise).
- **BS-030-03 — Template required.** A department must select a template group value (numeric; `Template selection required` otherwise). Value `0` denotes the system default template.
- **BS-030-04 — Default department cannot be private.** The system default department must remain public; attempting to set it private yields `System default department cannot be private`.
- **BS-030-05 — Default department cannot be deleted or disabled.** The default department is excluded from deletion (single and bulk), is force-excluded from bulk Make-Private, and renders with a disabled checkbox in the list.
- **BS-030-06 — Department deletion requires zero home staff.** A department with staff whose home department points to it cannot be deleted; staff must first be moved. Bulk delete refuses the entire batch if any selected department has staff.
- **BS-030-07 — Deletion re-homes dependents to the default department.** On delete, the department's tickets, home staff, and help-topic default-department pointers are all moved to the default department, and its group-access rows are removed (see FS-030.6).
- **BS-030-08 — Allowed-group access is a full-replace sync.** Saving a department replaces its group-access set with the submitted groups; the manager and primary members always retain access regardless.
- **BS-030-09 — Auto-response overrides.** A department may individually disable new-ticket and new-message auto-responses, overriding global settings; it may also designate a distinct outgoing auto-response email, defaulting to the department email when unset or deleted. The new-ticket / new-message disable checkboxes default to **enabled** (auto-respond on) when omitted from the POST: the save stores the posted value when the field is present, otherwise `1`. The `noreply_autoresp` flag is **read-only** at this management layer — it is exposed by the data model but is not written by the department add/edit form (no form control, not in the save's column set).
- **BS-030-10 — Signature is public-only and optional.** A department signature is offered on ticket reply only when the department is public and a signature exists.
- **BS-030-11 — Name and signature are HTML-stripped on save.**

### Teams

- **BS-030-12 — Team name required, minimum length, unique.** Name must be present, at least **3** characters, and unique. Errors: `Team name required` / `Team name must be at least 3 chars.` / `Team name already exists`.
- **BS-030-13 — Lead must be a member; removing the lead resets it.** The team lead is chosen from current members only. If the lead is marked for removal during an update, the lead is automatically cleared before saving.
- **BS-030-14 — Members are added from staff profiles, not the team form.** The team edit form only **removes** members; adding a staff member to a team is done from that staff member's profile (owned by FS-031).
- **BS-030-15 — Disabled teams are unavailable.** A disabled team is excluded from ticket assignment and alerts; the "available teams" lookup additionally requires the team to be enabled and to have at least one active, non-vacationing member whose group is enabled.
- **BS-030-16 — Team deletion is unguarded and releases associations.** A team can be deleted at any time; deletion removes its member rows and clears the team assignment on any tickets it owns.
- **BS-030-17 — Assignment-alerts override.** A team may disable assignment alerts, overriding global alert settings.

### Help Topics

- **BS-030-18 — Topic text required, minimum length, unique within parent.** Topic must be present, at least **5** characters, and unique among siblings under the same parent. Errors: `Help topic required` / `Topic is too short. 5 chars minimum` / `Topic already exists`.
- **BS-030-19 — Department and priority required.** A topic must select a department (`You must select a department`) and a priority (`You must select a priority`).
- **BS-030-20 — One-level nesting.** A topic may have a single parent (chosen from top-level topics). The parent picker only lists topics that themselves have no parent, enforcing at most one level of nesting. The display name of a child is rendered `Parent / Child`.
- **BS-030-21 — Topic drives ticket routing and defaults.** When a ticket is created from a topic, the topic supplies the default department, default priority, optional SLA override, optional auto-assignment (staff or team), and the auto-response decision — each applied only where the incoming request did not already specify a value. (Consumed in FS-011/FS-021/FS-041.) A help topic MAY carry an SLA override, but it does **not** unconditionally win over the department: the effective SLA resolution order — **explicit (filter) trump > department SLA > topic SLA > system default** — is owned canonically by FS-021.13 (`selectSLAId`). Department SLA is evaluated **before** topic SLA.
- **BS-030-22 — Auto-assign is mutually exclusive staff-or-team.** A topic auto-assigns to a staff member OR a team OR neither — never both. The combined control encodes the choice with an `s`/`t` prefix; selecting one clears the other.
- **BS-030-23 — SLA and thank-you page are optional overrides.** Topic SLA `0` means "no topic-level SLA"; thank-you page empty means "use the global/system default" and applies to web tickets only. Note: the form help text reads "(Overrides department's SLA)", but this is **misleading** — at runtime the department SLA is resolved *before* the topic SLA, so a department SLA actually takes precedence over a topic SLA. The authoritative resolution order (explicit trump > department > topic > system default) is owned by FS-021.13 (`selectSLAId`); see BS-030-21.
- **BS-030-24 — Auto-response override.** A topic may disable new-ticket auto-response, overriding department settings.
- **BS-030-25 — Disabled topics are excluded from selection.** Only active topics are offered to requesters; public visibility additionally restricts a topic to the public submission form (the public help-topic lookup requires both active and public).
- **BS-030-26 — Topic deletion promotes children and clears references.** Deleting a topic promotes its children to top-level, clears the topic on referencing tickets, and removes FAQ associations (see FS-030.16).
- **BS-030-27 — Topic text is trimmed and HTML-stripped on save.**

### Cross-object

- **BS-030-28 — Department, team, and help-topic management is admin-only.** Each page and view partial enforces the admin gate independently (see FS-030.1).
- **BS-030-29 — Internal-error ID mismatch guard.** On update of any object, a mismatch between the loaded object ID and the submitted hidden `id` raises an internal-error message and aborts the save (`Missing or invalid Dept ID (internal error).` / `Missing or invalid team` / `Internal error. Try again`).
- **BS-030-30 — Partial-success reporting.** Every bulk action reports total success with a plain success message, partial success as `N of M ...`, and total failure with an `Unable to ...` error (per-object wording in FS-030.7 / .12 / .17).

---

## Data Requirements

The canonical table/column schema and enum value sets are owned by **FS-091**. This spec references the following entities functionally; field names below are the behavioral facts surfaced by the management UI.

### Department
- Identity: department ID, name (HTML-stripped, ≥ 4 chars, unique), `created`, `updated`.
- Visibility: `ispublic` (1 = public, 0 = private/internal).
- Email/templating: email-account ID (`email_id`, required), template-group ID (`tpl_id`, required; `0` = system default), auto-response email ID (`autoresp_email_id`; `0` = use department email).
- Routing defaults: SLA ID (`sla_id`; `0` = system default), manager staff ID (`manager_id`; `0` = none).
- Auto-response flags: `ticket_auto_response`, `message_auto_response` (default-on; the form exposes "disable" inversions for both — checked = disabled), and `noreply_autoresp` (read-only here; surfaced by the model but not editable from this form and not in the department save's column set).
- Group membership: `group_membership` flag; allowed-group set stored as group↔department access rows.
- Signature: `dept_signature` (HTML-stripped free text, offered for public departments).
- Derived (read views): home-staff count (`users`), computed member roster (home staff + manager + group-access staff when group membership enabled).

### Team
- Identity: team ID, name (≥ 3 chars, unique), `created`, `updated`.
- Status: `isenabled` (1 = active, 0 = disabled).
- Lead: `lead_id` (must be a current member; `0` = none).
- Alerts: `noalerts` (1 = disable assignment alerts).
- Notes: `notes` (admin-internal free text).
- Membership: team-member rows (staff IDs); member count derived; "available teams" further filters on enabled team + ≥1 active, non-vacationing member in an enabled group.

### Help Topic
- Identity: topic ID, topic text (trimmed, HTML-stripped, ≥ 5 chars, unique within parent), `created`, `updated`.
- Nesting: parent topic ID (`topic_pid`; `0`/null = top-level); display name `Parent / Child`.
- Status & visibility: `isactive` (1/0), `ispublic` (1/0).
- Routing defaults: department ID (`dept_id`, required), priority ID (`priority_id`, required), SLA ID (`sla_id`; `0` = department default), thank-you page ID (`page_id`; empty = system default).
- Auto-assignment: `staff_id` XOR `team_id` (mutually exclusive; both `0` = unassigned), encoded in the UI via an `s`/`t`-prefixed combined value.
- Auto-response: `noautoresp` (1 = disable; default off → auto-respond on).
- Notes: `notes` (admin-internal free text).

---

## User Flows / Interactions

### UF-1: Create a department
1. Admin opens `departments.php`, clicks **Add New Department** (`?a=add`).
2. Fills Name, Type, Email, Template, optional SLA/Manager, auto-response overrides, ticks allowed Groups, optional signature.
3. Submits (`do=create`). On validation error, the form re-renders with inline errors and prior input preserved.
4. On success, allowed-group access rows are written, the success message `<Name> added successfully` shows, and the user lands back on the department list.

### UF-2: Delete departments in bulk
1. Admin selects one or more departments (the default's checkbox is disabled) and clicks **Delete Dept(s)**.
2. A confirmation dialog warns the deletion is irreversible; admin confirms.
3. Server rejects if the default is selected, or if any selected department has staff. Otherwise each is deleted, its tickets/staff/help-topic pointers re-homed to the default department, and a success / partial / failure message is shown.

### UF-3: Manage a team's roster and lead
1. Admin opens an existing team (`teams.php?id=...`).
2. Sees current members each with a **Remove** checkbox; to add members, admin instead edits the target staff member's profile (FS-031).
3. Optionally changes the lead (chosen from current members), status, alert override, notes.
4. Submits (`do=update`). If the lead was marked for removal, the lead resets to none; marked members are removed; the team reloads with `Team updated successfully`.

### UF-4: Configure a help topic for routing
1. Admin opens `helptopics.php`, clicks **Add New Help Topic**.
2. Enters Topic text, Status, Type, optional Parent, then required Priority and Department, optional SLA override, thank-you page, auto-assign target (a staff member or a team), and auto-response override.
3. Submits (`do=create`). The combined auto-assign control is decoded into a staff-or-team assignment. On success → `Help topic added successfully`, back to the (paginated) list.
4. Thereafter, tickets created on this topic inherit its department, priority, SLA, assignment, and auto-response decision (FS-011/FS-021/FS-041).

### UF-5: Confirmation dialogs (all three objects)
- Each list view embeds a hidden confirmation dialog with per-action prompts (`make_public-confirm`, `make_private-confirm`, `enable-confirm`, `disable-confirm`, `delete-confirm`). The delete prompts emphasize that deletion is irreversible. The chosen action is written into a hidden `a` field, the form posts `do=mass_process`, and the server dispatches on `a`.

---

## Edge Cases

- **EC-030-01 — Unknown object ID on edit.** A non-resolvable `id` sets `Unknown or invalid <object> ID.` and falls back to the list view; no object is loaded.
- **EC-030-02 — Update with no valid object loaded.** If `do=update` arrives but the object did not resolve, the error `Unknown or invalid <department|team|help topic>.` is shown and nothing is saved.
- **EC-030-03 — ID tampering on update.** A submitted hidden `id` that disagrees with the loaded object ID triggers an internal-error guard and aborts the save (BS-030-29).
- **EC-030-04 — Department delete refused (default or non-empty).** Deleting the default department, or any department with home staff, is silently refused (0 rows); bulk delete refuses the whole batch if either condition is hit by any selection.
- **EC-030-05 — Making the default department private.** Rejected with `System default department cannot be private`; bulk Make-Private silently skips the default rather than erroring.
- **EC-030-06 — Empty bulk selection.** Each object reports its own "must select at least one ..." error and performs no action.
- **EC-030-07 — Partial bulk success.** When only some selected rows are affected (e.g., already-public departments, undeletable rows), the `N of M ...` partial message is shown rather than success or failure.
- **EC-030-08 — Removing a team's lead member.** Removing the staff member who is the current lead during an update auto-clears the lead before save, preventing a dangling lead reference.
- **EC-030-09 — Deleting a parent help topic.** Children are promoted to top-level (parent cleared) rather than deleted, avoiding orphaned references.
- **EC-030-10 — Deleting an in-use team/topic.** Teams and topics have no in-use guard: deletion proceeds and releases ticket/member/FAQ associations (team → ticket team cleared; topic → ticket topic cleared + FAQ links removed).
- **EC-030-11 — Auto-assign target value with no prefix.** A help-topic auto-assign value not starting with `s` or `t` is treated as "no auto-assignment" (both staff and team cleared).
- **EC-030-12 — Auto-response email pointing at a deleted account.** A department's designated auto-response email that no longer exists falls back to the department's main email.
- **EC-030-13 — Disabled / empty team chosen for assignment.** A disabled team, or an enabled team with no active eligible members, is excluded from the "available teams" assignment lookup even though it remains in the management list.
- **EC-030-14 — Topic name collision across parents.** Uniqueness is scoped to the parent: the same topic text may exist under two different parents (or as both a top-level and a child elsewhere) without collision.
- **EC-030-15 — No-change re-save of a department or team.** Because the department and team update paths require the database to report a changed row, re-saving a department or team without altering any field can (when the save lands within the same one-second tick that already holds in `updated`) register zero affected rows and surface a generic update-failure error (`Unable to update <Name> Dept. Error occurred` / `Unable to update the team. Internal error`) even though nothing was wrong with the input. Help-topic updates do not exhibit this (they report success regardless of affected rows).
- **EC-030-16 — Required-select placeholder left at the prompt value.** The help-topic Priority and Department selects open on a non-numeric placeholder option (`— Select Priority —` / `— Select Department —`, value `""`). Submitting without choosing a real option fails validation with `You must select a priority` / `You must select a department` and the form re-renders with the entered values preserved.

---

## Dependencies

- **FS-002 — Staff authentication, sessions & access control**: provides the admin session, the `isAdmin()` gate, and CSRF protection enforced on every POST in these pages.
- **FS-031 — Admin: staff, groups & directory**: defines staff Groups (whose access checkboxes appear on the department form), staff records (managers, team leads, auto-assign targets), and **team membership addition** (done from the staff profile, not the team form). The Users/Members count links route to the FS-031 staff list filters.
- **FS-032 — Admin: system settings, SLA, priorities & categories**: defines SLA plans (selected by departments and topics) and ticket priorities (selected by topics), plus the system default SLA used as a fallback.
- **FS-040 — Email accounts, templates & outbound mail**: defines the email accounts (department email, auto-response email) and template groups (department template) referenced here.
- **FS-033 — Admin: logs, pages & content**: defines the thank-you Pages selectable on a help topic.
- **FS-011 / FS-021 / FS-041 — Ticket submission, staff workflow, inbound email**: consume help topics (routing, default department/priority/SLA, auto-assignment, auto-response), departments (ownership, transfer target, signature, auto-response), and teams (assignment) at runtime. In particular, **FS-021.13 (`selectSLAId`) is the canonical owner of SLA selection precedence** (explicit trump > department SLA > topic SLA > system default); FS-030 defines only that a topic *can* carry an SLA override, not its precedence (see BS-030-21 / BS-030-23).
- **FS-050 — Knowledge base & FAQ**: FAQ-to-topic associations are cleaned up on topic deletion.
- **FS-090 — Shared UI, navigation & data export**: provides the staff header/footer, the list-table styling, the confirmation-dialog pattern, and the paginator used by the help-topic list.
- **FS-091 — Reference data, enums & data model**: canonical owner of the department/team/help-topic table schemas and all enum/flag value sets (visibility, status, auto-response flags).

---

## Known Limitations

- **KL-030-01 — Department minimum name length is 4; team is 3; topic is 5.** These thresholds are inconsistent across the three objects and are hard-coded, not configurable.
- **KL-030-02 — Help-topic nesting is exactly one level.** The parent picker only offers top-level topics, so grandchild topics are structurally impossible; deleting a parent promotes (not re-parents) children.
- **KL-030-03 — Team members cannot be added from the team form.** Adding membership requires editing each staff member's profile individually; there is no multi-add roster editor on the team page.
- **KL-030-04 — No pagination on departments or teams.** Both list every row in one page; only help topics paginate. Large deployments render long single pages.
- **KL-030-05 — Bulk department delete is all-or-nothing on the "has staff" check.** A single selected department with staff blocks the entire batch, even deletable ones, with no per-row skip.
- **KL-030-06 — Department deletion re-homing is bulk, not per-ticket.** The source comments note the intended one-ticket-at-a-time move with alerts and log notes is not implemented; all tickets are re-homed in a single bulk update with no per-ticket alert or audit note.
- **KL-030-07 — Topic deletion does not warn about active tickets.** Deleting a topic silently clears the topic on any referencing tickets with no usage warning, unlike the department's staff guard.
- **KL-030-08 — Department template `0` and SLA `0` mean "system default" implicitly.** The UI relies on the `0` sentinel value rather than an explicit flag, conflating "unset" with "system default".
- **KL-030-09 — Auto-assignment encodes staff-vs-team in a single overloaded string.** The `s`/`t`-prefixed value is a UI convenience that the save routine must parse; a malformed value silently degrades to "unassigned" with no error.
- **KL-030-10 — No referential check that a chosen auto-assign staff/team is still valid at save time** beyond the active/enabled filtering applied when the option list is built; a stale posted value is stored as-is.
- **KL-030-11 — Update success is keyed on affected-row count for departments and teams but not help topics.** Department and team updates require a nonzero affected-row count to report success, so a genuine no-op save can spuriously error (EC-030-15); help-topic updates always report success once the query runs. The three objects are therefore behaviorally inconsistent on the no-change case.
- **KL-030-12 — Team "Last Updated" column header is a dead sort link.** The header emits a `sort=updated` link, but `updated` is not among the team's recognized sort keys, so the click silently reverts to the default `name` sort. (Help topics, by contrast, do support `sort=updated`.)
- **KL-030-13 — Team name and notes (and department/topic notes) are not HTML-stripped on save.** Only department `name`/`signature` and help-topic `topic` text are stripped of HTML on write; team `name`/`notes`, department free-text beyond name/signature, and topic `notes` are stored as submitted and sanitized only at render time, so the stored value is not normalized.
