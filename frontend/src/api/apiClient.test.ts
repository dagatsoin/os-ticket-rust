import { beforeEach, describe, expect, it, vi } from "vitest";
import { server, http, HttpResponse } from "../test/mockServer";
import { ApiClient } from "./apiClient";
import { ApiError } from "./types";

/**
 * apiClient unit tests (TDD target). Covers:
 *  - CSRF header injection from the realm XSRF cookie on mutating requests (AC-4)
 *  - 401 -> realm login redirect (AC-4)
 *  - shared error-envelope parsing into top-level + per-field errors (AC-5)
 */

function setCookie(name: string, value: string) {
  document.cookie = `${name}=${value}; path=/`;
}

function clearCookies() {
  for (const c of document.cookie.split(";")) {
    const name = c.split("=")[0].trim();
    if (name) document.cookie = `${name}=; path=/; expires=Thu, 01 Jan 1970 00:00:00 GMT`;
  }
}

describe("ApiClient", () => {
  beforeEach(() => {
    clearCookies();
    vi.restoreAllMocks();
  });

  it("injects X-CSRFToken from the realm XSRF cookie on a mutating (POST) request", async () => {
    setCookie("XSRF-TOKEN-STAFF", "staff-token-123");
    let receivedHeader: string | null = null;

    server.use(
      http.post("/api/staff/tickets/1/reply", ({ request }) => {
        receivedHeader = request.headers.get("X-CSRFToken");
        return HttpResponse.json({ ok: true });
      }),
    );

    const client = new ApiClient({ realm: "staff" });
    await client.post("/api/staff/tickets/1/reply", { body: "hi" });

    expect(receivedHeader).toBe("staff-token-123");
  });

  it("reads the CLIENT cookie for the client realm and ignores the staff cookie", async () => {
    setCookie("XSRF-TOKEN-STAFF", "staff-token");
    setCookie("XSRF-TOKEN-CLIENT", "client-token-xyz");
    let receivedHeader: string | null = null;

    server.use(
      http.post("/api/tickets/1/reply", ({ request }) => {
        receivedHeader = request.headers.get("X-CSRFToken");
        return HttpResponse.json({ ok: true });
      }),
    );

    const client = new ApiClient({ realm: "client" });
    await client.post("/api/tickets/1/reply", {});

    expect(receivedHeader).toBe("client-token-xyz");
  });

  it("does NOT inject X-CSRFToken on a non-mutating (GET) request", async () => {
    setCookie("XSRF-TOKEN-STAFF", "staff-token");
    let receivedHeader: string | null = "unset";

    server.use(
      http.get("/api/staff/me", ({ request }) => {
        receivedHeader = request.headers.get("X-CSRFToken");
        return HttpResponse.json({ id: 1 });
      }),
    );

    const client = new ApiClient({ realm: "staff" });
    await client.get("/api/staff/me");

    expect(receivedHeader).toBeNull();
  });

  it("redirects to the staff login on a 401 from the staff realm", async () => {
    const redirect = vi.fn();
    server.use(
      http.get("/api/staff/me", () =>
        HttpResponse.json({ error: { message: "unauthenticated" } }, { status: 401 }),
      ),
    );

    const client = new ApiClient({ realm: "staff", onUnauthorized: redirect });
    await expect(client.get("/api/staff/me")).rejects.toBeInstanceOf(ApiError);

    expect(redirect).toHaveBeenCalledWith("/staff/login");
  });

  it("redirects to the client login on a 401 from the client realm", async () => {
    const redirect = vi.fn();
    server.use(
      http.get("/api/tickets", () =>
        HttpResponse.json({ error: { message: "unauthenticated" } }, { status: 401 }),
      ),
    );

    const client = new ApiClient({ realm: "client", onUnauthorized: redirect });
    await expect(client.get("/api/tickets")).rejects.toBeInstanceOf(ApiError);

    expect(redirect).toHaveBeenCalledWith("/tickets/login");
  });

  it("parses the shared error envelope into top-level message + per-field errors (422)", async () => {
    server.use(
      http.post("/api/tickets", () =>
        HttpResponse.json(
          {
            error: {
              message: "Validation failed",
              fields: { email: "Invalid email", subject: "Required" },
            },
          },
          { status: 422 },
        ),
      ),
    );

    const client = new ApiClient({ realm: "client" });
    try {
      await client.post("/api/tickets", {});
      throw new Error("should have thrown");
    } catch (e) {
      expect(e).toBeInstanceOf(ApiError);
      const err = e as ApiError;
      expect(err.status).toBe(422);
      expect(err.message).toBe("Validation failed");
      expect(err.fields).toEqual({ email: "Invalid email", subject: "Required" });
    }
  });

  it("returns parsed JSON on a successful request", async () => {
    server.use(
      http.get("/api/health", () => HttpResponse.json({ status: "ok", db: "up" })),
    );
    const client = new ApiClient({ realm: "client" });
    const data = await client.get<{ status: string; db: string }>("/api/health");
    expect(data).toEqual({ status: "ok", db: "up" });
  });
});
