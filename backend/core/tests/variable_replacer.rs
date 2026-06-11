//! TS-M2-C1 AC-6 — the M2 catalog tokens resolve against a REAL seeded ticket.
//!
//! DB-backed; skipped (pass, log line) when `TEST_DATABASE_URL` is unset so the
//! offline build and a DB-less `cargo test` stay green.
//!
//! ```sh
//! TEST_DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_test \
//!   cargo test -p ost_core --test variable_replacer
//! ```
//!
//! The pure grammar/dot-path/unknown-preserved/%{url} rules (AC-1..AC-5) live as
//! unit tests in `src/variable.rs` (`cargo test -p ost_core variable`).
//!
//! @implements FS-040.11 / BS-040.14–.19: the M2 token catalog resolves against a
//!   seeded ticket row; `%{url}` resolves to the passed base URL (AC-6).

use ost_core::ticket::{create_ticket, NewTicket, NewTicketInput};
use ost_core::{VarContext, VariableReplacer};
use sqlx::postgres::PgPool;

fn test_db_url() -> Option<String> {
    std::env::var("TEST_DATABASE_URL").ok()
}

async fn migrated_pool() -> Option<PgPool> {
    let url = test_db_url()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

/// Ensure a department exists (the `create_ticket` default-dept lookup needs at
/// least one). Avoids a `tools` dev-dependency (which would form a cycle:
/// tools → ost_core). Idempotent on the unique `dept_name`.
async fn ensure_department(pool: &PgPool) -> String {
    let name = "Support";
    sqlx::query(
        "INSERT INTO department (dept_name, ispublic) VALUES ($1, true)
         ON CONFLICT (dept_name) DO NOTHING",
    )
    .bind(name)
    .execute(pool)
    .await
    .unwrap();
    name.to_string()
}

/// The AC-named module so `cargo test -p ost_core variable_replacer::catalog`
/// selects exactly this case.
mod catalog {
    use super::*;

    #[tokio::test]
    async fn m2_tokens_resolve_against_a_real_seeded_ticket() {
        let Some(pool) = migrated_pool().await else {
            eprintln!("TEST_DATABASE_URL unset — skipping catalog::m2_tokens_resolve...");
            return;
        };
        ensure_department(&pool).await;

        // Seed a real ticket via the shared core.
        let nt = NewTicket::validated(NewTicketInput {
            email: "catalog@example.com".into(),
            name: "Catalog Tester".into(),
            subject: "Token catalog".into(),
            body: "first message".into(),
            source: Some("Web".into()),
            dept_id: None,
        })
        .unwrap();
        let ticket = create_ticket(&pool, &nt).await.unwrap();

        // Read the row back the way a real caller (D2/E3) would, to build context.
        let row: (i64, String, String, String, String, String, String) = sqlx::query_as(
            r#"SELECT t."ticketID", t.name, t.subject, t.email, t.status,
                      to_char(t.created, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS create_date,
                      d.dept_name
               FROM ticket t JOIN department d ON d.dept_id = t.dept_id
               WHERE t.ticket_id = $1"#,
        )
        .bind(ticket.ticket_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        let (number, name, subject, email, status, create_date, dept_name) = row;

        let base_url = "http://localhost:3702";
        let ctx: VarContext = VarContext::ticket(
            number.to_string(),
            &name,
            &subject,
            &email,
            &status,
            &create_date,
            &dept_name,
        );
        let r = VariableReplacer::new(ctx, base_url);

        // Each catalog token yields the seeded ticket's value, no literal %{...}.
        assert_eq!(r.render("%{ticket.number}"), number.to_string());
        assert_eq!(r.render("%{ticket.name}"), name);
        assert_eq!(r.render("%{ticket.subject}"), subject);
        assert_eq!(r.render("%{ticket.email}"), email);
        assert_eq!(r.render("%{ticket.status}"), status);
        assert_eq!(r.render("%{ticket.create_date}"), create_date);
        assert_eq!(r.render("%{ticket.dept.name}"), dept_name);
        assert_eq!(r.render("%{url}"), base_url);

        // A combined template substitutes every token with no leftover literal.
        let out = r.render(
            "Ticket #%{ticket.number} (%{ticket.status}) for %{ticket.name} — %{url}",
        );
        assert!(!out.contains("%{"), "no literal token remains: {out}");
        assert!(out.contains(&number.to_string()));
        assert!(out.contains(base_url));
    }
}
