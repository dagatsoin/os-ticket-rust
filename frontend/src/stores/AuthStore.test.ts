import { beforeEach, describe, expect, it } from "vitest";
import { server, http, HttpResponse } from "../test/mockServer";
import { RootStore } from "./RootStore";
import { ApiError } from "../api/types";

/**
 * Auth store TDD target. Two independent realm stores that never share state.
 * Covers login success/failure, logout, and authenticated state transitions.
 */
describe("Auth stores (staff + client)", () => {
  let root: RootStore;

  beforeEach(() => {
    root = new RootStore();
  });

  it("exposes two distinct auth stores for the two realms", () => {
    expect(root.staffAuth.realm).toBe("staff");
    expect(root.clientAuth.realm).toBe("client");
    expect(root.staffAuth).not.toBe(root.clientAuth);
  });

  it("starts unauthenticated with no current user", () => {
    expect(root.staffAuth.isAuthenticated).toBe(false);
    expect(root.staffAuth.user).toBeNull();
  });

  it("sets the user and authenticated flag on a successful staff login", async () => {
    server.use(
      http.post("/api/staff/login", () =>
        HttpResponse.json({ id: 7, name: "Agent Smith" }),
      ),
    );

    await root.staffAuth.login({ username: "smith", password: "pw" });

    expect(root.staffAuth.isAuthenticated).toBe(true);
    expect(root.staffAuth.user).toEqual({ id: 7, name: "Agent Smith" });
    // Realms are independent: a staff login does not authenticate the client realm.
    expect(root.clientAuth.isAuthenticated).toBe(false);
  });

  it("captures field errors and stays unauthenticated on a 422 login failure", async () => {
    server.use(
      http.post("/api/staff/login", () =>
        HttpResponse.json(
          { error: { message: "Invalid credentials", fields: { password: "Wrong" } } },
          { status: 422 },
        ),
      ),
    );

    await expect(
      root.staffAuth.login({ username: "smith", password: "bad" }),
    ).rejects.toBeInstanceOf(ApiError);

    expect(root.staffAuth.isAuthenticated).toBe(false);
    expect(root.staffAuth.error?.fields).toEqual({ password: "Wrong" });
  });

  it("clears the user on logout", async () => {
    server.use(
      http.post("/api/tickets/login", () => HttpResponse.json({ id: 3, email: "c@x.io" })),
      http.post("/api/tickets/logout", () => HttpResponse.json({ ok: true })),
    );

    await root.clientAuth.login({ email: "c@x.io", token: "t" });
    expect(root.clientAuth.isAuthenticated).toBe(true);

    await root.clientAuth.logout();
    expect(root.clientAuth.isAuthenticated).toBe(false);
    expect(root.clientAuth.user).toBeNull();
  });
});
