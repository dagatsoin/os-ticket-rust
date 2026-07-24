# TS-M3-B3 — update(route): FS-020.6 pagination with limit param + total count

- **ID**: TS-M3-B3
- **Type**: Technical Story
- **Parent**: US-M3-B1
- **Labels**: Technical Story, M3
- **Scope**: small

## Context

Adds pagination to the queue route: accepts `p` (page number) and `limit` (page size) params,
applies LIMIT/OFFSET to the query, and returns total count for pagination controls. Implements
FS-020.6 pagination and FS-090.19 page-size resolution.

## Impact

- update(route): `GET /api/staff/tickets?status={status}&p={page}&limit={size}`
  - `p`: page number (1-based), default 1.
  - `limit`: page size override for this request; default uses resolved page size.
  - Response includes pagination metadata: `{ tickets: [...], pagination: { page, pageSize, totalCount, totalPages } }`.
- add(logic): page-size resolution per FS-090.19:
  1. If `limit` param is present and numeric: use it (clamped to 5-100 range).
  2. Else if staff has personal page limit configured: use it.
  3. Else if system `default_page_size` is configured: use it.
  4. Else: default to 25.
- update(query): apply `LIMIT {pageSize} OFFSET {(page-1) * pageSize}` to the ticket query.
- add(query): count query to get total matching tickets for pagination metadata.

## Page Size Constraints

- Minimum: 5
- Maximum: 100 (reasonable upper bound to prevent memory issues)
- If `limit` param exceeds max or is below min, clamp to bounds.
- The FS-090 spec mentions 5-50 step 5 for the configurable range; we allow freeform 5-100 for
  API flexibility but UI dropdowns can use the step-5 options.

## Response Shape

```json
{
  "tickets": [...],
  "pagination": {
    "page": 1,
    "pageSize": 25,
    "totalCount": 127,
    "totalPages": 6
  }
}
```

## Regressions

- The route without pagination params must work (page 1, default page size).
- Sort and status params must be preserved in pagination (tested by TS-M3-B1 + this ticket).
- Visibility scoping must be applied to both the listing and the count queries.

## Acceptance Tests

### AC-1: FS-020.6 — route returns pagination metadata. [API-ONLY]
- Setup: POST /api/dev/seed-tickets {"count":30,"status":"open"} -> {ids}
- Setup: POST /api/auth/login {"username":"agent","password":"Agent123!"} -> {session_cookie}
- Request: GET http://localhost:3701/api/staff/tickets?status=open
- Headers: Cookie: {session_cookie}
- Expect: 200, response has `pagination` object with keys: `page`, `pageSize`, `totalCount`, `totalPages`
- Status: [x]

### AC-2: FS-020.6 — default page size is 25 when no config or param. [API-ONLY]
- Setup: POST /api/dev/reset-config (clear page size config)
- Setup: POST /api/dev/seed-tickets {"count":30,"status":"open"} -> {ids}
- Request: GET http://localhost:3701/api/staff/tickets?status=open
- Headers: Cookie: {session_cookie}
- Expect: 200, `pagination.pageSize` == 25, `tickets` array length == 25, `totalCount` == 30, `totalPages` == 2
- Status: [x]

### AC-3: FS-020.6 — p param selects the page. [API-ONLY]
- Depends: AC-2 (30 tickets exist)
- Request: GET http://localhost:3701/api/staff/tickets?status=open&p=2
- Headers: Cookie: {session_cookie}
- Expect: 200, `pagination.page` == 2, `tickets` array length == 5 (remaining after page 1)
- Status: [x]

### AC-4: FS-020.6 — limit param overrides page size. [API-ONLY]
- Depends: AC-2 (30 tickets exist)
- Request: GET http://localhost:3701/api/staff/tickets?status=open&limit=10
- Headers: Cookie: {session_cookie}
- Expect: 200, `pagination.pageSize` == 10, `tickets` array length == 10, `totalPages` == 3
- Status: [x]

### AC-5: FS-020.6 — limit is clamped to min 5. [API-ONLY]
- Request: GET http://localhost:3701/api/staff/tickets?status=open&limit=2
- Headers: Cookie: {session_cookie}
- Expect: 200, `pagination.pageSize` == 5 (clamped up from 2)
- Status: [x]

