//! Staff visibility predicate for ticket queue scoping (TS-M3-A2, BS-020.2).
//!
//! @implements BS-020.2: Department + assignment visibility is always enforced.
//!
//! The visibility predicate restricts which tickets a staff member can see in any
//! queue or search. This is the security boundary ensuring agents only see tickets
//! in their departments or assigned to them.
//!
//! Per BS-020.2 and FS-020.4, the visibility predicate is the OR of:
//! 1. Tickets directly assigned to the staff member AND open (`staff_id = me AND status = 'open'`)
//! 2. IF NOT access-limited: tickets in any department the staff member has access to (`dept_id IN (my_depts)`)
//! 3. IF the staff member belongs to teams: tickets assigned to those teams AND open (`team_id IN (my_teams) AND status = 'open'`)

use sqlx::postgres::PgPool;

/// Staff visibility context for building the visibility predicate.
///
/// This struct holds all the information needed to determine which tickets
/// a staff member can see.
///
/// @implements BS-020.2: visibility context for scoping queries.
#[derive(Debug, Clone)]
pub struct StaffVisibility {
    /// The staff member's internal ID.
    pub staff_id: i32,
    /// Department IDs the staff member has access to (via their group).
    pub dept_ids: Vec<i32>,
    /// Team IDs the staff member belongs to.
    pub team_ids: Vec<i32>,
    /// Whether the staff member is "access-limited" (show_assigned_only).
    /// If true, they see ONLY assigned tickets, not whole departments.
    pub is_access_limited: bool,
}

impl StaffVisibility {
    /// Build the WHERE clause fragment for the visibility predicate.
    ///
    /// The predicate is the OR of:
    /// 1. Tickets directly assigned to the staff member AND open
    /// 2. IF NOT access-limited: tickets in accessible departments
    /// 3. IF has teams: tickets assigned to those teams AND open
    ///
    /// Returns `None` if the staff member has no visibility at all (empty depts,
    /// empty teams, and is access-limited) — they can only see tickets assigned
    /// directly to them.
    ///
    /// @implements FS-020.4: visibility predicate construction.
    /// @implements BS-020.2: department + assignment visibility rules.
    pub fn build_where_clause(&self) -> String {
        let mut clauses: Vec<String> = Vec::new();

        // Clause 1: Tickets directly assigned to the staff member AND open.
        // This is ALWAYS included — every staff member can see tickets assigned to them.
        // Note: All column references are qualified with `ticket.` to avoid ambiguity
        // when JOINing other tables (e.g., `staff AS assigned_staff`).
        clauses.push(format!(
            "(ticket.staff_id = {} AND ticket.status = 'open')",
            self.staff_id
        ));

        // Clause 2: Tickets in accessible departments (only if NOT access-limited).
        // @implements BS-020.2: access-limited agent sees only assigned tickets.
        if !self.is_access_limited && !self.dept_ids.is_empty() {
            let dept_list = self
                .dept_ids
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            clauses.push(format!("(ticket.dept_id IN ({}))", dept_list));
        }

        // Clause 3: Tickets assigned to the staff member's teams AND open.
        // @implements BS-020.2: team membership widens visibility.
        if !self.team_ids.is_empty() {
            let team_list = self
                .team_ids
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            clauses.push(format!(
                "(ticket.team_id IN ({}) AND ticket.status = 'open')",
                team_list
            ));
        }

        // Combine all clauses with OR.
        format!("({})", clauses.join(" OR "))
    }
}

