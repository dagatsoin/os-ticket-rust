import { beforeEach, describe, expect, it } from "vitest";
import { server, http, HttpResponse } from "../test/mockServer";
import { ApiClient } from "../api/apiClient";
import { OpenTicketStore } from "./OpenTicketStore";

/**
 * OpenTicketStore TDD target (TS-M1-B3 AC-4).
 * Covers client-side validation, submit gating, 201->confirmation, and the
 * 422->field-error mapping (backend is authoritative over client-side rules).
 */
describe("OpenTicketStore", () => {
  let store: OpenTicketStore;

  beforeEach(() => {
    store = new OpenTicketStore(new ApiClient({ realm: "client" }));
  });

  it("starts empty, invalid, and not submitted", () => {
    expect(store.values).toEqual({ name: "", email: "", subject: "", message: "" });
    expect(store.isValid).toBe(false);
    expect(store.canSubmit).toBe(false);
    expect(store.submitted).toBe(false);
    expect(store.ticketNumber).toBeNull();
  });

  it("flags required fields as invalid only after they are touched", () => {
    // Untouched -> no error surfaced yet (don't shout before the user types).
    expect(store.fieldErrors.name).toBeUndefined();
    store.touch("name");
    expect(store.fieldErrors.name).toBe("This field is required.");
  });

  it("validates email syntax (mirroring the backend rule)", () => {
    store.setValue("email", "not-an-email");
    store.touch("email");
    expect(store.fieldErrors.email).toBe("Enter a valid email address.");

    store.setValue("email", "user@example.com");
    expect(store.fieldErrors.email).toBeUndefined();
  });

  it("becomes valid + submittable once all required fields are filled correctly", () => {
    store.setValue("name", "Jane Doe");
    store.setValue("email", "jane@example.com");
    store.setValue("subject", "Help");
    store.setValue("message", "Something is broken.");
    expect(store.isValid).toBe(true);
    expect(store.canSubmit).toBe(true);
  });

  it("does not submit when invalid (surfaces all errors instead)", async () => {
    await store.submit();
    expect(store.submitted).toBe(false);
    expect(store.fieldErrors.name).toBe("This field is required.");
    expect(store.fieldErrors.email).toBe("This field is required.");
    expect(store.fieldErrors.subject).toBe("This field is required.");
    expect(store.fieldErrors.message).toBe("This field is required.");
  });

  it("posts to /api/tickets and stores the returned ticket number on 201", async () => {
    let received: unknown = null;
    server.use(
      http.post("/api/tickets", async ({ request }) => {
        received = await request.json();
        return HttpResponse.json({ ticketNumber: 123456 }, { status: 201 });
      }),
    );

    store.setValue("name", "Jane Doe");
    store.setValue("email", "jane@example.com");
    store.setValue("subject", "Help");
    store.setValue("message", "Something is broken.");
    await store.submit();

    expect(received).toEqual({
      name: "Jane Doe",
      email: "jane@example.com",
      subject: "Help",
      message: "Something is broken.",
    });
    expect(store.submitted).toBe(true);
    expect(store.ticketNumber).toBe("123456");
    expect(store.submitting).toBe(false);
  });

  it("maps a backend 422 envelope onto the corresponding fields", async () => {
    server.use(
      http.post("/api/tickets", () =>
        HttpResponse.json(
          {
            error: {
              message: "Please correct the highlighted fields",
              fields: { email: "Invalid email", subject: "Required" },
            },
          },
          { status: 422 },
        ),
      ),
    );

    store.setValue("name", "Jane Doe");
    store.setValue("email", "jane@example.com");
    store.setValue("subject", "Help");
    store.setValue("message", "Something is broken.");
    await store.submit();

    expect(store.submitted).toBe(false);
    expect(store.fieldErrors.email).toBe("Invalid email");
    expect(store.fieldErrors.subject).toBe("Required");
    expect(store.topError).toBe("Please correct the highlighted fields");
  });
});
