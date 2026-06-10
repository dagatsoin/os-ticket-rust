# FS-031: Admin — Staff, Groups & Directory

## Overview

This specification defines the administrative management of **staff accounts**, **permission groups**, the staff **directory** (a non-admin browse/search listing), and each staff member's editing of their **own account profile**. Together these features establish the human-actor side of the help desk: who has a login, what each login is permitted to do, how staff are organized into groups for permission inheritance and into departments for ticket visibility, and how a staff member self-services their contact details, preferences, and password.

Four entry points are owned by this spec:

- **Staff management** (`scp/staff.php` → `staffmembers.inc.php` list + `staff.inc.php` form) — admin-only CRUD over staff accounts, including username/password, contact details, account type (Admin vs Staff), account status (Active vs Locked), group/department/team assignment, vacation mode, directory visibility, limited-access flag, signature, time-zone/daylight settings, and admin notes.
- **Group management** (`scp/groups.php` → `groups.inc.php` list + `group.inc.php` form) — admin-only CRUD over permission groups: group name, enabled status, the full set of permission flags applied to all members, the department-access matrix, and admin notes.
- **Staff directory** (`scp/directory.php` → `directory.inc.php`) — a read-only, search-and-filter listing of directory-visible staff, available to **any** authenticated staff member (not admin-only).
- **Own profile** (`scp/profile.php` → `profile.inc.php`) — a staff member's self-service editing of their own contact info, preferences, signature, and password.

**Scope boundaries.** This spec covers the admin **CRUD** of staff and groups and the *definitions* of group permission flags. The **enforcement** of those permission flags and the staff **login/session/strike/lockout** machinery are owned by **FS-002** (Staff authentication, sessions & access control); they are referenced here as dependencies. Department, team, and help-topic CRUD are owned by **FS-030**. Canonical table/column schemas and enum value sets are owned by **FS-091** and are referenced, not restated. Validation/formatting helpers (`Validator`, `Format`, `Misc`) and signals (`Signal`) are owned by **FS-003**. Pagination (`Pagenate`) and the admin shell/header/footer/nav are owned by **FS-090 / FS-001**.

---

## Functional Requirements

### FS-031.1: Staff Management Access Gate

**Description**: The system shall restrict all staff-management screens (list, add, edit, mass-process) to authenticated administrators.

**Acceptance Criteria**:
- `scp/staff.php` includes `admin.inc.php`, which both requires an authenticated staff session and enforces the administrator gate (owned by FS-001/FS-002).
- Both view partials (`staffmembers.inc.php`, `staff.inc.php`) independently re-check the gate at top of file: if the admin-include constant is not defined OR there is no current staff OR the current staff is not an administrator, output the literal `Access Denied` and halt.
- A non-admin staff member who reaches these screens receives `Access Denied`; they do not see staff lists or forms.

### FS-031.2: Staff List, Filter, Sort & Paginate

**Description**: The system shall present administrators a paginated, sortable, filterable list of all staff accounts.

**Acceptance Criteria**:
- The default view (when no specific staff is targeted and the request is not "add") is the staff-members list.
- Each row displays: a selection checkbox; **Name** (linking to the edit form for that staff); **UserName**; **Status** (`Active` or **`Locked`**, with the suffix `(vacation)` shown in small italics when the staff is on vacation); **Group** (linking to the group's edit form); **Department** (linking to the department's edit form); **Created** date; **Last Login** date-time.
- The list supports three independent filters applied via a GET form (button label **`Apply`**):
  - **Department** filter (`did`) — drop-down seeded only with departments having at least one staff member, each shown as `Name (userCount)`; default option `— All Departments —`.
  - **Group** filter (`gid`) — drop-down of groups that have at least one member, each shown as `Name (userCount)`; default option `— All Groups —`.
  - **Team** filter (`tid`) — drop-down of teams that have at least one member, each shown as `Name (userCount)`; default option `— All Teams —`.
- Filters combine with AND logic; only numeric filter values are honored.
- Sortable columns and their sort keys: `name` (first+last name), `username`, `status` (active flag), `group` (group name), `dept` (department name), `created`, `login` (last login). Default sort is `name`, ascending. Each column header is a clickable link that toggles the order direction; the active column carries an order-direction CSS class.
- The list is paginated using the configured page limit (`PAGE_LIMIT`); a page caption shows the "showing N–M of T" range or **`No staff found!`** when empty.
- A **`Add New Staff`** link (target `staff.php?a=add`) is always present above the list.
- Below the list, when at least one row is shown, **mass-action** controls are offered: select All / None / Toggle, and buttons **`Enable`**, **`Lock`** (disable), **`Delete`**.

### FS-031.3: Create / Edit a Staff Account

**Description**: The system shall let an administrator create a new staff account or edit an existing one through a single form whose mode is determined by the request.

