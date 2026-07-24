# TS-M3-D3 — add(service): FS-021.13 SLA selection precedence + due-date computation

- **ID**: TS-M3-D3
- **Type**: Technical Story
- **Parent**: US-M3-D1
- **Labels**: Technical Story, M3
- **Scope**: small

## Context

Implements the SLA selection precedence logic and due-date computation. When a ticket is
created or transferred, the system selects the effective SLA by precedence, then computes
the due date as `created + grace_period` (or `reopened + grace_period` for reopened tickets).

## Impact

- add(service): FS-021.13 `ticket_service::select_sla(ticket) -> Option<SlaId>`:
  - Precedence: explicit trump value > department SLA > help topic SLA > system default SLA.
  - Returns the resolved SLA id or None.
- add(service): FS-032.11 `ticket_service::compute_due_date(ticket) -> Option<DateTime>`:
  - If ticket has explicit duedate, return it.
  - Else if SLA is set, return `base_time + grace_period` where:
    - base_time = ticket.reopened if reopened, else ticket.created.
  - Else return None.
- update(route): `POST /api/tickets` (open) applies SLA selection + due-date computation on create.
- update(route): ticket detail response includes `duedate`, `sla_id`, `sla_name`.

## Regressions

- Existing ticket creation flow must not break; SLA fields are optional.
- Tickets without SLA should have null duedate.

## Acceptance Tests

### AC-1: FS-021.13 — SLA selection: department SLA takes precedence over topic SLA. [API-ONLY]
- Setup: Run seed (Support dept has SLA "Standard" 24h; Billing topic has SLA "Urgent" 4h)
- Setup: POST /api/dev/seed-tickets with `{"dept":"Support","topic":"Billing"}`
- Request: GET http://localhost:3701/api/staff/tickets/{id}
- Headers: Authorization: Bearer {{staff_token}}
- Expect: 200, sla_id matches "Standard" SLA id (not "Urgent")
- Status: [x]

### AC-2: FS-021.13 — SLA selection: topic SLA used when department has no SLA. [API-ONLY]
- Setup: Create dept "External" with no SLA via seed or dev endpoint
- Setup: POST /api/dev/seed-tickets with `{"dept":"External","topic":"Billing"}`
- Request: GET http://localhost:3701/api/staff/tickets/{id}
- Headers: Authorization: Bearer {{staff_token}}
- Expect: 200, sla_id matches "Urgent" SLA id (topic SLA used)
- Status: [x]

### AC-3: FS-021.13 — SLA selection: system default used when dept and topic have no SLA. [API-ONLY]
- Setup: Create dept "External" with no SLA; topic "General" with no SLA
- Setup: POST /api/dev/seed-tickets with `{"dept":"External","topic":"General"}`
- Request: GET http://localhost:3701/api/staff/tickets/{id}
- Headers: Authorization: Bearer {{staff_token}}
- Expect: 200, sla_id matches "Default SLA" id (system default used)
- Status: [x]

### AC-4: FS-032.11 — Due date computed as created + grace_period. [API-ONLY]
- Setup: POST /api/dev/seed-tickets with `{"dept":"Support"}` (Support has Standard 24h SLA)
- Request: GET http://localhost:3701/api/staff/tickets/{id}
- Headers: Authorization: Bearer {{staff_token}}
- Expect: 200, duedate = created + 24 hours (within 60 second tolerance)
- Status: [x]

### AC-5: FS-032.11 — Reopened ticket: due date computed from reopened timestamp. [API-ONLY]
- Setup: POST /api/dev/seed-tickets with `{"dept":"Support"}` -> ticket_id
- Setup: POST /api/staff/tickets/{ticket_id}/close
- Setup: POST /api/staff/tickets/{ticket_id}/reopen
- Request: GET http://localhost:3701/api/staff/tickets/{ticket_id}
- Headers: Authorization: Bearer {{staff_token}}
- Expect: 200, duedate = reopened + grace_period (not created + grace_period)
- Status: [x]

### AC-6: BS-032.10 — Ticket detail response includes duedate, sla_id, sla_name. [API-ONLY]
- Setup: POST /api/dev/seed-tickets with `{"dept":"Support"}`
- Request: GET http://localhost:3701/api/staff/tickets/{id}
- Headers: Authorization: Bearer {{staff_token}}
- Expect: 200, response has fields: duedate (ISO timestamp), sla_id (integer), sla_name (string)
- Status: [x]

## Test Infrastructure

- Uses seeded staff credentials `agent` / `Agent123!` (TS-M1-A3).
- Dev endpoints needed:
  - **`POST /api/dev/seed-tickets`** — accepts `dept`, `topic` params for SLA precedence tests
- Requires TS-M3-D2 seed (SLA plans assigned to dept/topic).

## Dependencies

- **TS-M3-D1**: sla_plan schema.
- **TS-M3-D2**: SLA plans seeded.
- **EPIC-M3-C**: reopen action (for AC-5 testing).
