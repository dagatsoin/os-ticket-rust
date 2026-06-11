import { describe, expect, it, vi } from "vitest";
import { server, http, HttpResponse } from "../test/mockServer";
import {
  attachmentDownloadUrl,
  downloadAttachment,
  downloadErrorMessage,
  type Attachment,
} from "./downloadAttachment";

const FILE: Attachment = { id: 42, name: "policy.txt", size: 12, mime: "text/plain" };

describe("attachmentDownloadUrl (TS-M2-B2, §8)", () => {
  it("builds the session-bound client URL with no ticketId", () => {
    expect(attachmentDownloadUrl("client", 42)).toBe("/api/client/ticket/attachments/42");
  });

  it("builds the ticket-scoped staff URL", () => {
    expect(attachmentDownloadUrl("staff", 42, 7)).toBe("/api/staff/tickets/7/attachments/42");
  });

  it("throws when a staff download is missing the ticketId", () => {
    expect(() => attachmentDownloadUrl("staff", 42)).toThrow(/ticketId/);
  });
});

describe("downloadErrorMessage (TS-M2-B2, §9)", () => {
  it("maps 404/403 to an authorization-style message", () => {
    expect(downloadErrorMessage(404)).toMatch(/unavailable|not authorized/i);
    expect(downloadErrorMessage(403)).toMatch(/unavailable|not authorized/i);
  });

  it("maps a network failure (null) to a retry message", () => {
    expect(downloadErrorMessage(null)).toMatch(/try again/i);
  });
});

describe("downloadAttachment (TS-M2-B2, §9 fetch→blob→objectURL)", () => {
  function stubBrowser() {
    const triggerSave = vi.fn();
    const createObjectURL = vi.fn(() => "blob:mock-url");
    const revokeObjectURL = vi.fn();
    return { triggerSave, createObjectURL, revokeObjectURL };
  }

  it("fetches with credentials, makes an object-URL, and triggers a save on 200", async () => {
    let sentCredentials: RequestCredentials | undefined;
    server.use(
      http.get("/api/staff/tickets/7/attachments/42", ({ request }) => {
        sentCredentials = request.credentials;
        return new HttpResponse("file-bytes", {
          status: 200,
          headers: { "Content-Type": "text/plain" },
        });
      }),
    );

    const browser = stubBrowser();
    const result = await downloadAttachment("staff", FILE, 7, browser);

    expect(result).toEqual({ ok: true });
    expect(sentCredentials).toBe("include");
    expect(browser.createObjectURL).toHaveBeenCalledOnce();
    expect(browser.triggerSave).toHaveBeenCalledWith("blob:mock-url", "policy.txt");
  });

  it("uses the session-bound client route (no ticketId)", async () => {
    let hit = false;
    server.use(
      http.get("/api/client/ticket/attachments/42", () => {
        hit = true;
        return new HttpResponse("bytes", { status: 200 });
      }),
    );

    const browser = stubBrowser();
    const result = await downloadAttachment("client", FILE, undefined, browser);

    expect(hit).toBe(true);
    expect(result).toEqual({ ok: true });
  });

  it("returns a visible error on 404 and does NOT trigger a save (§9, EC-022.9)", async () => {
    server.use(
      http.get("/api/client/ticket/attachments/42", () =>
        HttpResponse.json({ error: { message: "Not found" } }, { status: 404 }),
      ),
    );

    const browser = stubBrowser();
    const result = await downloadAttachment("client", FILE, undefined, browser);

    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error).toMatch(/unavailable|not authorized/i);
    expect(browser.triggerSave).not.toHaveBeenCalled();
    expect(browser.createObjectURL).not.toHaveBeenCalled();
  });

  it("returns a visible error on 403", async () => {
    server.use(
      http.get("/api/staff/tickets/7/attachments/42", () =>
        HttpResponse.json({ error: { message: "Forbidden" } }, { status: 403 }),
      ),
    );

    const result = await downloadAttachment("staff", FILE, 7, stubBrowser());
    expect(result.ok).toBe(false);
  });
});