/// Load the visibility context for a staff member from the database.
///
/// Fetches:
/// - The staff member's `show_assigned_only` flag
/// - The department IDs their group has access to (via `group_dept_access`)
/// - The team IDs they belong to (via `team_member`)
///
/// @implements TS-M3-A2: query helpers to fetch visibility data.
pub async fn load_staff_visibility(
    pool: &PgPool,
    staff_id: i32,
) -> Result<StaffVisibility, sqlx::Error> {
    // Fetch the staff member's show_assigned_only flag.
    let is_access_limited: bool = sqlx::query_scalar(
        "SELECT COALESCE(show_assigned_only, false) FROM staff WHERE staff_id = $1",
    )
    .bind(staff_id)
    .fetch_one(pool)
    .await?;

    // Fetch the department IDs the staff member's group has access to.
    // @implements BS-020.2: department access derives from group -> dept access.
    let dept_ids: Vec<i32> = sqlx::query_scalar(
        "SELECT gda.dept_id
         FROM group_dept_access gda
         JOIN staff s ON s.group_id = gda.group_id
         WHERE s.staff_id = $1",
    )
    .bind(staff_id)
    .fetch_all(pool)
    .await?;

    // Fetch the team IDs the staff member belongs to.
    // @implements BS-020.2: team membership from team_member table.
    let team_ids: Vec<i32> = sqlx::query_scalar(
        "SELECT team_id FROM team_member WHERE staff_id = $1",
    )
    .bind(staff_id)
    .fetch_all(pool)
    .await?;

    Ok(StaffVisibility {
        staff_id,
        dept_ids,
        team_ids,
        is_access_limited,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @implements TS-M3-A2 AC-1: Agent sees tickets in accessible departments.
    #[test]
    fn visibility_includes_dept_when_not_access_limited() {
        let vis = StaffVisibility {
            staff_id: 1,
            dept_ids: vec![1, 2],
            team_ids: vec![],
            is_access_limited: false,
        };
        let clause = vis.build_where_clause();
        // Should include both direct assignment AND department access
        assert!(clause.contains("staff_id = 1"));
        assert!(clause.contains("dept_id IN (1, 2)"));
    }

    /// @implements TS-M3-A2 AC-2: Agent does NOT see tickets in inaccessible departments.
    #[test]
    fn visibility_excludes_dept_not_in_list() {
        let vis = StaffVisibility {
            staff_id: 1,
            dept_ids: vec![1], // Only dept 1
            team_ids: vec![],
            is_access_limited: false,
        };
        let clause = vis.build_where_clause();
        // Should include dept 1 but NOT dept 2
        assert!(clause.contains("dept_id IN (1)"));
        assert!(!clause.contains("dept_id IN (2)"));
    }

    /// @implements TS-M3-A2 AC-3: Agent sees tickets assigned to them regardless of dept.
    #[test]
    fn visibility_always_includes_direct_assignment() {
        let vis = StaffVisibility {
            staff_id: 1,
            dept_ids: vec![],
            team_ids: vec![],
            is_access_limited: true, // Access-limited but still sees own tickets
        };
        let clause = vis.build_where_clause();
        // Should always include the direct assignment clause
        // Note: clause uses `ticket.` prefix for all column references.
        assert!(clause.contains("ticket.staff_id = 1 AND ticket.status = 'open'"));
    }

    /// @implements TS-M3-A2 AC-4: Agent sees tickets assigned to their team.
    #[test]
    fn visibility_includes_team_assignment() {
        let vis = StaffVisibility {
            staff_id: 1,
            dept_ids: vec![1],
            team_ids: vec![1, 2],
            is_access_limited: false,
        };
        let clause = vis.build_where_clause();
        // Should include team assignment clause
        // Note: clause uses `ticket.` prefix for all column references.
        assert!(clause.contains("ticket.team_id IN (1, 2) AND ticket.status = 'open'"));
    }

    /// @implements TS-M3-A2 AC-5: Access-limited agent sees ONLY assigned tickets.
    #[test]
    fn access_limited_excludes_dept_visibility() {
        let vis = StaffVisibility {
            staff_id: 1,
            dept_ids: vec![1, 2], // Has dept access but is access-limited
            team_ids: vec![],
            is_access_limited: true,
        };
        let clause = vis.build_where_clause();
        // Should include direct assignment but NOT department access
        // Note: clause uses `ticket.` prefix for all column references.
        assert!(clause.contains("ticket.staff_id = 1 AND ticket.status = 'open'"));
        assert!(!clause.contains("ticket.dept_id IN"));
    }

    /// @implements TS-M3-A2 AC-7: Empty department list contributes no dept matches.
    #[test]
    fn empty_dept_list_no_dept_clause() {
        let vis = StaffVisibility {
            staff_id: 1,
            dept_ids: vec![],
            team_ids: vec![],
            is_access_limited: false,
        };
        let clause = vis.build_where_clause();
        // Should NOT include department clause at all
        // Note: clause uses `ticket.` prefix for all column references.
        assert!(!clause.contains("ticket.dept_id IN"));
        // But should still include direct assignment
        assert!(clause.contains("ticket.staff_id = 1"));
    }

    /// Combined scenario: access-limited with team membership.
    #[test]
    fn access_limited_with_team_sees_team_and_assigned() {
        let vis = StaffVisibility {
            staff_id: 1,
            dept_ids: vec![1], // Has dept but access-limited
            team_ids: vec![2],
            is_access_limited: true,
        };
        let clause = vis.build_where_clause();
        // Should include direct assignment AND team, but NOT department
        // Note: clause uses `ticket.` prefix for all column references.
        assert!(clause.contains("ticket.staff_id = 1 AND ticket.status = 'open'"));
        assert!(clause.contains("ticket.team_id IN (2) AND ticket.status = 'open'"));
        assert!(!clause.contains("ticket.dept_id IN"));
    }
}
