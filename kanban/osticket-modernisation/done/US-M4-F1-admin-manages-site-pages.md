# US-M4-F1 — Admin manages site pages

- **ID**: US-M4-F1
- **Type**: User Story
- **Parent**: EPIC-M4-F
- **Labels**: User Story, M4
- **Scope**: medium

## Spec References

- FS-033.11 (site pages list & access gate), FS-033.12 (results table, sorting & pagination)
- FS-033.13 (create & edit a site page), FS-033.14 (field validation & uniqueness)
- FS-033.15 (enable / disable / delete — bulk actions), FS-033.16 (single-page guards)
- BS-033.9 (default/bound pages are in-use and protected), BS-033.10 (unique page name/slug)
- KL-033.1 (no page versioning — PRESERVED), KL-033.2 (modernised)

## Context

Epic: EPIC-M4-F — Site Pages & Content. Pages produced here are consumed by EPIC-M4-A (landing/
offline/thank-you bindings, Integration AC-4) and EPIC-M4-C (topic thank-you page, Integration AC-5).
A bound page becomes in-use and delete-protected.

## Description

As an administrator, I can list site pages and create or edit one: name (unique), type
(landing / offline / thank-you / other), body, and active flag. I can enable/disable/delete pages in
bulk. A page bound as a default page (in System Settings) or referenced by a help topic is "in use"
and can be neither deleted nor disabled until unbound. Active `other` pages are also reachable
publicly by slug (`GET /api/pages/:slug`).

## Impact

- Frontend (page list + create/edit form + bulk actions)
- Backend (page CRUD + validation/uniqueness + in-use guards + content/config AJAX read)
- Database (`page`, `config`, `help_topic`)
- Browser (desktop)

## Business Rules

- BS-033.10: page name is unique (and the slug source for `other` public URLs — served by the unauthenticated `GET /api/pages/:slug`).
- BS-033.9: a page bound as landing/offline/thank-you (`config.*_page_id`) or referenced by a topic (`help_topic.page_id`) is in-use — **delete AND disable are refused**. A permitted delete resets any referencing `help_topic.page_id` back to `0`.
- FS-033.14: type is one of landing/offline/thank-you/other; name required + unique.
- BS-033.1: the content/config AJAX read endpoints are admin-gated.

## Regressions

- KL-033.1 preserved: no page version history — an edit overwrites in place.

## Acceptance Criteria

### AC-1: The site pages list renders with sort + pagination. [BROWSER]
- Setup: reseed (`cargo run -p tools --bin seed`); optionally seed a few pages — `POST http://localhost:3701/api/dev/seed-page` x N (so pagination is exercisable).
- Navigate: http://localhost:3702/staff/login → log in `admin`/`Admin123!` → /staff/admin/pages.
- Verify: a table lists pages with columns name, type, status, and an in-use indicator; column headers are sortable (click a header → order changes); pagination controls are present when rows exceed one page.
- Status: [x]

### AC-2: Admin creates a landing page and a thank-you page. [BROWSER]
- Setup: continue as admin at /staff/admin/pages.
- Action: click "Add Page"; Name "Welcome Landing", Type `landing`, Body "Welcome to support"; Save.
- Verify: success banner; "Welcome Landing" appears in the list.
- Action: click "Add Page" again; Name "Thanks Page", Type `thank-you`, Body "Thank you"; Save.
- Verify: both pages now appear in the list.
- Status: [x]

### AC-3: Duplicate page name is rejected. [BROWSER]
- Setup: continue as admin; "Welcome Landing" already exists (AC-2).
- Action: click "Add Page"; Name "Welcome Landing" (reused), Type `other`, Body "x"; Save.
- Verify: an inline "name already exists" error (BS-033.10) is shown; the page is not created.
- Status: [x]

### AC-4: A bound page is in-use and delete-protected. [BROWSER]
- Setup: continue as admin. Bind "Welcome Landing" as the default landing page — Navigate /staff/admin/settings → Pages tab (EPIC-M4-A, Integration AC-4) → set Landing Page = "Welcome Landing" → Save.
- Action: Navigate /staff/admin/pages; locate "Welcome Landing"; attempt to delete it (select checkbox + Delete, or its row delete control).
- Verify: the row shows an "in use" badge and BOTH delete and disable are refused/guarded with a message (BS-033.9); the page remains active.
- Status: [x]

### AC-5: Content/config AJAX read endpoints respond (admin-gated). [API-ONLY]
- Setup: admin session cookie (POST /api/staff/login admin/Admin123!); an `agent` session.
- Request: GET http://localhost:3701/api/staff/admin/content/ticket_variables → 200 (admin) with the ticket-variable reference list; as `agent` → 403.
- Request: GET http://localhost:3701/api/config/scp → 200 (admin) with the staff-config bundle; as `agent` → 403 (BS-033.1).
- Status: [x]

### AC-6: Public page endpoint serves an active `other` page by slug. [API-ONLY]
- Setup: seed an active `type='other'` page (POST /api/dev/seed-page) named e.g. "Terms Of Service".
- Request: GET http://localhost:3701/api/pages/{slug} (slug from the page name) with NO session → 200 with the body; an inactive/non-`other`/unknown slug → 404.
- Status: [x]

## Checklist (children)

- [ ] TS-M4-F1 — Backend: page CRUD + validation/uniqueness + in-use guards + content/config AJAX read
- [ ] TS-M4-F2 — Frontend: site pages list + create/edit form UI

## Test Infrastructure

- Admin account (`admin`/`Admin123!`). Existing dev endpoint: `POST /api/dev/seed-page` (insert/upsert a page row by name) — use it to pre-populate rows for the list/pagination AC-1.
- For AC-4, the Pages settings binding is done through the real EPIC-M4-A System Settings → Pages tab (already delivered).
- `POST /api/dev/reset-pages` is **built by TS-M4-F1** (restores page rows + clears bindings) — use it to reset page state cheaply between the bulk delete/disable ACs; reseed remains a fallback.

## Dependencies

- **EPIC-M4-PREP**: `page` table + `*_page_id` keys.
- **EPIC-M4-A**: default-page binding (in-use source). Consumed by **EPIC-M4-C** (topic thank-you page).
