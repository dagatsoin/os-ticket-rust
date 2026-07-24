# TS-M4-H2 — add(frontend): FS-022 canned-response list + create/edit form UI

- **ID**: TS-M4-H2
- **Type**: Technical Story
- **Parent**: US-M4-H1
- **Labels**: Technical Story, M4
- **Scope**: small

## Context

The canned-response screen inside the admin shell, reachable via the `can_manage_premade` capability
gate (TS-M4-A0), wired to TS-M4-H1.

## Impact

- add(frontend): `CannedListPage` — table (title, department, status) + mass actions.
- add(frontend): `CannedFormDialog` — title, optional department scope (dropdown populated from the
  premade-gated dept options endpoint / canned list response — NOT the admin gate), body textarea
  (with a token reference hint reusing the M2 catalog), attachment add/remove submitted as
  **multipart** (reusing the M2 AttachmentChip + `validateAttachment`; edit sends `keep_file_ids[]`
  for retained attachments), active flag; inline 422 + success.

## Regressions

- None; additive. Must not expose settings nav to a non-admin premade manager.

## Acceptance Tests

### AC-1: gate — a premade manager sees Canned Responses, not settings. [BROWSER]
- Setup: reseed; mint a can_manage_premade non-admin (admin `POST /api/staff/admin/groups {can_manage_premade:true}` + `POST /api/dev/seed-staff {username:"premade1", password:"Premade123!", group_id}`).
- Navigate: http://localhost:3702/staff/login → log in `premade1`/`Premade123!` → /staff/admin/canned.
- Verify: Canned Responses is present/reachable; System Settings (and other admin-only nav) is absent for this non-admin.
- Status: [x] — VERIFIED in-browser: premade1 (non-admin) → /staff/admin/canned renders with the canned list; the admin sidebar shows ONLY delegated entries (Canned Responses + FAQ Categories); System Settings and all other admin-only nav are absent.

### AC-2: FS-022 — create a response with a token body. [BROWSER]
- Setup: continue as `premade1` at /staff/admin/canned.
- Action: Add Canned Response; Title "Greeting", Body "Hello %{ticket.name}", Active; Save.
- Verify: success banner; "Greeting" appears in the list.
- Status: [x] — VERIFIED in-browser as premade1: Add Canned Response → Title "Greeting", Body "Hello %{ticket.name}" (token-hint shown), Enabled, Department Support → Save → "Greeting added successfully" banner; "Greeting" appears in the list.

### AC-3: FS-022 — edit + disable; disabled leaves the reply dropdown. [BROWSER]
- Setup: continue as `premade1`; an open ticket exists (seed if needed).
- Action: edit "Greeting" body → Save (success). Then disable it.
- Verify: open a ticket's reply composer → "Greeting" is absent from the canned-response dropdown once disabled.
- Status: [x] — VERIFIED in-browser: as premade1 edited "Greeting" body + Save (success banner), then disabled it (Enabled unchecked → Status Disabled). Opened seeded ticket #953069's reply composer → the Canned response dropdown no longer lists "Greeting" (only None / Acknowledge receipt / Sample). Before disabling, the same dropdown DID list Greeting (see US-M4-H1 AC-3).

## Test Infrastructure

- Vitest + RTL + MSW mocking `/api/staff/canned-responses*`. A can_manage_premade non-admin account.
- Reuses the M2 AttachmentChip shell + `validateAttachment` helper.

## Dependencies

- **TS-M4-A0**: shell + capability gate. **TS-M4-H1**: canned endpoints. **M2**: AttachmentChip, token catalog, reply composer.
