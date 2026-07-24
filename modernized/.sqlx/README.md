# SQLx offline query cache

This directory holds the **committed SQLx query cache** so the workspace builds
in **offline mode without a live database** (CI / `SQLX_OFFLINE=true cargo build`).

Convention (ROADMAP M1 Decision 5, owned by TS-M1-A1; all DB-touching tickets follow it):

- After adding or changing any compile-time query macro (`sqlx::query!`,
  `query_as!`, `query_scalar!`, …), regenerate the cache against the dev DB:

  ```sh
  DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_dev \
    cargo sqlx prepare --workspace
  ```

- Commit the resulting `.sqlx/query-*.json` files alongside the code change.
- Verify offline builds still work before committing:

  ```sh
  SQLX_OFFLINE=true cargo build   # no DATABASE_URL / no DB needed
  ```

`cargo-sqlx` is installed with:

```sh
cargo install sqlx-cli --version 0.8.6 --locked \
  --no-default-features --features rustls,postgres
```

As of TS-M1-A1 there are **no compile-time query macros yet** — the health-check
DB ping uses a runtime `sqlx::query_scalar(...)` (not the `!` macro), so this cache
is currently empty (`.gitkeep` keeps the directory tracked). Downstream tickets
(TS-M1-A2 migrations, TS-M1-B1 ticket core, …) populate it.
