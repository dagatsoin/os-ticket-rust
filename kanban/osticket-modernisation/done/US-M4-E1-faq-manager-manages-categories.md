# US-M4-E1 — FAQ manager manages FAQ categories

- **ID**: US-M4-E1
- **Type**: User Story
- **Parent**: EPIC-M4-E
- **Labels**: User Story, M4
- **Scope**: small

## Spec References

- FS-032.13 (FAQ category listing & mass actions)
- FS-032.14 (FAQ category create / edit form)
- FS-032.15 (FAQ category access control — `can_manage_faq`, NOT admin gate)
- FS-032.16 (FAQ category deletion cascade)
- BS-031-020 (`can_manage_faq` flag)
- KL-032.11 (`Category::lookup` row verification — modernised)

## Context

Epic: EPIC-M4-E — FAQ Categories. The first delegated, non-admin capability. A staff member whose
group has `can_manage_faq` can manage FAQ categories WITHOUT any system-settings/admin access —
proving the group-flag authorization model (Integration AC-6). FAQ articles are M7; this is the
category taxonomy only.

## Description

As a staff member with the "Can Manage FAQ" permission, I can list FAQ categories, create or edit one
(name, type public/internal, description, notes), and delete one. I can do this even though I have no
access to system settings or other admin screens. (Deletion is plain for now — FAQ articles arrive in
M7, so the documented cascade to contained FAQs' associations is defined-but-inert.)

## Impact

- Frontend (FAQ category list + form, reachable via the FAQ capability gate)
- Backend (faq_category CRUD + `can_manage_faq` gate + deletion cascade)
- Database (`faq_category`)
- Browser (desktop)

## Business Rules

- FS-032.15: access is gated by `can_manage_faq` (admins included), NOT the admin gate.
- FS-032.16: deleting a category is plain for now (a no-article category deletes cleanly); the documented cascade to contained FAQs' associations is defined-but-inert until M7 articles exist.
- KL-032.11 modernised: a missing category id yields a clean 404, not a hollow object.

## Regressions

- No settings/admin screens become reachable to a non-admin FAQ manager (Integration AC-6 negative check).

## Acceptance Criteria

### AC-1: A non-admin FAQ manager reaches the FAQ Categories screen (and NOT settings). [BROWSER]
- Setup: create the delegated `faqmgr` account — `POST /api/dev/seed-staff` `{"username":"faqmgr","password":"Faqmgr123!"}` → returns `{staffId, groupId}`; then `POST /api/dev/set-group-perm` `{"group_id":<groupId>,"can_manage_faq":true}` (that group carries NO admin/settings flags).
- Setup: `POST /api/dev/reset-faq-categories` for clean state; log out; log in as `faqmgr` / `Faqmgr123!` at http://localhost:3702/staff/login.
- Navigate: http://localhost:3702/staff/admin/faq-categories
- Verify: the FAQ Categories screen renders; the "System Settings" and other admin-only nav entries are absent, and navigating directly to an admin-only route (e.g. /staff/admin/settings/attachments) is blocked/redirected (Integration AC-6).
- Status: [x]

### AC-2: The FAQ manager creates a category. [BROWSER]
- Setup: logged in as `faqmgr` on /staff/admin/faq-categories (from AC-1).
- Action: click "Add Category"; type name "Getting Started"; select type Public; type a description; click Save.
- Verify: success banner; "Getting Started" appears in the category list.
- Status: [x]

### AC-3: The FAQ manager edits a category's type. [BROWSER]
- Setup: `faqmgr` on /staff/admin/faq-categories with a category present (from AC-2).
- Action: edit "Getting Started"; switch its type Public → Private; click Save.
- Verify: success; the list reflects the category as Private (Internal).
- Status: [x]

### AC-4: Deleting a category removes it (plain delete). [BROWSER]
- Setup: `faqmgr` on /staff/admin/faq-categories with a category present.
- Action: delete "Getting Started"; confirm in the confirmation dialog.
- Verify: success; the category is gone from the list. (FS-032.16 cascade is defined-but-inert — no FAQ articles exist until M7.)
- Status: [x]

### AC-5: A staff member WITHOUT the flag is denied. [API-ONLY]
- Setup: `agent` session cookie (`POST /api/staff/login` as `agent`/`Agent123!` — no `can_manage_faq`).
- Request: `GET /api/staff/faq-categories`.
- Headers: `agent` session cookie.
- Expect: 403 (FS-032.15 delegated gate denies a non-flagged staff).
- Status: [x]

## Checklist (children)

- [ ] TS-M4-E1 — Backend: faq_category CRUD + `can_manage_faq` gate + deletion cascade
- [ ] TS-M4-E2 — Frontend: FAQ category list + form UI (reachable without settings access)

## Test Infrastructure

- The delegated `faqmgr` account: `POST /api/dev/seed-staff` (returns `groupId`) + `POST /api/dev/set-group-perm {group_id, can_manage_faq:true}`. Both endpoints already exist (set-group-perm supports the `can_manage_faq` alias). Plus `agent` (no flag) and `admin` for the gate matrix.
- Dev **`POST /api/dev/reset-faq-categories`** — declared as a deliverable of TS-M4-E1 (`add(dev)`); provided by the dev iteration, no consolidation gap.

## Dependencies

- **EPIC-M4-PREP**: `faq_category` table.
- **EPIC-M4-B**: the `can_manage_faq` flag + a group/staff carrying it (Integration AC-6).
- **EPIC-M4-A (shell)**: FAQ screen reachable via a capability (not admin) gate.
