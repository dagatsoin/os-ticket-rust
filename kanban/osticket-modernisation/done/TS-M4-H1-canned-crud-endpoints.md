# TS-M4-H1 — add(route): FS-022 canned-response CRUD + can_manage_premade gate

- **ID**: TS-M4-H1
- **Type**: Technical Story
- **Parent**: US-M4-H1
- **Labels**: Technical Story, M4
- **Scope**: small

## Context

Backend for canned-response management, gated by `can_manage_premade`. The model and consumption path
already exist from M2; this adds the write surface.

## Impact

- add(gate): a reusable **`require_admin_or_permission(state, session, perm)`** helper in
  `auth/gate.rs` — admits admins OR staff whose group carries `perm`. **Reused by EPIC-M4-E** (FAQ
  categories, `can_manage_faq`).
- add(route): `GET /api/staff/canned-responses`, `POST`, `PUT /:id`, `DELETE /:id`, mass endpoint —
  under `/api/staff/canned-responses`, gated by `require_admin_or_permission(_, _, can_manage_premade)`.
- add(service): validation (title required); optional department scope; body with `%{token}` (M2
  catalog); active/disabled flag.
- add(service): **attachments via MULTIPART upload** reusing the M2 `drain_multipart` helper + blob
  store + `canned_attachment` link. Create/edit accept `multipart/form-data`; **edit supports
  `keep_file_ids[]`** to retain existing attachments (omitted ids are unlinked).
- add(service): **dept-scope options readable under the premade gate** — provide a department-list
  endpoint reachable by `can_manage_premade` (OR include dept options in the canned list response) so
  the form's department dropdown is populated without the admin gate.
- The consumption endpoint (M2 `GET` for the reply dropdown) filters to active responses — reused, not rebuilt.
- add(dev): `POST /api/dev/reset-canned` — **owned by this TS** (restores the two M2 seeded responses).
- **extend `POST /api/dev/set-group-perm`** to also toggle **`can_manage_premade`** + **`can_manage_faq`**
  (one-call delegated-gate setup) — **owned by this TS** (currently only the ticket flags are covered).

## Regressions

- The M2 reply-composer dropdown fetch keeps working; disabling a response removes it from the dropdown.

## Acceptance Tests

### AC-1: gate — can_manage_premade allowed, others denied. [API-ONLY]
- Setup: reseed. Mint the delegated account `premade1`/`Premade123!` with a group carrying `can_manage_premade` — grant the flag via the extended `POST /api/dev/set-group-perm {group_id, can_manage_premade:true}` (one-call), or the M4-B admin group POST; then `POST /api/dev/seed-staff {username:"premade1", password:"Premade123!", group_id, isadmin:false}`. Sessions for `premade1`, `agent`, and `admin`.
- Request: GET http://localhost:3701/api/staff/canned-responses as `premade1` → 200 (via `require_admin_or_permission`); as `agent` (no flag) → 403; as `admin` → 200.
- Status: [x] — VERIFIED live: GET /api/staff/canned-responses → premade1 200, agent 403, admin 200.

### AC-2: FS-022 — create validation. [API-ONLY]
- Setup: premade-manager session.
- Request: POST /api/staff/canned-responses {body:"x"} (no title) → 422 with a `title` field error.
- Status: [x] — VERIFIED live: POST (multipart, response="x", no title) → 422 `{"error":{"message":"Title required","fields":{"title":"Title required"}}}`. (Endpoint is multipart-only; a JSON body returns 422 "Expected a multipart/form-data request".)

### AC-3: FS-022 — create with a token body + multipart attachment; edit retains via keep_file_ids. [API-ONLY]
- Setup: `premade1` session.
- Request: POST /api/staff/canned-responses as `multipart/form-data` with fields title="T", body="Hi %{ticket.name}" and a file part → created; GET /api/staff/canned-responses/{id} shows the attachment linked (`canned_attachment`), reusing the M2 `drain_multipart` + blob store.
- Request: PUT /api/staff/canned-responses/{id} (multipart) with `keep_file_ids[]` listing the existing attachment id → the attachment is retained; omitting it unlinks the attachment.
- Status: [x] — VERIFIED live: multipart POST (title, response="Hi %{ticket.name}", attachment part) → created id 269; GET shows attachment id 133 (canned-att.txt) linked. PUT with keep_file_ids=133 → attachment retained; PUT without keep_file_ids → attachments []. Also title-uniqueness: duplicate title POST → 422 keyed on `title`. (Multipart body field is `response`; file part must be named `attachment`; requires allow_attachments config on.)

### AC-5: dept-scope options readable under the premade gate. [API-ONLY]
- Setup: `premade1` session (can_manage_premade, non-admin).
- Request: fetch the department options the canned form needs — the dept-list endpoint (or the dept options embedded in the canned list response) → 200 for `premade1` (readable without the admin gate).
- Status: [x] — VERIFIED live: GET /api/staff/canned-responses/dept-options as premade1 → 200 with the dept list [Billing, External, Sales, Support].

### AC-6: dev set-group-perm toggles the delegated flags. [API-ONLY]
- Setup: admin session; a target group id.
- Request: POST /api/dev/set-group-perm {group_id, can_manage_premade:true, can_manage_faq:true} → 200; the group now carries both flags (a `premade1` in that group passes the canned gate).
- Status: [x] — VERIFIED live: POST /api/dev/set-group-perm {groupId:29, can_manage_premade:true, can_manage_faq:true} → 200; DB groups.group_id=29 → can_manage_premade=t, can_manage_faq=t. premade1 (group 29, non-admin) then passes the canned gate (200). (dev endpoint expects `groupId` camelCase for the id; accepts the snake_case flag aliases.)

### AC-4: disabling removes it from the active consumption list. [API-ONLY]
- Setup: premade-manager session; a created active response id.
- Request: PUT /api/staff/canned-responses/{id} {active:false}; then GET the M2 reply-dropdown consumption list (active-only) → the response is absent.
- Status: [x] — VERIFIED live: GET /api/staff/tickets/1/canned (M2 consumption) listed id 269 while enabled; after PUT isenabled=false → 269 absent from the active-only list. (mass enable/disable/delete + reset-canned baseline restore also confirmed.)

## Test Infrastructure

- Delegated account `premade1`/`Premade123!` (group carries `can_manage_premade`) + `agent` + `admin`. Reuses the M2 `drain_multipart` + blob store. `.sqlx` updated.
- **Owns `POST /api/dev/reset-canned`** (build it here) — restores the two M2 seeded responses cheaply between CRUD runs; reseed remains a fallback.
- **Owns extending `POST /api/dev/set-group-perm`** to toggle `can_manage_premade` + `can_manage_faq` (one-call delegated-gate setup, in addition to the existing ticket flags).

## Dependencies

- **M2 (done)**: canned model + `%{token}` catalog + blob store + consumption endpoint.
- **EPIC-M4-B**: `can_manage_premade` flag. **TS-M4-A0**: capability gate.
