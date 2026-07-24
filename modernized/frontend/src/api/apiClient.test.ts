import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  server,
  http,
  HttpResponse,
  multipartHandler,
  readMultipart,
  type CapturedMultipart,
} from "../test/mockServer";
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

  // --- TS-M2-A0: FormData / multipart support (ROADMAP Decisions M2 §13) ---
  describe("FormData / multipart body (TS-M2-A0)", () => {
    function makeTicketForm(): FormData {
      const form = new FormData();
      form.append("name", "Ada Lovelace");
      form.append("email", "ada@example.com");
      form.append("subject", "Printer down");
      form.append("message", "It will not print.");
      form.append(
        "attachment",
        new File(["log-bytes"], "error.log", { type: "text/plain" }),
      );
      return form;
    }

    it("AC-1: sends a FormData body raw, without JSON.stringify and without a Content-Type header", async () => {
      let contentType: string | null = "unset";
      let captured: CapturedMultipart | undefined;

      server.use(
        http.post("/api/tickets", async ({ request }) => {
          contentType = request.headers.get("Content-Type");
          captured = await readMultipart(request);
          return HttpResponse.json({ id: 7 });
        }),
      );

      const client = new ApiClient({ realm: "client" });
      const res = await client.post<{ id: number }>("/api/tickets", makeTicketForm());

      expect(res).toEqual({ id: 7 });
      // The body parsed as real multipart parts — proving it was NOT a JSON string.
      expect(captured?.fields.subject).toBe("Printer down");
      // The apiClient set no application/json; the browser/fetch supplied the
      // multipart boundary instead.
      expect(contentType).toMatch(/^multipart\/form-data; boundary=/);
      expect(contentType).not.toBe("application/json");
    });

    it("AC-2: a plain-object body still serializes with application/json (M1 path unchanged)", async () => {
      let contentType: string | null = null;
      let rawBody: string | null = null;

      server.use(
        http.post("/api/tickets", async ({ request }) => {
          contentType = request.headers.get("Content-Type");
          rawBody = await request.text();
          return HttpResponse.json({ id: 1 });
        }),
      );

      const client = new ApiClient({ realm: "client" });
      await client.post("/api/tickets", { subject: "Hi" });

      expect(contentType).toBe("application/json");
      expect(rawBody).toBe(JSON.stringify({ subject: "Hi" }));
    });

    it("AC-3: injects X-CSRFToken on a mutating FormData request", async () => {
      setCookie("XSRF-TOKEN-STAFF", "staff-token-456");
      let receivedHeader: string | null = null;

      server.use(
        http.post("/api/staff/tickets/1/reply", async ({ request }) => {
          receivedHeader = request.headers.get("X-CSRFToken");
          await request.formData();
          return HttpResponse.json({ ok: true });
        }),
      );

      const form = new FormData();
      form.append("body", "On it.");
      const client = new ApiClient({ realm: "staff" });
      await client.post("/api/staff/tickets/1/reply", form);

      expect(receivedHeader).toBe("staff-token-456");
    });

    it("AC-3: parses the shared error envelope (422) on a FormData request", async () => {
      server.use(
        http.post("/api/tickets", () =>
          HttpResponse.json(
            {
              error: {
                message: "Validation failed",
                fields: { attachment: "File too large" },
              },
            },
            { status: 422 },
          ),
        ),
      );

      const client = new ApiClient({ realm: "client" });
      try {
        await client.post("/api/tickets", makeTicketForm());
        throw new Error("should have thrown");
      } catch (e) {
        expect(e).toBeInstanceOf(ApiError);
        const err = e as ApiError;
        expect(err.status).toBe(422);
        expect(err.message).toBe("Validation failed");
        expect(err.fields).toEqual({ attachment: "File too large" });
      }
    });

    it("AC-3: a 401 on a FormData request triggers the realm-login redirect", async () => {
      const redirect = vi.fn();
      server.use(
        http.post("/api/staff/tickets/1/reply", () =>
          HttpResponse.json({ error: { message: "unauthenticated" } }, { status: 401 }),
        ),
      );

      const form = new FormData();
      form.append("body", "hi");
      const client = new ApiClient({ realm: "staff", onUnauthorized: redirect });
      await expect(client.post("/api/staff/tickets/1/reply", form)).rejects.toBeInstanceOf(
        ApiError,
      );

      expect(redirect).toHaveBeenCalledWith("/staff/login");
    });

    it("AC-4: the multipartHandler harness asserts on individual text + file parts", async () => {
      let seen: CapturedMultipart | undefined;

      server.use(
        multipartHandler("/api/tickets", (captured) => {
          seen = captured;
          return { id: 99 };
        }),
      );

      const client = new ApiClient({ realm: "client" });
      const res = await client.post<{ id: number }>("/api/tickets", makeTicketForm());

      expect(res).toEqual({ id: 99 });
      // Text parts.
      expect(seen?.fields).toMatchObject({
        name: "Ada Lovelace",
        email: "ada@example.com",
        subject: "Printer down",
        message: "It will not print.",
      });
      // File part metadata + decoded content.
      expect(seen?.files.attachment?.name).toBe("error.log");
      expect(seen?.files.attachment?.type).toBe("text/plain");
      expect(seen?.files.attachment?.size).toBeGreaterThan(0);
      expect(seen?.files.attachment?.text).toBe("log-bytes");
    });
  });
});
