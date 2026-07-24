#!/usr/bin/env bash
# =============================================================================
# deploy.sh — (re)start the osTicket modernised backend on the VPS.
# =============================================================================
# Runs ON THE VPS, from the runtime dir, AFTER the binaries + SPA dist have been
# rsynced in (see README.md for the rsync + build-here steps). It is idempotent:
# re-running just reloads PM2 and re-checks health. It holds NO secrets — all
# config comes from the runtime `.env` loaded by PM2 (ecosystem.config.js).
#
# Usage (on the VPS):
#     cd /home/debian/apps/runtime/os-ticket-prod
#     ./deploy.sh
#
# It does NOT: provision the DB, run the seed, or touch nginx/certbot — those
# are one-time manual steps documented in README.md (running them from a script
# would risk clobbering certbot's in-place TLS edits or re-seeding prod).
# =============================================================================
set -euo pipefail

RUNTIME_DIR="${RUNTIME_DIR:-/home/debian/apps/runtime/os-ticket-prod}"
HEALTH_URL="${HEALTH_URL:-http://127.0.0.1:3701/api/health}"
ECOSYSTEM="${RUNTIME_DIR}/deploy/ecosystem.config.js"

cd "$RUNTIME_DIR"

echo "==> osTicket prod deploy — runtime dir: $RUNTIME_DIR"

# --- Preflight: required files must be in place ------------------------------
missing=0
for f in "bin/api" ".env" "$ECOSYSTEM" "dist/index.html"; do
  if [[ ! -e "$f" && ! -e "$RUNTIME_DIR/$f" ]]; then
    echo "!! missing required file: $f" >&2
    missing=1
  fi
done
if [[ "$missing" -ne 0 ]]; then
  echo "!! aborting: rsync the binaries + dist and create .env first (see README.md)" >&2
  exit 1
fi

chmod +x "$RUNTIME_DIR/bin/api" "$RUNTIME_DIR/bin/seed" 2>/dev/null || true
mkdir -p "$RUNTIME_DIR/logs" "$RUNTIME_DIR/blobs"

# --- Start or reload the PM2 app (zero-downtime reload if already running) ----
echo "==> pm2 startOrReload $ECOSYSTEM"
pm2 startOrReload "$ECOSYSTEM"
pm2 save

# --- Health check -------------------------------------------------------------
echo "==> health check: $HEALTH_URL"
ok=0
for attempt in $(seq 1 15); do
  if body="$(curl -fsS --max-time 5 "$HEALTH_URL" 2>/dev/null)"; then
    echo "   health: $body"
    if printf '%s' "$body" | grep -q '"status":"ok"'; then
      ok=1
      break
    fi
  fi
  echo "   (attempt $attempt) not ready yet, retrying..."
  sleep 2
done

if [[ "$ok" -ne 1 ]]; then
  echo "!! health check did NOT return status:ok — inspect: pm2 logs osticket-prod" >&2
  exit 1
fi

echo "==> OK — osticket-prod is up and healthy."
cat <<'EOF'

Next steps (one-time / as needed):
  * First deploy only — provision DB + seed admin (see README.md), e.g.:
      ADMIN_PASSWORD='a-strong-password' \
        DATABASE_URL="postgres://osticket:...@127.0.0.1:5432/osticket_prod" \
        ./bin/seed
  * nginx (first time): symlink the site, `sudo nginx -t`, `sudo systemctl reload nginx`,
    then `sudo certbot --nginx -d os-ticket.warfog.gg`.
  * Verify publicly: https://os-ticket.warfog.gg  and  /api/health
EOF
