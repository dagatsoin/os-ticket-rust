import { beforeEach, describe, expect, it } from "vitest";
import { server, http, HttpResponse, readMultipart, type CapturedMultipart } from "../test/mockServer";
import { ApiClient } from "../api/apiClient";
import { OpenTicketStore } from "./OpenTicketStore";

/**
 * Build a File whose ACTUAL byte length equals `size` (so it survives a real
 * fetch + multipart serialize, unlike a `size`-spoofed File which hangs undici).
 */
function fileOf(name: string, size: number, type = "application/pdf"): File {
  return new File(["a".repeat(size)], name, { type });
}

/** Fill the store with a valid M1 field set. */
function fillValid(store: OpenTicketStore): void {
  store.setValue("name", "Jane Doe");
  store.setValue("email", "jane@example.com");
  store.setValue("subject", "Help");
  store.setValue("message", "Something is broken.");
}

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

/**
 * TS-M2-A4: attachment file state, the shared client-side pre-check, FormData
 * assembly on submit, the local File.name on the confirmation chip, and the
 * backend 422 `attachment` field mapping (backend remains authoritative, §11).
 */
describe("OpenTicketStore — attachment (TS-M2-A4)", () => {
  let store: OpenTicketStore;

  beforeEach(() => {
    store = new OpenTicketStore(new ApiClient({ realm: "client" }));
  });

  it("starts with no file selected and no file error", () => {
    expect(store.file).toBeNull();
    expect(store.fileName).toBeNull();
    expect(store.fileError).toBeUndefined();
  });

  it("accepts a valid file and exposes its name (for the confirmation chip)", () => {
    store.setFile(fileOf("invoice.pdf", 1000));
    expect(store.file).not.toBeNull();
    expect(store.fileName).toBe("invoice.pdf");
    expect(store.fileError).toBeUndefined();
  });

  it("rejects a disallowed type via the shared pre-check and blocks submit", async () => {
    fillValid(store);
    store.setFile(fileOf("evil.exe", 1000, "application/octet-stream"));
    expect(store.fileError).toMatch(/invalid file type/i);
    expect(store.canSubmit).toBe(false);

    let hit = false;
    server.use(
      http.post("/api/tickets", () => {
        hit = true;
        return HttpResponse.json({ ticketNumber: 1 }, { status: 201 });
      }),
    );
    await store.submit();
    expect(hit).toBe(false);
    expect(store.submitted).toBe(false);
  });

  it("rejects an oversized permitted type as too big", () => {
    store.setFile(fileOf("big.pdf", 2 * 1024 * 1024));
    expect(store.fileError).toMatch(/too big/i);
  });

  it("clearing the file removes the file error and re-enables submit", () => {
    fillValid(store);
    store.setFile(fileOf("evil.exe", 1000));
    expect(store.canSubmit).toBe(false);
    store.clearFile();
    expect(store.file).toBeNull();
    expect(store.fileError).toBeUndefined();
    expect(store.canSubmit).toBe(true);
  });

  it("submits as multipart/form-data with the attachment part when a file is selected", async () => {
    let captured: CapturedMultipart | null = null;
    server.use(
      http.post("/api/tickets", async ({ request }) => {
        captured = await readMultipart(request);
        return HttpResponse.json({ ticketNumber: 654321 }, { status: 201 });
      }),
    );

    fillValid(store);
    store.setFile(fileOf("invoice.pdf", 1000));
    await store.submit();

    expect(captured).not.toBeNull();
    expect(captured!.contentType).toMatch(/^multipart\/form-data; boundary=/);
    expect(captured!.fields).toMatchObject({
      name: "Jane Doe",
      email: "jane@example.com",
      subject: "Help",
      message: "Something is broken.",
    });
    expect(captured!.files.attachment?.name).toBe("invoice.pdf");
    expect(store.submitted).toBe(true);
    expect(store.ticketNumber).toBe("654321");
    // The confirmation chip label comes from the local File.name (§11).
    expect(store.fileName).toBe("invoice.pdf");
  });

  it("still submits as JSON (no multipart) when no file is selected", async () => {
    let asJson: unknown = null;
    server.use(
      http.post("/api/tickets", async ({ request }) => {
        expect(request.headers.get("Content-Type")).toMatch(/application\/json/);
        asJson = await request.json();
        return HttpResponse.json({ ticketNumber: 111111 }, { status: 201 });
      }),
    );

    fillValid(store);
    await store.submit();

    expect(asJson).toEqual({
      name: "Jane Doe",
      email: "jane@example.com",
      subject: "Help",
      message: "Something is broken.",
    });
    expect(store.submitted).toBe(true);
  });

  it("maps a backend 422 `attachment` field error onto fileError", async () => {
    server.use(
      http.post("/api/tickets", () =>
        HttpResponse.json(
          { error: { message: "Attachment rejected", fields: { attachment: "File type not allowed" } } },
          { status: 422 },
        ),
      ),
    );

    fillValid(store);
    store.setFile(fileOf("invoice.pdf", 1000));
    await store.submit();

    expect(store.submitted).toBe(false);
    expect(store.fileError).toBe("File type not allowed");
    expect(store.topError).toBe("Attachment rejected");
  });
});
