# TS-M4-E1 — add(route): FS-032.13/.14/.16 faq_category CRUD + can_manage_faq gate + deletion cascade

- **ID**: TS-M4-E1
- **Type**: Technical Story
- **Parent**: US-M4-E1
- **Labels**: Technical Story, M4
- **Scope**: small

## Context

Backend for FAQ category management, gated by the `can_manage_faq` capability (NOT the admin gate) —
the first delegated capability in M4.

## Impact

- add(route): `GET /api/staff/faq-categories` (list + `GET /:id`), `POST`, `PUT /:id`, mass endpoint —
  under `/api/staff/faq-categories` (**NOT** `/admin`), each gated by
  `require_admin_or_permission(can_manage_faq)` (admins included; non-admins allowed if flagged).
- add(service): fields — `name` (required), `ispublic` (public/internal), `description`, `notes`.
  FS-032.14 validation (name required); KL-032.11 — `GET /:id` on a missing id returns 404 (no hollow object).
- add(service): FS-032.16 delete is **plain** — no `faq` articles exist yet (M7), so the documented
  cascade to contained FAQs' associations is **defined-but-inert** (a category with no articles
  deletes cleanly; the cascade wiring lands with M7 articles).
- add(dev): `POST /api/dev/reset-faq-categories`.

## Regressions

- The gate is capability-based; it must NOT grant any settings/admin route to a non-admin.

## Acceptance Tests

### AC-1: FS-032.15 — gate allows can_manage_faq, denies others. [API-ONLY]
- Setup: `cargo run -p tools --bin seed`; create `faqmgr` — `POST /api/dev/seed-staff {"username":"faqmgr","password":"Faqmgr123!"}` → `groupId`; `POST /api/dev/set-group-perm {"group_id":<groupId>,"can_manage_faq":true}`.
- Request: `GET /api/staff/faq-categories` with the `faqmgr` session cookie → Expect 200; with the `agent` cookie (no flag) → Expect 403; with the `admin` cookie → Expect 200.
- Status: [x]

### AC-2: FS-032.14 — create validation. [API-ONLY]
- Setup: `faqmgr` (or admin) session cookie.
- Request: `POST /api/staff/faq-categories` without a name → Expect 422 with a name field error.
- Status: [x]

### AC-3: FS-032.16 — plain delete (cascade inert until M7). [API-ONLY]
- Setup: `faqmgr` cookie; a category created via POST.
- Request: `DELETE /api/staff/faq-categories/:id` → Expect 200; the category is gone. No `faq` articles exist yet so the documented association cascade is defined-but-inert (a no-article category deletes cleanly).
- Status: [x]

### AC-4: KL-032.11 — missing id returns 404, not a hollow object. [API-ONLY]
- Setup: `faqmgr` (or admin) cookie.
- Request: `GET /api/staff/faq-categories/999999` → Expect 404; `PUT /api/staff/faq-categories/999999` → Expect 404 (no hollow object).
- Status: [x]

## Test Infrastructure

- A non-admin `can_manage_faq` account + `agent` + `admin`. Dev `reset-faq-categories`. `.sqlx` updated.

## Dependencies

- **EPIC-M4-PREP**: `faq_category` table. **EPIC-M4-B**: `can_manage_faq` flag + a carrying group.
