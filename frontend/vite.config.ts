/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// osTicket modernisation frontend.
// Dev server on 3702; /api proxied to the Rust/Axum backend on 3701 (avoids CORS).
// See ROADMAP.md → Ports (37xx) and Decisions → 6.
export default defineConfig({
  plugins: [react()],
  server: {
    port: 3702,
    strictPort: true,
    proxy: {
      "/api": {
        target: "http://localhost:3701",
        changeOrigin: true,
      },
    },
  },
  test: {
    globals: true,
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    css: false,
  },
});
