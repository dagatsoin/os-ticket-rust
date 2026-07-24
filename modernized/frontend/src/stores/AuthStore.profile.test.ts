import { beforeEach, describe, expect, it } from "vitest";
import { server, http, HttpResponse } from "../test/mockServer";
import { RootStore } from "./RootStore";

/**
 * TS-M4-A0 AuthStore additions: the `profileLoaded` tri-state (so guards render a
 * spinner rather than flash a redirect) plus the `isAdmin` / `can(flag)` consumers
 * hydrated from GET /api/staff/me.
 */
describe("AuthStore — admin profile (TS-M4-A0)", () => {
  let root: RootStore;

  beforeEach(() => {
    // Router-aware 401 hook so a failing /me doesn't hit jsdom's window.location.
    root = new RootStore({ onUnauthorized: () => {} });
  });

  it("starts in the 'loading' profile state so guards don't flash a redirect", () => {
    expect(root.staffAuth.profileLoaded).toBe("loading");
    expect(root.staffAuth.isAdmin).toBe(false);
  });

  it("hydrates isAdmin + capability flags from /me and resolves to 'loaded'", async () => {
    server.use(
      http.get("/api/staff/me", () =>
        HttpResponse.json({
          id: 1,
          username: "admin",
          isadmin: true,
          can_manage_faq: true,
          can_manage_premade: false,
          can_ban_emails: false,
          can_view_staff_stats: true,
        }),
      ),
    );

    await root.staffAuth.loadProfile();

    expect(root.staffAuth.profileLoaded).toBe("loaded");
    expect(root.staffAuth.isAuthenticated).toBe(true);
    expect(root.staffAuth.isAdmin).toBe(true);
    expect(root.staffAuth.can("can_manage_faq")).toBe(true);
    expect(root.staffAuth.can("can_manage_premade")).toBe(false);
    expect(root.staffAuth.can("can_view_staff_stats")).toBe(true);
  });

  it("treats a non-admin with a delegated flag correctly", async () => {
    server.use(
      http.get("/api/staff/me", () =>
        HttpResponse.json({ id: 2, username: "faqmod", isadmin: false, can_manage_faq: true }),
      ),
    );

    await root.staffAuth.loadProfile();

    expect(root.staffAuth.isAdmin).toBe(false);
    expect(root.staffAuth.can("can_manage_faq")).toBe(true);
    expect(root.staffAuth.can("can_ban_emails")).toBe(false);
  });

  it("resolves to 'error' (unauthenticated) when /me fails, without spinning forever", async () => {
    server.use(
      http.get("/api/staff/me", () =>
        HttpResponse.json({ error: { message: "no session" } }, { status: 401 }),
      ),
    );

    await root.staffAuth.loadProfile();

    expect(root.staffAuth.profileLoaded).toBe("error");
    expect(root.staffAuth.isAuthenticated).toBe(false);
    expect(root.staffAuth.isAdmin).toBe(false);
  });

  it("ensureProfileLoaded fetches once even when called by several mounted guards", async () => {
    let hits = 0;
    server.use(
      http.get("/api/staff/me", () => {
        hits += 1;
        return HttpResponse.json({ id: 1, username: "admin", isadmin: true });
      }),
    );

    root.staffAuth.ensureProfileLoaded();
    root.staffAuth.ensureProfileLoaded();
    root.staffAuth.ensureProfileLoaded();

    // Let the single in-flight request settle.
    await new Promise((r) => setTimeout(r, 0));
    await new Promise((r) => setTimeout(r, 0));

    expect(hits).toBe(1);
    expect(root.staffAuth.isAdmin).toBe(true);
  });

  it("logout resolves the profile state to 'loaded' (known-absent), not 'loading'", async () => {
    server.use(
      http.post("/api/staff/login", () => HttpResponse.json({ ok: true, csrfToken: "c" })),
      http.get("/api/staff/me", () => HttpResponse.json({ id: 1, username: "admin", isadmin: true })),
      http.post("/api/staff/logout", () => HttpResponse.json({ ok: true })),
    );

    await root.staffAuth.login({ username: "admin", password: "Admin123!" });
    expect(root.staffAuth.isAdmin).toBe(true);

    await root.staffAuth.logout();
    expect(root.staffAuth.profileLoaded).toBe("loaded");
    expect(root.staffAuth.isAuthenticated).toBe(false);
    expect(root.staffAuth.isAdmin).toBe(false);
  });
});