**Acceptance Criteria**:
- The form renders in **Add** mode when the request action is `add` (title `Add New Staff`, submit button `Add Staff`, hidden action `create`); otherwise, when a valid staff target is loaded, in **Edit** mode (title `Update Staff`, submit `Save Changes`, hidden action `update`).
- In Add mode the form pre-seeds defaults: forced password change enabled, account status Active, directory listing visible, account type Staff (not Admin).
- The form collects, grouped under labeled sections:
  - **User Information**: Username (required), First Name (required), Last Name (required), Email Address (required), Phone Number + extension, Mobile Number.
  - **Account Password**: Password + Confirm Password; in Add mode the temp password is required (label `Temp. password required *`); in Edit mode it is optional (label `To reset the password enter a new one below`). A **Forced Password Change** checkbox forces a password change on next login.
  - **Staff's Signature**: an optional free-text signature used on outgoing emails.
  - **Account Status & Settings**: Account Type radio (**Admin** / **Staff**); Account Status radio (**Active** / **Locked**); **Assigned Group** drop-down (required; disabled groups shown with ` (Disabled)`); **Primary Department** drop-down (required); **Staff's Time Zone** drop-down (required, shown as `GMT <offset> - <timezone>`); **Daylight Saving** checkbox (with a live "Current Time" preview); **Limited Access** checkbox ("Limit ticket access to ONLY assigned tickets"); **Directory Listing** checkbox ("Show the user on staff's directory"); **Vacation Mode** checkbox ("No ticket assignment or alerts").
  - **Assigned Teams**: a checkbox per team (disabled teams shown with ` (Disabled)`); a staff member sees tickets assigned to any team they belong to regardless of the ticket's department.
  - **Admin Notes**: internal free text viewable by all admins.
