// PM2 process config for the osTicket modernised backend (production).
//
// Runs the pre-compiled, statically-linked `api` binary directly
// (interpreter: 'none' — PM2 does NOT wrap it in node). The binary and the SPA
// dist are rsynced into the runtime dir; secrets live ONLY in the runtime
// `.env` (env_file), never in this committed file.
//
// Usage on the VPS (from the runtime dir):
//   pm2 startOrReload deploy/ecosystem.config.js   # (deploy.sh does this)
//   pm2 save                                        # persist across reboot
//
// The api binary reads its config from the process environment; env_file loads
// the runtime `.env` (DATABASE_URL, APP_ENV, APP_HOST, APP_PORT,
// APP_FRONTEND_ORIGIN, BLOB_ROOT, SMTP_*). See .env.production.example.

const RUNTIME_DIR = "/home/debian/apps/runtime/os-ticket-prod";

module.exports = {
  apps: [
    {
      name: "osticket-prod",
      script: "./bin/api",
      interpreter: "none",
      cwd: RUNTIME_DIR,
      // Load all runtime config + secrets from the runtime .env (NOT committed).
      env_file: `${RUNTIME_DIR}/.env`,
      exec_mode: "fork",
      instances: 1,
      autorestart: true,
      max_restarts: 10,
      // Give a slow DB / migration apply room before PM2 counts a boot as up.
      min_uptime: "10s",
      max_memory_restart: "256M",
      out_file: `${RUNTIME_DIR}/logs/api.out.log`,
      error_file: `${RUNTIME_DIR}/logs/api.err.log`,
      merge_logs: true,
      time: true,
    },
  ],
};
