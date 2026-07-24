# M4 — MILESTONE – Admin Configuration

- **ID**: M4
- **Type**: Milestone (root ticket)
- **Parent**: — (top of hierarchy)
- **Labels**: Milestone
- **Column**: **done** (2026-07-24) — all 9 child epics in **done**; root E2E complete: all 8 Integration ACs (`[BROWSER]`) are `[x]` (AC-7 re-tested green after the log_level fix).

## Spec References

- FS-030 (admin departments, teams, help topics)
- FS-031 (admin staff accounts, permission groups, staff directory, own profile)
- FS-032 (admin system settings ~110 keys, SLA plans, priorities, FAQ categories)
- FS-033 (admin system logs, site pages, content/config AJAX)
- FS-022 (canned-response admin surface — the M2-deferred CRUD UI)

## Description

Replaces M1's seed-only configuration with a real admin control panel. This milestone gives an
administrator a screen-driven way to shape the helpdesk: create and edit **departments, teams and
help topics** (routing), **staff accounts and permission groups** (authorization), the **seven-tab
system settings**, **SLA plans and priorities**, **FAQ categories**, **site pages**, and a
**system-log viewer** — plus the **canned-response CRUD UI** deferred out of M2.

Business value: the app stops depending on migration seeds and one-off SQL. Admins configure
routing, authorization and SLA behaviour from the browser; new staff can be provisioned and log
in; settings written in one screen take effect across the running app. This is the milestone that
makes the product self-serviceable by an operator instead of a developer.

## Scope

**In scope (M4)**:
- EPIC-M4-PREP — schema & seed expansion (dependency root for every other epic)
- EPIC-M4-A — System settings (7 tabs + Attachments) — FS-032.1–.7
- EPIC-M4-B — Staff, groups & permissions — FS-031
- EPIC-M4-C — Departments, teams & help topics (routing) — FS-030
- EPIC-M4-D — SLA plans & priorities (priorities read-only) — FS-032.8–.12
- EPIC-M4-E — FAQ categories (gated by `can_manage_faq`) — FS-032.13–.16
- EPIC-M4-F — Site pages & content — FS-033.9–.16
- EPIC-M4-G — System logs (viewer + minimal write-side logging + purge sweep fn) — FS-033.1–.8
- EPIC-M4-H — Canned-response CRUD UI (gated by `can_manage_premade`) — FS-022 admin surface

**Explicitly deferred (DEVIATION M4-D3 — the M3 runtime carry-overs move to M5, NOT M4)**:
- PDF print (FS-021.17) → M5
- CSV export (FS-020.10) → M5
- Dashboard statistics + activity chart (FS-020.12/.13) → M5
- Phone / staff-initiated ticket creation (FS-021.20) → M5
- Ban/unban requester email (FS-021.14) → M5 (needs banlist infra; the `can_ban_emails` group flag IS added in M4-B, but the ban action UI is M5)

> Note: department-manager status is small and IS included in EPIC-M4-B (staff `isadmin` + the
> department `manager` pointer + the effective-access rule), which unblocks the M3-deferred
> manager-gated state flags whenever M5 wires them. If it proves non-trivial during consolidation
> it may be trimmed to the pointer only — decision recorded on EPIC-M4-B.

## Deviations from legacy (pinned — see ROADMAP "Decisions (M4)")

- **M4-D1 (RISK-1) — Minimal email_account + template_group model, forward-ported into PREP.**
  The department Email/Template required selects (FS-030.4, BS-030-02/03) and the FS-032 Emails
  settings tab resolve against **real rows**, not system-default stubs. M5/FS-040 extends this model.
- **M4-D2 (RISK-3) — Minimal write-side logging facility built in EPIC-M4-G.** An
  `osTicket::log()`-equivalent records `syslog` rows on notable admin/system events so the FS-033
  log viewer shows real data. Full FS-003 logging is NOT required. The cron purge **trigger** is a
  noted M6 forward-dependency; the purge sweep **function** lands in M4-G, invoked by a dev/manual
  trigger for now.
- **M4-D3 — M3 runtime carry-overs deferred to M5, not M4** (list above).
- **KL treatment** — cosmetic/bug-class KLs are **modernised** (correct behaviour, each noted as a
  deliberate deviation): KL-030-11/12, KL-031-002, KL-032.3/.4/.9/.10/.11, KL-033.2/.7. Structural
  KLs are **preserved** as faithful 1.7 behaviour: one-level topic nesting (KL-030-02),
  single-group-per-staff (KL-031-001), fixed priority set (KL-032.1), no page versioning (KL-033.1).

## Acceptance Criteria (root E2E — browser-only journey, no [API-ONLY])

> One continuous admin browser journey exercising the cross-epic intersections. Validated by
> qa-criterion-tester against the running frontend once all children reach `qa`. Run reseed
> (`cargo run -p tools --bin seed --reset`) ONCE before AC-1; state created in earlier ACs is
> consumed by later ones. All ACs are `[BROWSER]` — the milestone root owns no `[API-ONLY]` ACs.

