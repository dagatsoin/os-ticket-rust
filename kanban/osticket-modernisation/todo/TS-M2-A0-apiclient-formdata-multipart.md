# TS-M2-A0 — update(frontend): apiClient FormData/multipart support

- **ID**: TS-M2-A0
- **Type**: Technical Story
- **Parent**: EPIC-M2-A
- **Labels**: Technical Story, M2
- **Scope**: small

## Context

Every M2 frontend upload path (the `/open` file input, the staff reply composer with own-file +
canned `cannedId`) submits `multipart/form-data`, but the M1 apiClient
(`frontend/src/api/apiClient.ts`, owned by TS-M1-A5) unconditionally `JSON.stringify`s the body and
sets `Content-Type: application/json`. This shared enabler teaches the apiClient to pass a `FormData`
body through untouched so the browser sets the multipart boundary, while keeping the M1 cross-cutting
behaviour (CSRF header, error-envelope parsing, 401 → realm-login redirect) intact. Built once, ahead
of the UI tickets that depend on it. See ROADMAP **Decisions (M2) §13** and §2 (multipart strategy).

## Impact

- update(frontend): in `frontend/src/api/apiClient.ts`, when `body instanceof FormData` → do **not**
  `JSON.stringify` and do **not** set a `Content-Type` header (let the browser set the multipart
  boundary); a non-FormData body keeps the M1 JSON path unchanged.
- update(frontend): preserve the M1 cross-cutting behaviour on the multipart path — inject the
  `X-CSRFToken` header (per ROADMAP M1 Decisions §2), parse the shared error envelope (M1 §4), and
  perform the 401 → realm-login redirect (M1 §6).
- update(test harness): extend the MSW mock-API harness (TS-M1-A5) with **multipart-capable
  handlers** that read `await request.formData()`, so B3/C4/D2-style component tests can assert on
  uploaded parts. **TDD** — write the FormData/multipart handler tests first.

## Regressions

- Existing JSON requests (all M1 calls) must behave exactly as before — same headers, same envelope
  parsing, same 401 redirect. Re-run the apiClient unit tests + any M1 component test that exercises it.

## Acceptance Tests

> Frontend Vitest + MSW unit tests against `frontend/src/api/apiClient.ts` (TDD). No browser, no live
> backend. Run: `npm --prefix frontend test -- apiClient`. These are [API-ONLY] because A0 is a pure
> client-plumbing enabler with no user-facing surface — its UI consumers (A4, D3) carry the [BROWSER] ACs.

### AC-1: a FormData body is sent without JSON.stringify and without a Content-Type header. [API-ONLY]
- Run: call `apiClient(...)` with a `FormData` body; intercept the outgoing request in MSW.
- Verify: the request body is the raw FormData (not a JSON string) and apiClient sets **no** `Content-Type` header (browser-provided multipart boundary).
- Status: [ ]

### AC-2: a JSON body still serializes with application/json (M1 path unchanged). [API-ONLY]
- Run: call `apiClient(...)` with a plain-object body.
- Verify: the body is `JSON.stringify`d and `Content-Type: application/json` is set — the M1 path is unchanged.
- Status: [ ]

### AC-3: the CSRF header, error-envelope parsing, and 401 redirect are preserved on the FormData path. [API-ONLY]
- Run: a mutating FormData request; separately mock a 422 envelope response and a 401 response.
- Verify: the request injects `X-CSRFToken`; the 422 envelope is parsed into field errors; the 401 triggers the realm-login redirect — identical to the JSON path.
- Status: [ ]

### AC-4: the MSW harness can assert on multipart parts. [API-ONLY]
- Run: a multipart-capable MSW handler reads `await request.formData()`.
- Verify: the handler can assert on individual parts (text fields + a file part), enabling the A4/D3 component tests.
- Status: [ ]

## Test Infrastructure

- Extends the existing MSW mock-API harness (TS-M1-A5) — multipart-capable handlers consumed by A4 and D3.

## Dependencies

- TS-M1-A5 (apiClient + MSW harness).

## Blocks

- TS-M2-A4 (/open file input), TS-M2-D3 (reply composer — canned dropdown + file input, incl. the D4
  reply path), and the D3-driven multipart POST.
