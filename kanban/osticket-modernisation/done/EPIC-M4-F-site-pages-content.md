# EPIC-M4-F — EPIC – Site Pages & Content

- **ID**: EPIC-M4-F
- **Type**: Epic
- **Parent**: M4
- **Labels**: Epic, M4
- **Column**: derived from children

## Spec References

- FS-033.9 (ticket-variable reference card — content AJAX)
- FS-033.10 (staff configuration bundle — config AJAX `/config/scp`)
- FS-033.11 (site pages list & access gate)
- FS-033.12 (site pages results table, sorting & pagination)
- FS-033.13 (create & edit a site page)
- FS-033.14 (site page field validation & uniqueness)
- FS-033.15 (enable / disable / delete pages — bulk actions)
- FS-033.16 (single-page enable/disable/delete guards — entity rules)
- BS-033.9 (default/bound pages are in-use and protected), BS-033.10 (unique page name/slug)

## Context

Milestone: M4 — Admin Configuration. Produces the `page` rows consumed by EPIC-M4-A (landing/offline/
thank-you bindings, Integration AC-4) and EPIC-M4-C (topic thank-you page, Integration AC-5). Also
lands the content/config AJAX read endpoints that back the ticket-variable card and the SCP config
bundle. KL-033.1 (no page versioning) preserved.

## Description

- **Site pages**: list with sort/pagination; create/edit (name unique, type
  landing/offline/thank-you/other, body, active); validation & uniqueness (BS-033.10);
  enable/disable/delete bulk actions with entity guards — a page bound as a default page or referenced
  by a help topic is **in-use** and protected from deletion (BS-033.9). KL-033.2 modernised.
- **Content/config AJAX**: read-only endpoints — the ticket-variable reference card and the
  `/config/scp` staff-configuration bundle.

## Business value

Operators author the public-facing pages (landing, offline, thank-you) and bind them where they
apply. The in-use protection prevents breaking a live binding, and the content endpoints give the
admin UI its variable reference and config bootstrap.

## Acceptance Criteria

> No ACs — intermediate parent; verification owned by leaf tickets. Column derived from children.

## Children (column derived: min of these)

- [ ] [US-M4-F1](US-M4-F1-admin-manages-site-pages.md) — Admin manages site pages
  - [ ] [TS-M4-F1](TS-M4-F1-page-crud-content-endpoints.md) — Backend: page CRUD + validation/uniqueness + in-use guards + content/config AJAX read
  - [ ] [TS-M4-F2](TS-M4-F2-page-list-form-ui.md) — Frontend: site pages list + create/edit form UI

## Dependencies

- **EPIC-M4-PREP**: `page` table + `*_page_id` config keys.
- **EPIC-M4-A (shell)**: admin screens mount inside TS-M4-A0; Pages settings tab binds these rows.
- Blocks: EPIC-M4-C (topic thank-you page), EPIC-M4-A Pages tab (default-page bindings).
