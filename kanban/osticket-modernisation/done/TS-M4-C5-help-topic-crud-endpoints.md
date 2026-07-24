# TS-M4-C5 — add(route): FS-030.15/.16/.17 help-topic CRUD + routing + one-level nesting + delete promote

- **ID**: TS-M4-C5
- **Type**: Technical Story
- **Parent**: US-M4-C3
- **Labels**: Technical Story, M4
- **Scope**: medium

## Context

Backend for help-topic management: list, create/edit with routing fields, one-level parent nesting,
delete-promotes-children, bulk actions.

## Impact

- add(route): `GET /api/staff/admin/help-topics` (paginated), `POST`, `PUT /:id`, bulk endpoint — admin-gated.
- add(route): **`GET /api/staff/admin/help-topics/form-options`** → `{priorities, departments, sla,
  pages:[thank-you only], staff, teams, parent_topics:[topic_pid=0 only]}` — the source for the topic
  form's selects. `pages` is filtered to thank-you pages only; `parent_topics` is filtered to
  top-level topics only (`topic_pid=0`) so the parent picker can never offer a child.
- add(service): BS-030-18 (text required ≥5 chars, unique-within-parent via `UNIQUE(topic,topic_pid)`),
  BS-030-19 (`dept_id` + `priority_id` required), BS-030-20 (ONE-LEVEL nesting enforced **write-side**:
  reject a `parentId` that points at a non-top-level topic — not merely a filtered picker),
  BS-030-22 (auto-assign encoded as `s<id>`=staff / `t<id>`=team / else both NULL — mutually exclusive),
  BS-030-23 (SLA `0` = no topic SLA; empty thank-you page = system default).
- add(service): delete — BS-030-26 (promote children to top-level `topic_pid=0`, clear
  `ticket.topic_id=0`; the FAQ-link clear is **defined-but-noop** — no `faq_topic` table yet).
- Topic-update has NO no-op guard (FS-030.15 — success even with no changes). add(dev): `reset-help-topics`.

## Regressions

- The M3 routing consumers (FS-011/021) keep reading topic fields; SLA precedence owned by FS-021.13.

## Acceptance Tests

### AC-1: BS-030-18/19 — validation. [API-ONLY]
- Setup: `cargo run -p tools --bin seed`; admin session cookie (`POST /api/staff/login` as `admin`/`Admin123!`).
- Request: `POST /api/staff/admin/help-topics` with 4-char text → Expect 422 "5 chars minimum".
- Request: `POST /api/staff/admin/help-topics` with a valid name but no dept → Expect 422 "You must select a department"; with no priority → Expect 422 "You must select a priority".
- Status: [x]

### AC-2: BS-030-20 — parent options top-level only AND write-side one-level enforcement. [API-ONLY]
- Setup: admin cookie; a seeded parent topic and a child topic.
- Request: `GET /api/staff/admin/help-topics/form-options` → Expect `parent_topics` contains only topics with `topic_pid=0` (no child topics offered).
- Request: `POST /api/staff/admin/help-topics` with `parentId` pointing at the existing **child** topic (a non-top-level parent) → Expect 422 (write-side rejection — one-level nesting enforced server-side, not just filtered in the picker).
- Status: [x]

### AC-3: BS-030-22 — auto-assign is staff XOR team (`s`/`t` encoding). [API-ONLY]
- Setup: admin cookie.
- Request: `POST /api/staff/admin/help-topics` encoding a staff auto-assignee as `s<id>` → Expect the staff assignee retained (team NULL) on GET.
- Request: `PUT` the same topic encoding a team as `t<id>` → Expect the team assignee retained and the staff cleared on GET; supplying neither leaves both NULL (mutually exclusive, per BS-030-22).
- Status: [x]

### AC-4: BS-030-23 — thank-you page + SLA override optional. [API-ONLY]
- Setup: admin cookie; a thank-you page id (`POST /api/dev/seed-page`).
- Request: `POST /api/staff/admin/help-topics` with a thank-you page id + SLA `0` → Expect 200 saved; SLA `0` persists as "no topic SLA" (empty thank-you page would mean system default).
- Status: [x]

### AC-5: BS-030-26 — delete promotes children + clears references. [API-ONLY]
- Setup: admin cookie; a parent topic with one child; a ticket referencing the parent (`seed-ticket`).
- Request: `DELETE /api/staff/admin/help-topics/:parentId` → Expect 200; the child's `topic_pid` is now `0` (promoted to top-level); the referencing ticket has `topic_id` cleared to `0`. (The FAQ-link clear is defined-but-noop — there is no `faq_topic` table yet.)
- Status: [x]

## Test Infrastructure

- Admin account; dev `reset-help-topics`. `.sqlx` cache updated. Requires SLA plans + a thank-you page.

## Dependencies

- **EPIC-M4-PREP**, **TS-M4-A0**, **EPIC-M4-B (staff/team)**, **EPIC-M4-D (SLA)**, **EPIC-M4-F (page)**.
