# TS-M4-E2 — add(frontend): FS-032.13/.14 FAQ category list + form UI

- **ID**: TS-M4-E2
- **Type**: Technical Story
- **Parent**: US-M4-E1
- **Labels**: Technical Story, M4
- **Scope**: small

## Context

The FAQ category screen inside the admin shell, reachable via the `can_manage_faq` capability gate
(TS-M4-A0), wired to TS-M4-E1.

## Impact

- add(frontend): `FaqCategoryListPage` — table (name, type, description) + mass actions.
- add(frontend): `FaqCategoryFormDialog` — name, type (public/internal), description, notes; inline 422 + success.
- add(store): **`FaqCategoryStore`** (MobX, TDD) — base path `/api/staff/faq-categories`.
- route: `/staff/admin/faq-categories` wrapped in **`RequireCapability("can_manage_faq")`** (NOT the
  admin gate). The FAQ nav entry appears for any staff with `can_manage_faq`, even without admin access.

## Regressions

- None; additive; must not expose settings nav to a non-admin FAQ manager.

## Acceptance Tests

### AC-1: Integration AC-6 — a non-admin FAQ manager sees the FAQ screen, not settings. [BROWSER]
- Setup: create `faqmgr` — `POST /api/dev/seed-staff {"username":"faqmgr","password":"Faqmgr123!"}` → `groupId`; `POST /api/dev/set-group-perm {"group_id":<groupId>,"can_manage_faq":true}`.
- Setup: log in as `faqmgr` / `Faqmgr123!` at http://localhost:3702/staff/login.
- Navigate: http://localhost:3702/staff/admin/faq-categories
- Verify: the FAQ Categories screen renders; the "System Settings" (and other admin-only) nav entries are absent, and a direct hit on an admin-only route is blocked/redirected.
- Status: [x]

### AC-2: FS-032.14 — create a category. [BROWSER]
- Setup: `faqmgr` on /staff/admin/faq-categories.
- Action: click "Add Category"; type a name; select a type (public); type a description; click Save.
- Verify: success banner; the category appears in the list.
- Status: [x]

### AC-3: FS-032.13 — edit + delete from the list. [BROWSER]
- Setup: `faqmgr` on /staff/admin/faq-categories with a category present.
- Action: edit its type (public → private); click Save → the list reflects the change.
- Action: delete the category; confirm in the dialog → the category is gone from the list.
- Status: [x]

## Test Infrastructure

- Vitest + RTL + MSW mocking `/api/staff/faq-categories*`. A can_manage_faq non-admin account.

## Dependencies

- **TS-M4-A0**: shell + capability gate. **TS-M4-E1**: FAQ endpoints. **EPIC-M4-B**: the flag/group.
- **Router**: adds `/staff/admin/faq-categories` (under `RequireCapability("can_manage_faq")`) to the
  shared admin `<Route>` block in `router.tsx` — coordinate the single coherent router edit with
  C2/C4/C6 (all C/E screens touch the same block; FAQ differs only in its capability gate).
