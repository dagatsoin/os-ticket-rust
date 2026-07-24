# TS-M4-F1 — add(route): FS-033.13/.14/.15/.16 page CRUD + in-use guards + content/config AJAX

- **ID**: TS-M4-F1
- **Type**: Technical Story
- **Parent**: US-M4-F1
- **Labels**: Technical Story, M4
- **Scope**: medium

## Context

Backend for site-page management + the content/config read endpoints (ticket-variable card,
`/config/scp` bundle). The in-use guard consults both the `*_page_id` config bindings and help-topic
references.

## Impact

- add(route): `GET /api/staff/admin/pages`, `POST`, `PUT /:id`, mass endpoint — admin-gated.
- add(service): FS-033.14 validation (name required + unique BS-033.10; type in
  landing|offline|thank-you|other); FS-033.16 single-page guards.
- add(service): BS-033.9 **in-use** = bound as landing/offline/thank-you (`config.*_page_id == page.id::text`,
  string compare — config values are text) **OR** referenced by `help_topic.page_id`. An in-use page
  refuses **both delete and disable**. On a **permitted delete** (page not in-use), reset any
  `help_topic.page_id` pointing at it back to **`0`** (the topic "no page" sentinel).
- add(route): **unauthenticated public** `GET /api/pages/:slug` — serves an active `type='other'`
  page, `slug` derived from the page name. Owned by this TS.
- add(route): `GET /api/staff/admin/content/ticket_variables` (FS-033.9, **admin-gated**),
  `GET /api/config/scp` (FS-033.10, **admin-gated per BS-033.1**).
- add(dev): `POST /api/dev/reset-pages` — **owned by this TS** (restores page rows + clears bindings
  cheaply for the bulk delete/disable ACs).

## Regressions

- KL-033.1 preserved (no versioning). The Pages settings tab (EPIC-M4-A) reads this page list.

## Acceptance Tests

### AC-1: FS-033.14 — create validation + uniqueness. [API-ONLY]
- Setup: reseed; admin session cookie (POST /api/staff/login admin/Admin123!).
- Request: POST /api/staff/admin/pages {type:"landing", body:"x"} (no name) → 422 name error.
- Request: POST /api/staff/admin/pages {name:"Dup", type:"other", body:"x"} then repeat the same name → 422 "already exists" (BS-033.10).
- Request: POST /api/staff/admin/pages {name:"Bad", type:"bogus", body:"x"} → 422 (type must be landing|offline|thank-you|other).
- Status: [x]

### AC-2: BS-033.9 — bound page is in-use; delete AND disable refused. [API-ONLY]
- Setup: admin session; seed a page (POST /api/dev/seed-page {name:"Land1", type:"landing"}).
- Request: bind it via config (set `landing_page_id` = that page id, as text — through the settings endpoint or POST /api/dev/seed-config); DELETE /api/staff/admin/pages/{id} → refused (in-use); mass **disable** it → also refused (in-use pages cannot be disabled).
- Request: seed a page and reference it from a help topic (`help_topic.page_id`); DELETE it → refused.
- Status: [x]

### AC-3: FS-033.15 — bulk enable/disable/delete of unbound pages; topic ref reset on delete. [API-ONLY]
- Setup: admin session; seed 2 unbound pages.
- Request: mass endpoint disable → both status=disabled; enable → status=active.
- Request: mass endpoint delete of the unbound pages → removed (GET no longer lists them).
- Verify: after deleting a page that a help topic had pointed at (once unbound from config), that topic's `page_id` is reset to `0` (the "no page" sentinel).
- Status: [x]

### AC-4: FS-033.9/.10 — content/config read endpoints respond (admin-gated). [API-ONLY]
- Setup: admin session; also an `agent` (non-admin) session.
- Request: GET http://localhost:3701/api/staff/admin/content/ticket_variables (admin) → 200 with the variable list; as `agent` → 403.
- Request: GET http://localhost:3701/api/config/scp (admin) → 200 with the staff-config bundle; as `agent` → 403 (admin-gated per BS-033.1).
- Status: [x]

### AC-5: public page endpoint serves an active `other` page by slug. [API-ONLY]
- Setup: seed an active `type='other'` page named e.g. "Terms Of Service" (POST /api/dev/seed-page).
- Request: GET http://localhost:3701/api/pages/{slug} (slug derived from the name) with **no session** → 200 with the page body.
- Verify: a non-`other` type, an inactive page, or an unknown slug → 404; the endpoint is public (no auth required).
- Status: [x]

## Test Infrastructure

- Admin account + `agent` (for the admin-gate 403 on content/config). Dev `POST /api/dev/seed-page` (exists), `POST /api/dev/seed-config` (bind config for the in-use test). `.sqlx` cache updated.
- **Owns `POST /api/dev/reset-pages`** (build it here) — restores page rows + clears bindings for the bulk delete/disable ACs; reseed remains a fallback.

## Dependencies

- **EPIC-M4-PREP**: `page` table + `*_page_id` keys. **TS-M4-A0**: admin gate.
