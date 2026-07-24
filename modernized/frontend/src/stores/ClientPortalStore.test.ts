import { beforeEach, describe, expect, it } from "vitest";
import { server, http, HttpResponse } from "../test/mockServer";
import { ApiClient } from "../api/apiClient";
import { ClientPortalStore } from "./ClientPortalStore";

/**
 * ClientPortalStore TDD target (TS-M1-D2 AC-4). Covers login (ticket#/email),
 * the subsequent read-only thread load (M+R only), and error handling. The
 * client realm holds NO token — login yields a cookie session, and the bound
 * ticket is fetched from /api/client/ticket.
 */
describe("ClientPortalStore", () => {
  let store: ClientPortalStore;

  beforeEach(() => {
    // No-op redirect so a 401 in the bad-credentials path doesn't reach jsdom's
    // window.location (the redirect itself is covered by the apiClient tests).
    store = new ClientPortalStore(new ApiClient({ realm: "client", onUnauthorized: () => {} }));
  });

  it("starts logged out with no ticket", () => {
    expect(store.isAuthenticated).toBe(false);
    expect(store.ticket).toBeNull();
    expect(store.error).toBeNull();
  });

  it("logs in (ticket#/email), then loads the bound ticket thread (M+R, in order)", async () => {
    let loginBody: unknown = null;
    server.use(
      http.post("/api/client/login", async ({ request }) => {
        loginBody = await request.json();
        return HttpResponse.json({ ok: true, csrfToken: "c" });
      }),
      http.get("/api/client/ticket", () =>
        HttpResponse.json({
          number: 123456,
          subject: "Cannot log in",
          status: "open",
          created: "2026-06-01T10:00:00Z",
          entries: [
            { id: 10, threadType: "M", poster: "Alice", body: "Cannot log in" },
            { id: 12, threadType: "R", poster: "Agent", body: "Try a reset" },
          ],
        }),
      ),
    );

    await store.login({ ticketNumber: "123456", email: "alice@example.com" });

    expect(loginBody).toEqual({ ticketNumber: "123456", email: "alice@example.com" });
    expect(store.isAuthenticated).toBe(true);
    expect(store.ticket?.number).toBe(123456);
    expect(store.ticket?.entries.map((e) => e.id)).toEqual([10, 12]);
    expect(store.ticket?.entries.map((e) => e.threadType)).toEqual(["M", "R"]);
    expect(store.error).toBeNull();
  });

  it("surfaces a generic error and stays logged out on bad credentials", async () => {
    server.use(
      http.post("/api/client/login", () =>
        HttpResponse.json(
          { error: { message: "Authentication error - try again!" } },
          { status: 401 },
        ),
      ),
    );

    await expect(
      store.login({ ticketNumber: "000000", email: "nobody@example.com" }),
    ).rejects.toBeTruthy();

    expect(store.isAuthenticated).toBe(false);
    expect(store.ticket).toBeNull();
    expect(store.error).toBe("Authentication error - try again!");
  });

  it("can reload the bound ticket on its own (session restore)", async () => {
    server.use(
      http.get("/api/client/ticket", () =>
        HttpResponse.json({
          number: 123456, subject: "Cannot log in", status: "open",
          created: "2026-06-01T10:00:00Z",
          entries: [{ id: 10, threadType: "M", poster: "Alice", body: "Cannot log in" }],
        }),
      ),
    );

    await store.loadTicket();
    expect(store.ticket?.number).toBe(123456);
    expect(store.isAuthenticated).toBe(true);
  });
});
