# osTicket modernised — production deploy runbook

Target: **https://os-ticket.warfog.gg** on a Debian VPS (user `debian`).

Strategy: the linux/amd64 backend binaries are **built locally on a Mac via
Docker** (the VPS has no Rust toolchain and little RAM) and shipped. The SPA is
built locally too. nginx serves the SPA and reverse-proxies `/api/*` to the
backend on `127.0.0.1:3701`. PM2 supervises the backend. certbot adds TLS.

```
Browser ──HTTPS──▶ nginx (:443) ─┬─ /            → SPA static (dist/)
                                 └─ /api/*        → 127.0.0.1:3701 (api, PM2)
                                                     └─ Postgres :5432 (osticket_prod)
                                                     └─ BLOB_ROOT (attachment bytes)
```

Paths on the VPS:
| Purpose         | Path                                              |
|-----------------|---------------------------------------------------|
| Source checkout | `~/apps/source/os-ticket`  (optional; git pull)   |
| Runtime dir     | `~/apps/runtime/os-ticket-prod`                   |
| Binaries        | `~/apps/runtime/os-ticket-prod/bin/{api,seed}`    |
| SPA             | `~/apps/runtime/os-ticket-prod/dist/`             |
| Blob store      | `~/apps/runtime/os-ticket-prod/blobs/`            |
| Runtime env     | `~/apps/runtime/os-ticket-prod/.env` (secrets)    |

Ports: backend **3701** (loopback). Postgres **5432** (shared container).

---

## 1. Build the artifacts locally (on this Mac)

From the repo root (`osTicket-1.7/`):

```sh
# Backend: static linux/amd64 musl binaries (api + seed) → modernized/deploy/bin/
docker buildx build --platform linux/amd64 -f modernized/Dockerfile \
  --target export --output type=local,dest=modernized/deploy/bin modernized

file modernized/deploy/bin/api    # → ELF 64-bit x86-64, static-pie linked, stripped

# Frontend: production SPA → modernized/frontend/dist/
npm --prefix modernized/frontend run build
```

`bin/` and `dist/` are gitignored build outputs — they are shipped by rsync,
not committed. Migrations are **embedded** in the `api` binary
(`sqlx::migrate!`) and applied on startup, so the `migrations/` dir is **not**
shipped.

## 2. Ship the artifacts to the VPS

```sh
VPS=debian@os-ticket.warfog.gg
RUNTIME=~/apps/runtime/os-ticket-prod

ssh $VPS "mkdir -p $RUNTIME/bin $RUNTIME/dist $RUNTIME/logs $RUNTIME/blobs $RUNTIME/deploy"

# Binaries
rsync -av modernized/deploy/bin/            $VPS:$RUNTIME/bin/
# SPA (delete removed assets)
rsync -av --delete modernized/frontend/dist/ $VPS:$RUNTIME/dist/
# Deploy assets (ecosystem + deploy.sh; nginx conf handled in step 5)
rsync -av modernized/deploy/ecosystem.config.js modernized/deploy/deploy.sh \
          modernized/deploy/nginx-prod.conf     $VPS:$RUNTIME/deploy/
```

## 3. Provision the database (first time only)

On the shared postgres container (superuser `postgres`):

```sh
docker exec -it backend-db-1 psql -U postgres <<'SQL'
CREATE ROLE osticket LOGIN PASSWORD 'a-strong-db-password';
CREATE DATABASE osticket_prod OWNER osticket;
SQL
```

Schema is created automatically by the `api` binary on first startup (embedded
migrations) — no manual `migrate run` needed.

## 4. Write the runtime `.env` (secrets — never committed)

```sh
ssh $VPS
cd ~/apps/runtime/os-ticket-prod
cp deploy/../../deploy/.env.production.example .env   # or paste the template
$EDITOR .env
```

Fill in: `DATABASE_URL` (the role/password from step 3), `APP_ENV=production`,
`APP_HOST=127.0.0.1`, `APP_PORT=3701`,
`APP_FRONTEND_ORIGIN=https://os-ticket.warfog.gg`,
`BLOB_ROOT=/home/debian/apps/runtime/os-ticket-prod/blobs`, and the `SMTP_*`
values if real outbound mail is wanted (leave `SMTP_HOST` empty to disable mail).
See `.env.production.example` for the full template.

> **Keep `APP_ENV=production`** — it is the fail-safe default and disables the
> `/api/dev/*` test endpoints.

## 5. Start the backend (PM2)

```sh
cd ~/apps/runtime/os-ticket-prod
./deploy/deploy.sh          # pm2 startOrReload + health check on /api/health
```

`deploy.sh` is idempotent — re-run it after every artifact rsync to reload the
process and re-verify health. Ensure PM2 restarts on reboot (one time):

```sh
pm2 startup    # run the command it prints (with sudo)
pm2 save
```

## 6. Seed the admin + reference data (first time only)

Run the shipped `seed` binary once, passing a strong admin password **inline**
(it is read from the environment and never stored in a file):

```sh
cd ~/apps/runtime/os-ticket-prod
set -a; . ./.env; set +a          # load DATABASE_URL from the runtime env
ADMIN_PASSWORD='a-strong-admin-password' ./bin/seed
```

This creates the departments/groups/help-topics/SLA/config defaults and the
`admin` account with **your** password (not the public `Admin123!` default).
The seed is idempotent; re-running with `ADMIN_PASSWORD` set also rotates the
admin password. The `agent`/`agent2` demo accounts keep their documented dev
passwords — disable or delete them for a locked-down prod if desired.

## 7. nginx + TLS (first time)

```sh
# Install the site (HTTP-only at this point)
sudo cp ~/apps/runtime/os-ticket-prod/deploy/nginx-prod.conf \
        /etc/nginx/sites-available/os-ticket.conf
sudo ln -sf /etc/nginx/sites-available/os-ticket.conf \
            /etc/nginx/sites-enabled/os-ticket.conf
sudo nginx -t && sudo systemctl reload nginx

# Add TLS — certbot edits the installed file in place (adds listen 443 ssl; etc.)
sudo certbot --nginx -d os-ticket.warfog.gg
```

The committed `nginx-prod.conf` stays HTTP-only; the on-VPS file becomes the
post-certbot source of truth. Do not overwrite it on later deploys unless you
re-run certbot.

## 8. Verify

```sh
curl -fsS http://127.0.0.1:3701/api/health          # {"status":"ok","db":"ok"}
```
Then open **https://os-ticket.warfog.gg** and log in as `admin`.

---

## Redeploying (subsequent releases)

```sh
# On the Mac: rebuild changed artifacts
docker buildx build --platform linux/amd64 -f modernized/Dockerfile \
  --target export --output type=local,dest=modernized/deploy/bin modernized
npm --prefix modernized/frontend run build

# Ship + reload
rsync -av modernized/deploy/bin/            $VPS:$RUNTIME/bin/
rsync -av --delete modernized/frontend/dist/ $VPS:$RUNTIME/dist/
ssh $VPS "cd $RUNTIME && ./deploy/deploy.sh"
```

New migrations ride along inside the rebuilt `api` binary and apply on the next
start. Only re-run the seed if reference data changed (it is idempotent);
only re-run steps 3/7 for infra changes.