- On successful create, the success message is `<name> added successfully` and the form is dismissed back toward the list. On successful update, the message is `Staff updated successfully`.
- On validation failure, the form re-renders with the submitted values and per-field error markers, and a general banner (`Unable to add staff. Correct any error(s) below and try again.` on create, `Unable to update staff. Correct any error(s) below and try again!` on update).
- A **Cancel** button returns to `staff.php` without saving; a **Reset** button (a form reset) restores the form's initial field values without saving.
- The **Forced Password Change** control is a checkbox whose presence (when checked) sets the forced-change flag on save; on the create form it is pre-checked, and in Add mode the flag is pre-seeded on (BS-031-013, EC-031-013). (The control's submitted value is not what determines the flag — the persisted flag is set whenever the field is present in the submission.)

### FS-031.4: Staff Mass Actions (Enable / Lock / Delete)

**Description**: The system shall let an administrator enable, lock (disable), or delete multiple selected staff accounts in one operation.

**Acceptance Criteria**:
- At least one staff member must be selected, else: `You must select at least one staff member.`
- The current administrator may not include themselves in any mass action: if their own id is among the selected ids, the operation is refused with `You can not disable/delete yourself - you could be the only admin!`
- **Enable** sets the active flag on for all selected; reports `Selected staff activated` (or a partial-count warning `N of M selected staff activated`).
- **Lock (disable)** sets the active flag off for all selected **except** the acting administrator's own id (guarded in the query); reports `Selected staff disabled` (or partial `N of M selected staff disabled`).
- **Delete** removes each selected staff except the acting administrator; reports `Selected staff deleted successfully`, partial `N of M selected staff deleted`, or failure `Unable to delete selected staff.`
- Locked staff cannot log in to the Staff Control Panel (enforcement owned by FS-002); the confirmation copy states this.
- Each destructive action presents a confirmation dialog before submission (delete copy warns "Deleted staff CANNOT be recovered").

### FS-031.5: Staff Field Validation

**Description**: The system shall validate staff account fields on create and edit before persisting.

**Acceptance Criteria** — the system rejects the save and reports the matching message when:
- First name empty → `First name required`; last name empty → `Last name required`.
- Username empty or invalid → `Username required` (or the validator's specific message); username already used by a different staff id → `Username already in use`.
- Email empty or not a valid email → `Valid email required`; email already used as a system email → `Already in-use system email`; email already used by another staff member → `Email already in use by another staff member`.
- Phone present but invalid → `Valid number required` (same for Mobile).
- Password rules (see BS-031-013): missing temp password on create → `Temp. password required` / `Required`; new password shorter than 6 characters → `Must be at least 6 characters`; password and confirmation mismatch → `Password(s) do not match`.
- Department not selected → `Department required`; group not selected → `Group required`; time zone not selected → `Time zone required`.
- (Own profile only) An exact-duplicate staff email reports `Email already in-use by another staff member` (note: the **admin** staff form uses the wording `Email already in use by another staff member` — the two screens differ by the hyphen/spacing; both are exact literals from source — see FS-031.13).
- The save would remove or lock the last active administrator → `Cowardly refusing to remove or lock out the only active administrator` (attached to the `isadmin` field) — see BS-031-014.

### FS-031.6: Group Management Access Gate

**Description**: The system shall restrict all group-management screens to authenticated administrators.

**Acceptance Criteria**:
- `scp/groups.php` includes `admin.inc.php` (admin gate, FS-002).
- Both group view partials re-check the admin gate at top of file and emit `Access Denied` on failure, identical to FS-031.1.

### FS-031.7: Group List, Sort & Mass Actions

**Description**: The system shall present administrators a sortable list of all permission groups with mass enable/disable/delete.

**Acceptance Criteria**:
- Each row displays: a selection checkbox; **Group Name** (linking to the group edit form); **Status** (`Active` or **`Disabled`**); **Members** count (a clickable link to the staff list filtered by that group when > 0, else `0`); **Departments** count (number of departments the group can access); **Created On** date; **Last Updated** date-time.
- Sortable columns/keys: `name`, `status`, `users` (member count), `depts` (department-access count), `created`, `updated`. Default sort `name` ascending; headers toggle order direction.
- The list is **not paginated** (all groups shown); caption reads `Showing 1-N of N groups` or `No groups found!`.
- An **`Add New Group`** link (`groups.php?a=add`) is present.
- Mass actions when rows exist: **Enable**, **Disable**, **Delete**, each with a confirmation dialog (delete warns "Deleted groups CANNOT be recovered and might affect staff's access").
- At least one group must be selected, else `You must select at least one group.`
- The acting administrator may not mass-process a group they belong to: if their own group id is among the selected ids → `As an admin, you can't disable/delete a group you belong to - you might lockout all admins!`
- Enable/disable update the group-enabled flag and the updated timestamp; messages mirror the staff mass-action pattern (`Selected groups activated` / `Selected groups disabled` / `Selected groups deleted successfully`, with partial-count warnings and failure messages).

### FS-031.8: Create / Edit a Group

**Description**: The system shall let an administrator create or edit a permission group through a single mode-aware form.

**Acceptance Criteria**:
- Add mode: title `Add New Group`, submit `Create Group`, action `create`, with defaults Status=Active and "Can Create Tickets"=Yes pre-checked. Edit mode: title `Update Group`, submit `Save Changes`, action `update`.
- The form collects:
  - **Group Information**: **Name** (required); **Status** radio **Active** / **Disabled**.
  - **Group Permissions**: the full permission-flag set (BS-031-020), each as a Yes/No radio pair with an explanatory hint, applied to all group members.
  - **Department Access**: a checkbox per department; checked departments form the group↔department access set; convenience `Select All` / `Select None` links.
  - **Admin Notes**: internal free text.
- Group name validation: empty → `Group name required`; fewer than 3 characters → `Group name must be at least 3 chars.`; duplicate of another group's name → `Group name already exists`.
- On success: create → `<name> added successfully`; update → `Group updated successfully`. On failure, the form re-renders with submitted values and the banner `Unable to add group. Correct error(s) below and try again.` (create) or `Unable to update group. Correct any error(s) below and try again!` (update).
- A note states: a disabled group limits its members' access, but administrators are exempt from group-disabled restriction (enforcement owned by FS-002).
- The form offers **Reset** (a form reset) and **Cancel** (returns to `groups.php` without saving) controls in both modes.

### FS-031.9: Staff Directory (Browse & Search)

**Description**: The system shall provide every authenticated staff member a searchable, filterable, paginated directory of directory-visible staff.

**Acceptance Criteria**:
- The directory (`scp/directory.php`) is reachable by any authenticated staff member; its gate requires a staff session (not the admin gate). The view re-checks: if the staff-include constant is undefined OR no current staff OR the subject is not staff → `Access Denied`.
- Only staff whose **directory-visible** flag is set are listed.
- Columns: **Name**, **Department**, **Email Address**, **Phone Number**, **Phone Ext**, **Mobile Number**. No selection checkboxes, edit links, or mass actions (read-only).
- A free-text search box (`q`) matches as follows (BS-031-030): a purely numeric term matches phone / extension / mobile; a term containing `@` that is a valid email matches the email exactly; any other term matches email OR last name OR first name (substring, case-insensitive).
- A **Department** filter (`did`) drop-down lists only departments having at least one directory-visible staff member, each as `Name (userCount)`; default `— All Departments —`. The search submit button is labeled **`Filter`**.
- Sortable columns/keys: `name`, `email`, `dept`, `phone`, `mobile`, `ext`, `created`, `login`; default `name` ascending.
- Paginated by `PAGE_LIMIT`; caption shows the showing-range or `No staff members found!`.

### FS-031.10: Own Profile View & Edit

**Description**: The system shall let each staff member view and edit their own account profile.

**Acceptance Criteria**:
- `scp/profile.php` loads the **currently authenticated** staff member's record (it ignores any submitted id except a dummy id used to guard against tampering — see EC-031-006). The view gate requires a staff session and a loaded subject.
- The profile shows **Username** as read-only (it cannot be changed via the profile).
- Editable fields: First Name (required), Last Name (required), Email Address (required), Phone Number + extension, Mobile Number; **Preferences**: Time Zone (required), Daylight Saving, **Maximum Page Size** (`— system default —` or 5..50 in steps of 5), **Auto Refresh Rate** (`— disable —` or 1..30 minutes), **Default Signature** (`— None —` / `My Signature` / `Dept. Signature (if set)`), **Default Paper Size** (`— None —` / `Letter` / `Legal` / `A4` / `A3`); **Signature** free text.
- A **Show Assigned Tickets** checkbox is offered only to staff who are an administrator or a department manager.
- **Password** change section: New Password + Confirm New Password, plus **Current Password** — except when the session carries a password-reset token, in which case the current-password field is hidden and the reset token authorizes the change (see BS-031-013 and FS-002 for the reset-token flow).
- On success: `Profile updated successfully`; the session's time-zone offset and daylight settings are refreshed from the saved values. On failure: per-field errors plus `Profile update error. Try correcting the errors below and try again!`.
- A Cancel button (label `Cancel Changes`) returns to the staff dashboard (`index.php`) without saving; a Reset button (label `Reset Changes`) is also offered (a form reset).
- The **Maximum Page Size** drop-down highlights the currently-effective value: if the staff has no per-staff override, the system page-size default is shown selected. The **Auto Refresh Rate** drop-down lists 1–10 minutes in steps of 1 and 11–30 in steps of 2 (not a uniform 1-minute step across the whole range).

### FS-031.11: Forced-Password-Change and Vacation Notices on Profile

**Description**: The system shall surface contextual notices when a staff member opens their own profile.

**Acceptance Criteria**:
- If the staff member's account is flagged for forced password change and there is no other error, an error banner is shown: `Hi <FirstName> - You must change your password to continue!`
- Else, if the staff member is on vacation and there is no other warning, a warning banner is shown: `Welcome back <FirstName>! You are listed as 'on vacation' Please let your manager know that you are back.`

### FS-031.12: Periodic Password-Reset Aging (Forced Change on Login)

**Description**: The system shall optionally force a staff member to change their password after a configured age, surfaced as a forced-password-change at next login.

**Acceptance Criteria**:
- A staff account tracks the age of its password as the elapsed time since the password-reset timestamp (or, for never-reset legacy accounts, since account creation).
- A configured **password-reset period** (in months) is consulted on login; when the period is non-zero AND the account's password age exceeds `period × 30 × 24 × 60 × 60` seconds, the account is considered "reset due."
- On a successful login of a non-administrator whose password is reset-due, the forced-password-change flag is set, so the next page entry surfaces the forced-change banner (FS-031.11). **Administrators are exempt** from this automatic forcing.
- (The reset-period configuration value and the login itself are owned by FS-002/FS-032; this spec documents only the staff-account aging behavior that drives the forced-change flag.)

### FS-031.13: Own-Profile Password Change Section

**Description**: The system shall validate the own-profile password change only when the staff member is attempting a change, using profile-specific messages and the current-password / reset-token authorization path.

**Acceptance Criteria**:
- The password-change validation block runs only when at least one of **New Password**, **Confirm New Password**, or **Current Password** is non-empty; if all three are blank, the password is left unchanged and no password errors are raised.
- Within that block, in order: a blank New Password → `New password required`; New Password shorter than 6 characters → `Must be at least 6 characters`; New Password and Confirm mismatch → `Password(s) do not match`.
- Authorization for the change is one of two mutually exclusive paths:
  - **Reset-token path** (a reset token is present in the session): the token must resolve to this staff id and be within the reset window, else `Invalid reset token. Logout and try again`. No current password is requested or checked.
  - **Current-password path** (no reset token): a blank Current Password → `Current password required`; an incorrect Current Password → `Invalid current password!`; a New Password equal (case-insensitively) to the Current Password → `New password MUST be different from the current password!`.
- On a successful password change the system clears the forced-password-change flag, records the password-reset timestamp, cancels outstanding reset tokens for the account, and emits a password-change signal (BS-031-013).

---

## Business Rules

### BS-031-001: Staff Identity Is Username + Email, Both Unique
A staff account is uniquely identified by its **username** (unique across staff) and its **email** (unique across staff and not colliding with any configured system email). Lookups resolve a numeric value as a staff id, an email-shaped value as the email, and any other value as the username.

### BS-031-002: Account Type — Admin vs Staff
Each account is either **Admin** or **Staff** (`isadmin`). Admin grants access to the administrative control panel and exempts the holder from group/department permission restrictions (enforcement in FS-002). Account type is set on create/edit via a radio pair defaulting to Staff.

### BS-031-003: Account Status — Active vs Locked
Each account has an active flag rendered as **Active** / **Locked**. A locked account cannot log in (FS-002). Status is toggled via the create/edit radio or via the list mass Enable/Lock actions.

### BS-031-004: Group Membership Drives Permissions
Every staff account belongs to exactly one **group** (required). The group supplies the permission flags (BS-031-020) that apply to all its members. A staff account also inherits the **department-access** set of its group (BS-031-021).

### BS-031-005: Primary Department & Effective Department Access
Every staff account has one required **primary department**. The set of departments a staff member may access is the union of: their primary department, any department they manage (a department whose manager is this staff member), and the departments granted to their group. The result is de-duplicated and emptied of null entries. If the combined lookup yields nothing, the system falls back to the union of the group's department-access set and the staff's primary department. This effective set governs ticket visibility (consumed by FS-020/FS-021). A staff member is considered able to access a department only when that department is in the effective set **and** the staff member is not flagged Limited Access (BS-031-006).

### BS-031-006: Limited Access Overrides Department Access
When a staff account is flagged **Limited Access** (assigned-only), department access is suppressed for ticket visibility: the staff member sees only tickets explicitly assigned to them (or to teams they belong to), regardless of department membership.

### BS-031-007: Team Membership Is Cross-Department
A staff member assigned to one or more **teams** can access tickets assigned to those teams regardless of the ticket's department. Team assignment is edited as a checkbox set on the staff form; saving fully reconciles membership (inserts new memberships, removes unchecked ones).

### BS-031-008: Vacation Mode Suppresses Assignment & Alerts
A staff account on **Vacation Mode** receives no new ticket assignments and no alerts. An account is considered **available** only when it is active, its group is enabled, and it is not on vacation. "Available staff" lookups (e.g., for assignment pickers) exclude inactive, disabled-group, and on-vacation staff.

### BS-031-009: Directory Visibility Flag
A staff account's **Directory Listing** flag (`isvisible`) controls whether the account appears in the staff directory (FS-031.9). It does not affect login, permissions, or admin lists. Default on create is visible.

### BS-031-010: Signature & Signature Type
A staff account may have an optional free-text **signature** used as a selectable choice on ticket replies. The profile's **Default Signature** preference may be `none`, `mine` (the staff signature), or `dept` (the department's signature if set). Selecting `mine` without having a signature is rejected: `You don't have a signature`.

### BS-031-011: Time Zone & Daylight Saving
A staff account requires a **time zone** (referenced from the timezone reference table; canonical set in FS-091). An optional **Daylight Saving** flag indicates the account observes daylight time. These two values drive the offset applied to all date-time display for that session and are written into the session on login and on profile save.

### BS-031-012: Per-Staff Display Preferences
Each staff account carries display preferences: **Maximum Page Size** (records per list page; `0` = system default, otherwise 5–50 in steps of 5), **Auto Refresh Rate** (ticket-page refresh in minutes; `0` = disabled, otherwise selectable values 1–10 in steps of 1 and 11–30 in steps of 2), and **Default Paper Size** for PDF printing (`Letter` / `Legal` / `A4` / `A3` / `none`, where the stored "no preference" value is the literal `none`). The **Default Signature Type** stored "no preference" value is the literal `none`. These preferences are editable only on the staff's own profile, not the admin staff form.

### BS-031-013: Password Rules
- A new password must be **at least 6 characters**.
- Password and confirmation must match.
- On admin **create**, a temp password is required.
- On the **own profile**, the password-change validation runs only when New Password, Confirm New Password, or Current Password is non-empty; a blank New Password while the block is engaged reports `New password required`. Changing the password normally requires the **current password**: a blank current password → `Current password required`; an incorrect current password → `Invalid current password!`; the new password must differ (case-insensitively) from the current one → `New password MUST be different from the current password!`. (See FS-031.13 for the full ordered profile flow.)
- A valid **password-reset token** in the session substitutes for the current password (the current-password field is then hidden); an invalid/expired token yields `Invalid reset token. Logout and try again`. (Reset-token issuance/expiry window is owned by FS-002.)
- Saving a new password clears the forced-password-change flag, records the reset timestamp, cancels outstanding reset tokens, and emits a password-change signal.
- The **Forced Password Change** flag, when set, requires the staff member to change their password on next login (surfaced via FS-031.11).

### BS-031-014: Last-Administrator Protection
The system refuses any save that would leave the install with **no active administrator**. If the edit would clear the admin flag or lock the account, and a count shows exactly one active administrator and it is the account being edited, the save is rejected: `Cowardly refusing to remove or lock out the only active administrator`.

### BS-031-015: Self-Action Protection (Staff Mass Actions)
An administrator cannot disable or delete **themselves** through mass actions (`You can not disable/delete yourself - you could be the only admin!`). The disable query additionally excludes the acting administrator's id as a safety net.

### BS-031-016: Staff Deletion Side Effects
Deleting a staff account is permitted only when there **is** a currently-acting staff member in context AND the target is **not** that acting staff (the delete routine returns "0 deleted" when no acting staff is set or when the target id equals the acting staff id). On deletion the system unassigns the deleted staff from any **open** tickets (sets staff assignment to none) and removes all of their **team memberships**; these house-cleaning steps run only when the staff row was actually removed. A deletion signal is emitted regardless. Deleted staff cannot be recovered.

### BS-031-017: Username Character Rules
A username must pass the username validator owned by **FS-003.9 / BS-013** (length and allowed-character rules, with their literal error strings); on failure the staff save is rejected. Usernames are stored stripped of HTML tags.

### BS-031-018: Phone Number Acceptance
Phone and mobile numbers, when provided, must contain only digits and the punctuation `( ) - . +` and spaces, and after stripping that punctuation must be numeric with a length between **7 and 16** digits; otherwise `Valid number required`. Stored numbers are normalized via the phone formatter (FS-003).

### BS-031-019: Group Identity & Minimum Name Length
A group is identified by a **unique name** of at least **3 characters**. Group names are stored stripped of HTML tags. A group carries an **enabled** flag (Active / Disabled) and a free-text admin-notes field.

### BS-031-020: Group Permission Flag Set (Canonical)
A group defines the following permission flags, each a boolean applied to every member. (Enforcement of each is owned by FS-002 and the consuming feature spec.)

| Flag (storage key) | Form label | Meaning |
|---|---|---|
| `can_create_tickets` | Can **Create** Tickets | Open tickets on behalf of clients. |
| `can_edit_tickets` | Can **Edit** Tickets | Edit tickets. |
| `can_post_ticket_reply` | Can **Post Reply** | Post a ticket reply. |
| `can_close_tickets` | Can **Close** Tickets | Close tickets (staff may still post a response). |
| `can_assign_tickets` | Can **Assign** Tickets | Assign tickets to staff members. |
| `can_transfer_tickets` | Can **Transfer** Tickets | Transfer tickets between departments. |
| `can_delete_tickets` | Can **Delete** Tickets | Delete tickets (not recoverable). |
| `can_ban_emails` | Can Ban Emails | Add/remove emails from the banlist via the ticket interface. |
| `can_manage_premade` | Can Manage Premade | Add/update/disable/delete canned responses and attachments. |
| `can_manage_faq` | Can Manage FAQ | Add/update/disable/delete knowledgebase categories and FAQs. |
| `can_view_staff_stats` | Can View Staff Stats. | View other staff members' stats in allowed departments. |

> Note: a derived capability "can manage tickets" is true for any admin OR any group member with delete-tickets OR close-tickets (defined in FS-002).

### BS-031-021: Group↔Department Access Matrix
A group has an associated set of departments its members may access, edited as a department checkbox set on the group form. Saving fully reconciles the set (inserts checked, removes unchecked). This set feeds the effective department access of every member (BS-031-005).

### BS-031-022: Group Deletion Requires Zero Members
A group **cannot be deleted while it has members**; deletion is refused in that case. Deleting an empty group also removes its department-access rows.

### BS-031-023: Self-Group Protection (Group Mass Actions)
An administrator cannot mass-disable or mass-delete a group they themselves belong to (`As an admin, you can't disable/delete a group you belong to - you might lockout all admins!`).

### BS-031-024: Group-Disabled Exemption for Admins
A disabled group limits its members' access; **administrators are exempt** from this restriction (an admin in a disabled group retains access). Enforcement owned by FS-002.

### BS-031-030: Directory Search Term Interpretation
The directory free-text search interprets the term by shape: purely numeric → match phone/extension/mobile; contains `@` and is a valid email → exact email match; otherwise → substring match against email, last name, or first name (case-insensitive).

### BS-031-031: Directory Visibility Restriction
The directory lists **only** directory-visible staff (BS-031-009); its department filter likewise counts only visible staff. The directory exposes contact fields only (name, department, email, phone, extension, mobile) and never editing controls.

### BS-031-032: Profile Edits Self Only
The own-profile screen always operates on the authenticated staff member's record. A submitted id that does not match the session's staff id (other than the dummy form id check) is treated as an internal error and the action is denied (EC-031-006).

### BS-031-033: Staff-List Filter Drop-down Population
The staff-list filters draw their option sets as follows: the **Department** drop-down lists departments that have at least one staff member, each as `Name (count)` (an inner join filtered with a `count > 0` clause); the **Group** and **Team** drop-downs list only groups / teams that have at least one member (achieved by an inner join to the membership relation), each as `Name (count)`. Empty departments / groups / teams therefore never appear in the filters. The counts shown are unrestricted member counts (the staff list itself is admin-only, so visibility is not factored in here — contrast the directory's department filter, BS-031-031, which counts only directory-visible staff).

### BS-031-034: Group List Counts and Linkage
The group list shows, per group, a distinct **member count** and a distinct **department-access count**. The member count, when greater than zero, is a link to the staff list filtered by that group (`staff.php?gid=<id>`); a zero member count renders as a plain `0` with no link. The department-access count always renders as a plain number (never a link). Groups with zero members and/or zero department-access rows still appear in the list (left-join based), so the list is a complete enumeration of all groups.

---

## Data Requirements

> Canonical table/column definitions and enum value sets live in **FS-091**. This section describes the data **functionally**; it does not restate schema.

**Staff account** must capture: identity (username, first name, last name, email); contact (phone, phone extension, mobile); credentials (password hash, forced-password-change flag, password-reset timestamp); account type (admin flag); status (active flag); organizational placement (group reference, primary department reference, team memberships); behavior flags (limited/assigned-only access, directory-visible, on-vacation, show-assigned-tickets); locale (time-zone reference, daylight-saving flag); presentation preferences (max page size, auto-refresh rate, default signature type, default paper size); free-text signature; admin notes; and lifecycle timestamps (created, updated, last login).

**Group** must capture: unique name; enabled flag; the eleven permission flags (BS-031-020); admin notes; created and updated timestamps; and (derived for display) member count and department-access count.

**Group↔Department access** is a many-to-many association between a group and the departments its members may access.

**Team membership** is a many-to-many association between a staff member and the teams they belong to (team CRUD owned by FS-030).

Derived/computed values surfaced by this spec: effective department access (BS-031-005), availability (BS-031-008), member/department counts on the group list, and the live "Current Time" preview computed from the chosen time zone and daylight flag.

---

## User Flows / Interactions

**Flow A — Add a staff account.** Admin opens the staff list → clicks **Add New Staff** → fills User Information, a temp password (forced-change pre-checked), account type/status, group, primary department, time zone, optional teams/flags/signature/notes → submits **Add Staff** → on success sees `<name> added successfully` and returns toward the list; on error the form re-renders with field markers.

**Flow B — Edit / lock a staff account.** Admin clicks a name in the list → edits fields → **Save Changes** → `Staff updated successfully`. Alternatively, from the list, selects rows and clicks **Lock** / **Enable** / **Delete**, confirms the dialog, and sees the count-based result message. Last-admin and self-action protections may block the action.

**Flow C — Define a group.** Admin opens the group list → **Add New Group** → sets name, status, toggles each permission flag, checks accessible departments (with Select All/None), adds notes → **Create Group**. Editing follows the same form with **Save Changes**. Members count links back to the staff list filtered by that group.

**Flow D — Browse the directory.** Any staff member opens the directory → optionally types a search term and/or picks a department → **Filter** → reads the contact listing, sorts by any column, pages through results. No editing is possible.

**Flow E — Edit own profile.** A staff member opens their profile → updates contact info, preferences, signature → optionally changes password (current password required unless a reset token is active) → **Save Changes** → `Profile updated successfully`; session locale settings refresh. Forced-change or returning-from-vacation banners may appear on entry.

---

## Edge Cases

- **EC-031-001 — Unknown staff/group id.** A request carrying an id that resolves to no record yields `Unknown or invalid staff ID.` / `Unknown or invalid group ID.` and the update path reports `Unknown or invalid staff.` / `Unknown or invalid group.`
- **EC-031-002 — Empty mass selection.** Submitting a mass action with no rows selected yields `You must select at least one staff member.` / `You must select at least one group.`
- **EC-031-003 — Acting admin in selection.** Including oneself (staff) or one's own group (groups) in a mass action is blocked with the respective lockout-prevention message (BS-031-015 / BS-031-023).
- **EC-031-004 — Removing the last admin.** Clearing admin or locking the sole active administrator on the edit form is rejected (BS-031-014). (The mass-delete path does not run the count check directly but is guarded by self-exclusion; deleting all *other* admins is possible, leaving the acting admin.)
- **EC-031-005 — Deleting a non-empty group.** Group delete silently fails for any group with members (BS-031-022); the mass result reports a partial or `Unable to delete selected groups`.
- **EC-031-006 — Profile id tampering.** If the profile form's submitted id differs from the session staff id, the action is denied with `Internal Error. Action Denied` (and the inner update repeats an `Internal Error` guard).
- **EC-031-007 — Email collides with a system email.** Using an address already configured as a system (mailbox) email is rejected on both the admin form (`Already in-use system email`) and the profile (`Already in-use as system email`).
- **EC-031-008 — Reset token mismatch on profile.** A session reset token whose stored value does not match the staff id, or whose age exceeds the reset window, yields `Invalid reset token. Logout and try again`.
- **EC-031-009 — Mass action partial success.** When only some selected rows succeed, the result is a warning of the form `N of M selected … ` rather than a flat success or failure.
- **EC-031-010 — Disabled group / team in pickers.** Disabled groups and teams still appear in the staff form's pickers, suffixed ` (Disabled)`, so an admin may knowingly assign to them; a disabled group restricts non-admin members (BS-031-024).
- **EC-031-011 — Selecting "My Signature" with no signature.** The profile rejects a `mine` default-signature choice when the signature field is empty (`You don't have a signature`).
- **EC-031-012 — Directory numeric-term query quirk.** The numeric-search branch builds a phone/ext/mobile match clause that (in the observed source) omits an `OR` between the extension and mobile conditions — documented as KL-031-002.
- **EC-031-013 — Forced-password-change checkbox is presence-driven.** The staff form's Forced-Password-Change control is a checkbox carrying the literal value `0`, yet the persisted forced-change flag is set to on whenever that field is *present* in the submission (i.e., when the box is checked), and left untouched when absent. The submitted value is therefore not what drives the flag; only the field's presence does. Admins should treat the box as "force on next login = checked." (BS-031-013, FS-031.3.)
- **EC-031-014 — Profile password block engaged by current-password alone.** Because the own-profile password validation engages whenever New, Confirm, or Current password is non-empty, filling only the Current Password (with both new fields blank) still engages the block and reports `New password required` rather than silently ignoring the input. (FS-031.13.)
- **EC-031-015 — Reset-token window check edge.** On the own profile, when a session reset token is present, the system verifies the token maps to the staff id and is within the configured reset window; a token whose stored modification time cannot be read, or whose age exceeds the window, yields `Invalid reset token. Logout and try again`. (Token issuance/expiry owned by FS-002; see also EC-031-008.)
- **EC-031-016 — Per-form email-collision wording differs.** A staff email already used by another staff member is rejected on the admin form as `Email already in use by another staff member` and on the own profile as `Email already in-use by another staff member` (hyphenation differs); a collision with a configured system email is `Already in-use system email` (admin) vs `Already in-use as system email` (profile, per EC-031-007). (FS-031.13.)

---

## Dependencies

- **FS-001 / FS-090** — Admin shell, header/footer/nav (`admin.inc.php`, `staff.inc.php` bootstrap, `setTabActive`), and the `Pagenate` pagination component / `PAGE_LIMIT`.
- **FS-002** — Staff authentication, the administrator access gate, password-reset token issuance/expiry, login strike/lockout, session establishment, and **enforcement** of every group permission flag and the active/group-enabled/vacation availability rules. Group-disabled and admin-exemption enforcement live here.
- **FS-003** — `Validator` (email/phone/username), `Format` (HTML escaping, phone normalization, date/datetime rendering, `striptags`), `Misc` (random codes, GMT time), and `Signal` (model created/modified/deleted, auth password-change).
- **FS-030** — Department, team, and help-topic CRUD. Staff reference a primary department and team memberships; groups grant department access — the referenced departments/teams are defined there.
- **FS-040** — System email accounts (the source of the "already in-use system email" collision check) and the password-reset email template (`staff.pwreset`).
- **FS-091** — Canonical table schemas (`staff`, `groups`, `group_dept_access`, `team_member`, `timezone`) and all enum/value sets (account-type, status, signature type, paper size, etc.).

---

## Known Limitations

- **KL-031-001 — Single group per staff.** A staff member belongs to exactly one group; permissions cannot be composed from multiple groups, and there is no per-staff permission override independent of the group.
- **KL-031-002 — Directory numeric search SQL gap.** In the directory's numeric-term branch the phone/extension/mobile match clause is missing an `OR` operator between two conditions, so a numeric search does not reliably match the mobile field. Behavior is as observed in source; not corrected here.
- **KL-031-003 — Group list is unpaginated.** The group list renders all groups with no pagination; very large group counts would render in a single page.
- **KL-031-004 — MD5 password fallback.** Legacy accounts whose stored password is a bare MD5 hash are still accepted on login and transparently re-hashed (or forced to change) — a migration accommodation rather than an intended permanent behavior (login/crypto details owned by FS-002).
- **KL-031-005 — Mass-delete bypasses last-admin count check.** The mass-delete path relies only on self-exclusion, not the explicit last-active-admin count check used on the edit form; the explicit guard is form-only.
- **KL-031-006 — Admin notes are unstructured.** Both staff and group admin notes are free text with no formatting, history, or attribution beyond "viewable by all admins".
- **KL-031-007 — Password-age basis is approximate.** Password aging (FS-031.12) is computed from the stored reset timestamp (or account-creation time for never-reset accounts) compared against server time; the source itself flags a possible time-zone discrepancy in this comparison. The threshold uses a flat 30-day month, so "N months" is approximated as `N × 30` days rather than calendar months.
- **KL-031-008 — Reset-token expiry check is order-dependent on token freshness.** In the own-profile reset-token validation, the window comparison reads the token's last-modified time after asserting it is unreadable; the practical effect is that a valid, current token authorizes the change while an unreadable or over-age token is rejected. The behavior is as observed in source and is not corrected here (token lifecycle owned by FS-002).
- **KL-031-009 — Group form has no per-field "create vs update" banner distinction beyond title/button.** The group add/edit form distinguishes mode only by title (`Add New Group` / `Update Group`) and submit label (`Create Group` / `Save Changes`); both modes offer the same Reset and Cancel (returns to `groups.php`) controls.