### AC-6: FS-020.6 — limit is clamped to max 100. [API-ONLY]
- Request: GET http://localhost:3701/api/staff/tickets?status=open&limit=500
- Headers: Cookie: {session_cookie}
- Expect: 200, `pagination.pageSize` == 100 (clamped down from 500)
- Status: [x]

### AC-7: FS-090.19 — staff personal page limit is used when set. [API-ONLY]
- Setup: POST /api/dev/set-staff-config {"staff_id":1,"page_limit":15}
- Request: GET http://localhost:3701/api/staff/tickets?status=open (no limit param)
- Headers: Cookie: {session_cookie}
- Expect: 200, `pagination.pageSize` == 15
- Status: [x]

### AC-8: FS-090.19 — system default_page_size is used when no personal limit. [API-ONLY]
- Setup: POST /api/dev/set-staff-config {"staff_id":1,"page_limit":null} (clear personal)
- Setup: POST /api/dev/set-config {"key":"default_page_size","value":"20"}
- Request: GET http://localhost:3701/api/staff/tickets?status=open (no limit param)
- Headers: Cookie: {session_cookie}
- Expect: 200, `pagination.pageSize` == 20
- Status: [x]

### AC-9: FS-020.6 — page beyond total pages returns empty tickets array. [API-ONLY]
- Setup: POST /api/dev/seed-tickets {"count":10,"status":"open"} -> {ids} (page size 25 = 1 page)
- Request: GET http://localhost:3701/api/staff/tickets?status=open&p=5
- Headers: Cookie: {session_cookie}
- Expect: 200, `tickets` == [], `pagination.page` == 5, `pagination.totalPages` == 1
- Status: [x]

### AC-10: FS-020.6 — invalid p param defaults to page 1. [API-ONLY]
- Request: GET http://localhost:3701/api/staff/tickets?status=open&p=abc
- Headers: Cookie: {session_cookie}
- Expect: 200, `pagination.page` == 1
- Status: [x]

### AC-11: FS-020.6 — p=0 or negative defaults to page 1. [API-ONLY]
- Request: GET http://localhost:3701/api/staff/tickets?status=open&p=0
- Headers: Cookie: {session_cookie}
- Expect: 200, `pagination.page` == 1
- Request: GET http://localhost:3701/api/staff/tickets?status=open&p=-3
- Headers: Cookie: {session_cookie}
- Expect: 200, `pagination.page` == 1
- Status: [x]

### AC-12: Total count respects visibility scoping. [API-ONLY]
- Setup: POST /api/dev/seed-dept {"name":"External"} -> {external_dept_id}
- Setup: POST /api/dev/seed-tickets {"count":20,"status":"open","dept_id":1} (Support - agent has access)
- Setup: POST /api/dev/seed-tickets {"count":10,"status":"open","dept_id":{external_dept_id}} (External - no access)
- Request: GET http://localhost:3701/api/staff/tickets?status=open
- Headers: Cookie: {session_cookie}
- Expect: 200, `pagination.totalCount` == 20 (only visible tickets counted)
- Status: [x]

### AC-13: Pagination works with sort params. [API-ONLY]
- Setup: POST /api/dev/seed-tickets {"count":30,"status":"open","vary_created":true} -> {ids}
- Request: GET http://localhost:3701/api/staff/tickets?status=open&sort=date&order=DESC&limit=10&p=2
- Headers: Cookie: {session_cookie}
- Expect: 200, page 2 of date-sorted results; tickets 11-20 by date DESC
- Status: [x]

## Test Infrastructure

### Dev Endpoints Required for Test Setup

| Endpoint | Purpose | Request Shape |
|----------|---------|---------------|
| `POST /api/dev/seed-tickets` | Create test tickets | `{"count":N,"status":"...","dept_id":N}` |
| `POST /api/dev/set-staff-config` | Set staff personal page limit | `{"staff_id":N,"page_limit":N\|null}` |
| `POST /api/dev/set-config` | Set system config value | `{"key":"...","value":"..."}` |
| `POST /api/dev/reset-config` | Clear config to defaults | `{}` |
| `POST /api/dev/seed-dept` | Create department for visibility test | `{"name":"..."}` |

## Dependencies

- **TS-M3-A1**: status param handling.
- **TS-M3-A2**: visibility scoping (must be applied to count query).
- **TS-M3-B1**: sort param handling (pagination + sort work together).
- **TS-M3-prep**: seed with `default_page_size` config key.
