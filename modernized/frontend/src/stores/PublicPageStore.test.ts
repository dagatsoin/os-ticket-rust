import { beforeEach, describe, expect, it } from "vitest";
import { server, http, HttpResponse } from "../test/mockServer";
import { RootStore } from "./RootStore";

/**
 * PublicPageStore business logic (TDD): fetches a public site page by slug via
 * GET /api/pages/:slug (unauthenticated), distinguishing a 404 (not-found) from a
 * generic error, and clearing prior state between loads.
 */

describe("PublicPageStore (public site pages)", () => {
  let root: RootStore;
  beforeEach(() => {
    root = new RootStore();
  });

  it("loads a page by slug and exposes name + body", async () => {
    server.use(
      http.get("/api/pages/terms-of-service", () =>
        HttpResponse.json({ name: "Terms Of Service", body: "<p>Be nice.</p>" }),
      ),
    );
    await root.publicPage.load("terms-of-service");

    expect(root.publicPage.page).toEqual({ name: "Terms Of Service", body: "<p>Be nice.</p>" });
    expect(root.publicPage.notFound).toBe(false);
    expect(root.publicPage.error).toBeNull();
  });

  it("flags notFound on a 404 without a generic error", async () => {
    server.use(
      http.get("/api/pages/missing", () =>
        HttpResponse.json({ error: { message: "Page not found" } }, { status: 404 }),
      ),
    );
    await root.publicPage.load("missing");

    expect(root.publicPage.page).toBeNull();
    expect(root.publicPage.notFound).toBe(true);
    expect(root.publicPage.error).toBeNull();
  });

  it("surfaces a generic error on a non-404 failure", async () => {
    server.use(
      http.get("/api/pages/boom", () =>
        HttpResponse.json({ error: { message: "server exploded" } }, { status: 500 }),
      ),
    );
    await root.publicPage.load("boom");

    expect(root.publicPage.page).toBeNull();
    expect(root.publicPage.notFound).toBe(false);
    expect(root.publicPage.error).toBe("server exploded");
  });

  it("clears prior state when loading a new slug", async () => {
    server.use(
      http.get("/api/pages/first", () =>
        HttpResponse.json({ name: "First", body: "one" }),
      ),
      http.get("/api/pages/second", () =>
        HttpResponse.json({ error: { message: "Page not found" } }, { status: 404 }),
      ),
    );
    await root.publicPage.load("first");
    expect(root.publicPage.page).not.toBeNull();

    await root.publicPage.load("second");
    expect(root.publicPage.page).toBeNull();
    expect(root.publicPage.notFound).toBe(true);
  });
});
