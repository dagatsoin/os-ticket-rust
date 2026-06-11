# TS-M1-B3 — add(web): BS-011 "Open a New Ticket" form + confirmation UI

- **ID**: TS-M1-B3
- **Type**: Technical Story
- **Parent**: US-M1-2
- **Labels**: Technical Story, M1
- **Scope**: small

## Context

The public-facing UI for ticket submission. Renders the form, performs client-side validation,
posts to `POST /api/tickets`, and shows the confirmation page with the ticket number.

**M1 field list = name / email / subject / message** — simplified vs FS-011.2 (no help topic,
no CAPTCHA, no attachments; all deferred to later milestones). The form **MUST render backend
**422** field errors mapped onto the corresponding fields** (the backend is authoritative — not
only client-side validation). The **MobX form store** (validation state, submit gating, 422 →
field mapping) is a **TDD target**.

## Impact

- add(web): public route + page "Open a New Ticket" with name, email, subject, message (MUI form + MobX form state).
- add(web): inline validation (required + email) mirroring backend rules; disabled submit until valid.
- add(web): confirmation page rendering the returned ticket number.

## Regressions

- None (greenfield).

## Acceptance Tests

> Component/unit tests run via Vitest + RTL with a mocked apiClient (MSW) — `npm test`. No live
> backend, no browser interaction → all [API-ONLY]. (Browser-level submission is covered by
> US-M1-2 AC-1/AC-2/AC-3 and the M1 root E2E.)

### AC-1: Component — the form shows validation errors for empty required fields and invalid email (mocked client). [API-ONLY]
- Request: `npm test` — the form component test renders the form, leaves required fields empty / enters an invalid email, and asserts inline errors appear and submit is gated.
- Expect: validation errors shown; submit disabled until valid.
- Status: [ ]

### AC-2: Component — a backend 422 response maps each field error onto its field (mocked client). [API-ONLY]
- Request: `npm test` — the mocked apiClient returns a 422 envelope with `fields`; assert each field error is rendered on its corresponding field.
- Expect: server field errors mapped onto fields (backend authoritative).
- Status: [ ]

### AC-3: Component — a successful submit renders the confirmation with the returned ticket number (mocked client). [API-ONLY]
- Request: `npm test` — the mocked apiClient returns 201 + a ticket number; assert the confirmation view renders that number.
- Expect: confirmation shows the returned number.
- Status: [ ]

### AC-4: The form-store validation / submit-gating / 422-mapping logic is covered by unit tests (TDD target). [API-ONLY]
- Request: `npm test` — the MobX form-store unit tests cover validation state, submit gating, and 422→field mapping in isolation.
- Expect: form-store unit tests green.
- Status: [ ]

## Test Infrastructure

- Backed by the real `POST /api/tickets` route for the browser E2E; mockable for component tests.

## Dependencies

- TS-M1-A5 (scaffold), TS-M1-B2 (route).
