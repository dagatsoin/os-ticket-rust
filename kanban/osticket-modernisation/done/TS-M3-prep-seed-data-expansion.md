# TS-M3-prep — add(seed): M3 seed data expansion

- **ID**: TS-M3-prep
- **Type**: Technical Story
- **Parent**: M3 (cross-cutting, not under a single epic)
- **Labels**: Technical Story, M3
- **Scope**: small

## Context

M3 requires expanded seed data beyond M1/M2. The M1 seed created one department (Support),
one group, and one staff account (agent). M3 needs a second department, teams, help topics,
and config keys to test the full queue and workflow features.

**Note**: SLA plan seeding is handled separately by TS-M3-D2, which owns the SLA seed data
(Standard 24h, Urgent 4h, Default 48h) and the SLA assignments to departments/topics.

## Impact

- add(seed): second department "Sales" (dept_id=2)
- add(seed): team "Tier 2" with the agent as a member (uses `team` and `team_member` tables)
- add(seed): help topics (uses `help_topic` table):
  - "General" — isactive=true, ispublic=true, noautoresp=false, priority_id=2, dept_id=Support, staff_id=null, team_id=null, sla_id=null (inherits default), page_id=0, sort=1
  - "Billing" — isactive=true, ispublic=true, noautoresp=false, priority_id=2, dept_id=Sales, staff_id=null, team_id=null, sla_id=null (TS-M3-D2 assigns Urgent SLA later), page_id=0, sort=2
- update(seed): group-dept access — agent's group has access to both Support and Sales
- add(seed): config keys — `show_assigned_tickets=1`, `show_answered_tickets=0`, `ticket_lock_time=2`, `max_page_size=25`
- update(seed): group permission flags — ensure `can_close_tickets=1`, `can_delete_tickets=1`, `can_assign_tickets=1`, `can_transfer_tickets=1`, `can_edit_tickets=1`

## Regressions

- The M1/M2 seed data must remain intact. This is an additive expansion.
- The `--reset` flag should clear and re-seed all data including the new M3 additions.

## Acceptance Tests

### AC-1: The seed creates two departments (Support, Sales). [API-ONLY]
- Setup: `cargo run -p tools --bin seed -- --reset`
- Request: `psql -U postgres -d osticket_dev -c "SELECT name FROM department ORDER BY dept_id"`
- Expect: 200, rows containing "Support" and "Sales"
- Status: [x]

### AC-2: The seed creates one team (Tier 2) with agent as member. [API-ONLY]
- Setup: seed applied (from AC-1)
- Request: `psql -U postgres -d osticket_dev -c "SELECT t.name, s.username FROM team t JOIN team_member tm ON t.team_id = tm.team_id JOIN staff s ON tm.staff_id = s.staff_id"`
- Expect: 200, row with team "Tier 2" and username "agent"
- Status: [x]

### AC-3: The seed creates two help topics (General, Billing) with full column values. [API-ONLY]
- Setup: seed applied (from AC-1)
- Request: `psql -U postgres -d osticket_dev -c "SELECT topic, isactive, ispublic, noautoresp, priority_id, dept_id, sla_id, page_id, sort FROM help_topic ORDER BY topic_id"`
- Expect: 200, "General" row (isactive=t, ispublic=t, noautoresp=f, priority_id=2, dept_id=Support, sla_id=null, page_id=0, sort=1) and "Billing" row (isactive=t, ispublic=t, noautoresp=f, priority_id=2, dept_id=Sales, sla_id=null, page_id=0, sort=2)
- Status: [x]

### AC-4: The agent's group has access to both departments. [API-ONLY]
- Setup: seed applied (from AC-1)
- Request: `psql -U postgres -d osticket_dev -c "SELECT d.name FROM group_dept_access gda JOIN department d ON gda.dept_id = d.dept_id WHERE gda.group_id = (SELECT group_id FROM staff WHERE username = 'agent')"`
- Expect: 200, rows containing both "Support" and "Sales"
- Status: [x]

### AC-5: The config keys for M3 are seeded. [API-ONLY]
- Setup: seed applied (from AC-1)
- Request: `psql -U postgres -d osticket_dev -c "SELECT key, value FROM config WHERE key IN ('show_assigned_tickets', 'show_answered_tickets', 'ticket_lock_time', 'max_page_size') ORDER BY key"`
- Expect: 200, rows: max_page_size=25, show_answered_tickets=0, show_assigned_tickets=1, ticket_lock_time=2
- Status: [x]

### AC-6: The agent's group has the required permission flags. [API-ONLY]
- Setup: seed applied (from AC-1)
- Request: `psql -U postgres -d osticket_dev -c "SELECT can_close_tickets, can_delete_tickets, can_assign_tickets, can_transfer_tickets, can_edit_tickets FROM groups WHERE group_id = (SELECT group_id FROM staff WHERE username = 'agent')"`
- Expect: 200, all flags = true (t, t, t, t, t)
- Status: [x]

## Test Infrastructure

- The expanded seed enables the M3 E2E tests (second department for transfer, SLA for overdue, etc.).

## Dependencies

- **TS-M3-schema**: migration must land first (creates `team`, `team_member`, `help_topic` tables and the `groups` permission flag columns).
- **M1 done**: the seed binary and base schema exist.