### AC-1 (Integration A→C/D): A department selects an SLA (from D) and a help topic selects a Priority (from D); the default department and default SLA are protected from deletion. [BROWSER]
- Setup: reseed (`cargo run -p tools --bin seed --reset`); log in as an admin at http://localhost:3702/staff/admin.
- Action: open Departments (http://localhost:3702/staff/admin/departments) → add/edit a department; the SLA select lists the SLA plans seeded/created in EPIC-M4-D. Select one and save.
- Verify: the department saves with the chosen SLA plan (A→C dept↔SLA, D→C SLA-on-dept).
- Action: attempt to delete the default department and the default SLA plan.
- Verify: both deletions are refused with the protection message — the default department's delete checkbox is disabled (and it cannot be made private) and the default SLA plan's delete is blocked.
- Action: open Help Topics (http://localhost:3702/staff/admin/help-topics) → add/edit a help topic; the Priority select lists the M4-D priority set. Select a priority and save.
- Verify: the help topic saves with the chosen Priority (D→C priority-on-topic — priority is a help-topic field, not a department field).
- Status: [x]

### AC-2 (Integration B→C): A staff account created in B is usable as a department manager, team lead and topic auto-assignee; a group created in B appears in the department access matrix. [BROWSER]
- Action: in Staff, create a new active staff account; in Groups, create a new group.
- Action: in Departments, the new staff appears in the Manager select; in Teams, in the Lead select; in Help Topics, in the auto-assign select.
- Verify: the new group appears as a checkbox row in the department's Department-Access matrix.
- Status: [x]

### AC-3 (Integration B↔C): A team member is added from B's staff profile; removing the staff clears the membership. [BROWSER]
- Action: open the new staff member's profile (B) and add them to a team.
- Verify: the Teams roster (C) shows that staff as a member.
- Action: delete/lock the staff member (respecting last-admin protection).
- Verify: the team roster no longer lists them.
- Status: [x]

### AC-4 (Integration A↔F): A page created in F is bindable as landing/offline/thank-you in A; binding it makes it in-use-protected in F. [BROWSER]
- Action: in Site Pages (F), create a page of type `landing`.
- Action: in System Settings → Pages tab (A), bind that page as the landing page and save.
- Verify: back in Site Pages (F), the page shows "in use" and its delete is refused/guarded.
- Status: [x]

### AC-5 (Integration F→C): A thank-you page is selectable on a help topic. [BROWSER]
- Action: in Site Pages (F), create a page of type `thank-you`.
- Action: in Help Topics (C), edit a topic and select that thank-you page.
- Verify: the topic saves with the thank-you page bound.
- Status: [x]

### AC-6 (Integration B→E): `can_manage_faq` on a NON-admin group grants FAQ-category access without settings access. [BROWSER]
- Action: in Groups (B), create a non-admin group with `can_manage_faq=Yes`; assign a staff member to it.
- Action: log in as that non-admin staff member.
- Verify: the FAQ Categories screen (E) is reachable and usable.
- Verify: the System Settings / admin-only screens are NOT reachable for that user.
- Status: [x]

### AC-7 (Integration A→wide): Page-size / login-window / default keys written in A take effect in B's pagination and password aging. [BROWSER]
- Action: in System Settings (A), change the default max page size and the password-reset period; save.
- Verify: the Staff list (B) paginates using the new page size.
- Verify: a staff account whose password age exceeds the new period surfaces the forced-change banner on next login (or the profile aging notice), demonstrating the setting took effect.
- Status: [x]

### AC-8 (Integration D1 gate): The department Email and Template required selects resolve against the minimal email_account / template_group model. [BROWSER]
- Action: in Departments (C), open the add/edit form.
- Verify: the Email select lists the seeded `email_account` row(s) and the Template select lists the seeded `template_group` value(s) (DEVIATION M4-D1) — not an empty/stub list.
- Verify: saving without an Email or Template selection is rejected with `Email selection required` / `Template selection required`.
- Status: [x]

## Children (column derived: min of these)

- [ ] EPIC-M4-PREP — Schema & seed expansion (blocker for all)
- [ ] EPIC-M4-A — System Settings (7 tabs + Attachments)
- [ ] EPIC-M4-B — Staff, Groups & Permissions
- [ ] EPIC-M4-C — Departments, Teams & Help Topics
- [ ] EPIC-M4-D — SLA Plans & Priorities
- [ ] EPIC-M4-E — FAQ Categories
- [ ] EPIC-M4-F — Site Pages & Content
- [ ] EPIC-M4-G — System Logs
- [ ] EPIC-M4-H — Canned Response CRUD UI

## Build order (encoded via child dependencies)

**PREP → A → (B ∥ D ∥ F ∥ G ∥ H) → C → E.** C is last among the routing objects (it consumes A,
B, D and F); E follows B's `can_manage_faq` gate. The shared admin-panel FE shell (TS-M4-A0) is a
dependency for every admin screen.

## Dependencies

- **M1/M2/M3 (done)**: ticket/thread/staff/session/auth, attachments, canned-response model, the
  partial department/sla/groups/group_dept_access/team/team_member/help_topic/priority/config schema.
- **EPIC-M4-PREP** blocks every other M4 epic (schema + seed).
- **M4-D1** minimal email_account/template_group model unblocks EPIC-M4-C department form + EPIC-M4-A Emails tab.
- **Forward**: M5 consumes M4's config/routing (email pipeline) and picks up the M4-D3 deferrals;
  M6 wires the cron purge trigger onto EPIC-M4-G's purge sweep function.
