// Shared MSW mock-API server. The default handlers cover the health endpoint;
// individual tests override per-case with `server.use(...)`.
// This is the mock-API layer reused by the B3/C4/D2 component tests.
import { setupServer } from "msw/node";
import { http, HttpResponse } from "msw";

export const handlers = [
  http.get("/api/health", () =>
    HttpResponse.json({ status: "ok", db: "up" }),
  ),
];

export const server = setupServer(...handlers);

// Re-export so test files can build ad-hoc handlers without a second import.
export { http, HttpResponse };
