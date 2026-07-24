# syntax=docker/dockerfile:1
#
# Portable linux/amd64 production build for the osTicket modernised backend.
#
# Compiles BOTH workspace binaries — the `api` server (crate `api`) and the
# `seed` tool (crate `tools`) — as fully-STATIC musl executables
# (x86_64-unknown-linux-musl), so they run on any linux/amd64 host with no
# glibc / OpenSSL / runtime dependency (ideal for a low-RAM Debian VPS with no
# Rust toolchain).
#
# Why musl works here: every dependency is pure-Rust or musl-friendly —
# sqlx-postgres, rustls (ring), argon2, lettre. The `clux/muslrust` image ships
# the musl cross-toolchain that `ring` needs, so the static link succeeds.
#
# Build-time facts that keep this self-contained (NO database needed to build):
#   * All SQL is runtime string-based (`sqlx::query` / `query_scalar`, no
#     compile-time `sqlx::query!` macros) — nothing to check against a live DB.
#   * Migrations are embedded at compile time via `sqlx::migrate!("../../migrations")`,
#     so the shipped `api` binary carries the schema and applies it on startup —
#     the `migrations/` dir does NOT need to be shipped to the VPS.
#   * `SQLX_OFFLINE=true` is set as belt-and-suspenders for the committed `.sqlx/`
#     cache convention.
#
# Build + extract the two binaries to ./deploy/bin (build context = modernized/):
#
#   docker buildx build --platform linux/amd64 -f modernized/Dockerfile \
#     --target export --output type=local,dest=modernized/deploy/bin modernized
#
# (An image build `-t osticket-build` also works; the `export` stage is a
# scratch image containing just the two binaries for a clean `--output` extract.)

# ---- Stage 1: build both binaries as static linux/amd64 musl ---------------
FROM clux/muslrust:stable AS builder

ENV SQLX_OFFLINE=true
WORKDIR /volume

# Copy only what the workspace build needs (frontend/, var/, target/, deploy/
# are excluded via .dockerignore and are not copied here regardless).
COPY Cargo.toml Cargo.lock ./
COPY .sqlx ./.sqlx
COPY migrations ./migrations
COPY backend ./backend
COPY tools ./tools

# Release build, static musl target, both binaries in one pass.
RUN cargo build --release --target x86_64-unknown-linux-musl --bin api --bin seed \
    && strip target/x86_64-unknown-linux-musl/release/api \
    && strip target/x86_64-unknown-linux-musl/release/seed

# ---- Stage 2: export just the two static binaries --------------------------
# A scratch stage so `--target export --output type=local` writes exactly the
# two files (api, seed) into the destination directory — nothing else.
FROM scratch AS export
COPY --from=builder /volume/target/x86_64-unknown-linux-musl/release/api /api
COPY --from=builder /volume/target/x86_64-unknown-linux-musl/release/seed /seed
