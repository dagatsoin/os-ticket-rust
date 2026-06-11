import { beforeEach, describe, expect, it } from "vitest";
import { server, http, HttpResponse, readMultipart, type CapturedMultipart } from "../test/mockServer";
import { ApiClient } from "../api/apiClient";
import { StaffTicketStore } from "./StaffTicketStore";

/** A real-byte File so it survives a multipart serialize under jsdom. */
function fileOf(name: string, size: number, type = "application/pdf"): File {
  return new File(["a".repeat(size)], name, { type });
}

/**
 * StaffTicketStore TDD target (TS-M1-C4 AC-5; AC-3 refetch-on-reply behaviour).
 * Covers queue loading, detail loading (ordered, all M/R/N), and the reply flow
 * that refreshes the thread FROM THE REPLY RESPONSE (no optimistic append).
 */
describe("StaffTicketStore", () => {
  let store: StaffTicketStore;

  beforeEach(() => {
    store = new StaffTicketStore(new ApiClient({ realm: "staff" }));
  });

  it("loads the open-tickets queue (newest first, as served)", async () => {
    server.use(
      http.get("/api/staff/tickets", () =>
        HttpResponse.json([
          { id: 2, number: 200002, subject: "Newer", email: "b@x.io", created: "2026-06-02T00:00:00Z" },
          { id: 1, number: 100001, subject: "Older", email: "a@x.io", created: "2026-06-01T00:00:00Z" },
        ]),
      ),
    );

    await store.loadQueue();

    expect(store.queue.map((t) => t.number)).toEqual([200002, 100001]);
    expect(store.queue[0].subject).toBe("Newer");
    expect(store.loadingQueue).toBe(false);
  });

  it("loads a ticket detail with its thread entries in served order", async () => {
    server.use(
      http.get("/api/staff/tickets/1", () =>
        HttpResponse.json({
          id: 1,
          number: 100001,
          subject: "Help",
          email: "a@x.io",
          name: "Alice",
          status: "open",
          created: "2026-06-01T00:00:00Z",
          entries: [
            { id: 10, threadType: "M", poster: "Alice", body: "It broke" },
            { id: 11, threadType: "N", poster: "Agent", body: "internal note" },
            { id: 12, threadType: "R", poster: "Agent", body: "On it" },
          ],
        }),
      ),
    );

    await store.loadDetail(1);

    expect(store.detail?.number).toBe(100001);
    expect(store.detail?.entries.map((e) => e.id)).toEqual([10, 11, 12]);
    // Staff see all types including the internal N note.
    expect(store.detail?.entries.map((e) => e.threadType)).toEqual(["M", "N", "R"]);
  });

  it("replies and refreshes the thread FROM the reply response (no optimistic append)", async () => {
    let postedBody: unknown = null;
    server.use(
      http.get("/api/staff/tickets/1", () =>
        HttpResponse.json({
          id: 1, number: 100001, subject: "Help", email: "a@x.io", name: "Alice",
          status: "open", created: "2026-06-01T00:00:00Z",
          entries: [{ id: 10, threadType: "M", poster: "Alice", body: "It broke" }],
        }),
      ),
      http.post("/api/staff/tickets/1/reply", async ({ request }) => {
        postedBody = await request.json();
        // The reply response is the authoritative refreshed detail.
        return HttpResponse.json({
          id: 1, number: 100001, subject: "Help", email: "a@x.io", name: "Alice",
          status: "open", created: "2026-06-01T00:00:00Z",
          entries: [
            { id: 10, threadType: "M", poster: "Alice", body: "It broke" },
            { id: 13, threadType: "R", poster: "Agent", body: "Fixed it" },
          ],
        });
      }),
    );

    await store.loadDetail(1);
    expect(store.detail?.entries).toHaveLength(1);

    store.setReplyBody("Fixed it");
    await store.reply(1);

    // The request used the backend's `body` key.
    expect(postedBody).toEqual({ body: "Fixed it" });
    // The thread came from the reply RESPONSE, not a local append.
    expect(store.detail?.entries).toHaveLength(2);
    expect(store.detail?.entries[1]).toMatchObject({ id: 13, threadType: "R", body: "Fixed it" });
    expect(store.replying).toBe(false);
  });

  it("clears the detail when switching context", async () => {
    server.use(
      http.get("/api/staff/tickets/1", () =>
        HttpResponse.json({
          id: 1, number: 100001, subject: "Help", email: "a@x.io", name: "Alice",
          status: "open", created: "2026-06-01T00:00:00Z", entries: [],
        }),
      ),
    );
    await store.loadDetail(1);
    expect(store.detail).not.toBeNull();
    store.clearDetail();
    expect(store.detail).toBeNull();
  });
});

/**
 * StaffTicketStore — canned-response + multipart reply extensions (TS-M2-D3).
 * Covers the canned list, selecting a response (fetch + textarea fill +
 * carried chips + retained cannedId), and the multipart reply assembly.
 */
describe("StaffTicketStore — canned + multipart reply (TS-M2-D3)", () => {
  let store: StaffTicketStore;

  beforeEach(() => {
    store = new StaffTicketStore(new ApiClient({ realm: "staff" }));
  });

  it("loads the canned-response list (enabled + dept-scoped, as served)", async () => {
    server.use(
      http.get("/api/staff/tickets/1/canned", () =>
        HttpResponse.json([{ id: 7, title: "Acknowledge receipt" }]),
      ),
    );

    await store.loadCanned(1);

    expect(store.cannedList).toEqual([{ id: 7, title: "Acknowledge receipt" }]);
  });

  it("selectCanned fetches the detail, fills the reply body (substituted), carries chips + retains cannedId", async () => {
    server.use(
      http.get("/api/staff/tickets/1/canned/7", () =>
        HttpResponse.json({
          body: "Hi, we received ticket 100001.",
          attachments: [{ id: 50, name: "policy.txt", size: 12, mime: "text/plain" }],
        }),
      ),
    );

    await store.selectCanned(1, 7);

    // The textarea is filled with the SUBSTITUTED body (no literal %{...}).
    expect(store.replyBody).toBe("Hi, we received ticket 100001.");
    expect(store.replyBody).not.toMatch(/%\{/);
    // The carried attachments are surfaced read-only for the composer chips.
    expect(store.cannedAttachments).toEqual([
      { id: 50, name: "policy.txt", size: 12, mime: "text/plain" },
    ]);
    // The cannedId is retained for the multipart POST (server re-renders + binds).
    expect(store.selectedCannedId).toBe(7);
  });

  it("clearing the canned selection drops the body, chips and cannedId", async () => {
    server.use(
      http.get("/api/staff/tickets/1/canned/7", () =>
        HttpResponse.json({ body: "x", attachments: [] }),
      ),
    );
    await store.selectCanned(1, 7);
    store.clearCanned();
    expect(store.selectedCannedId).toBeNull();
    expect(store.cannedAttachments).toEqual([]);
  });

  it("posts a multipart reply with body + cannedId + attachment, then refreshes from the response", async () => {
    let captured: CapturedMultipart | null = null;
    server.use(
      http.post("/api/staff/tickets/1/reply", async ({ request }) => {
        captured = await readMultipart(request);
        return HttpResponse.json({
          id: 1, number: 100001, subject: "Help", email: "a@x.io", name: "Alice",
          status: "open", created: "2026-06-01T00:00:00Z", isanswered: true,
          entries: [
            { id: 10, threadType: "M", poster: "Alice", body: "It broke" },
            {
              id: 13, threadType: "R", poster: "Agent", body: "Hi, we received ticket 100001.",
              attachments: [{ id: 50, name: "policy.txt", size: 12, mime: "text/plain" }],
            },
          ],
        });
      }),
    );

    store.setReplyBody("Hi, we received ticket 100001.");
    store.setSelectedCannedId(7);
    store.setReplyFile(fileOf("agent-note.pdf", 100));
    await store.reply(1);

    expect(captured).not.toBeNull();
    expect(captured!.contentType).toMatch(/^multipart\/form-data; boundary=/);
    expect(captured!.fields.body).toBe("Hi, we received ticket 100001.");
    expect(captured!.fields.cannedId).toBe("7");
    expect(captured!.files.attachment?.name).toBe("agent-note.pdf");
    // Thread refreshed from the reply response (existing pattern) incl. isanswered.
    expect(store.detail?.isanswered).toBe(true);
    expect(store.detail?.entries).toHaveLength(2);
    // The composer is reset after a successful post.
    expect(store.replyBody).toBe("");
    expect(store.selectedCannedId).toBeNull();
    expect(store.replyFile).toBeNull();
  });

  it("a plain reply (no canned, no file) still posts the M1 JSON body", async () => {
    let asJson: unknown = null;
    server.use(
      http.post("/api/staff/tickets/1/reply", async ({ request }) => {
        expect(request.headers.get("Content-Type")).toMatch(/application\/json/);
        asJson = await request.json();
        return HttpResponse.json({
          id: 1, number: 100001, subject: "Help", email: "a@x.io", name: "Alice",
          status: "open", created: "2026-06-01T00:00:00Z", isanswered: true,
          entries: [{ id: 13, threadType: "R", poster: "Agent", body: "Fixed it" }],
        });
      }),
    );

    store.setReplyBody("Fixed it");
    await store.reply(1);

    expect(asJson).toEqual({ body: "Fixed it" });
    expect(store.detail?.entries).toHaveLength(1);
  });

  it("rejects an own-file that fails the shared pre-check (reuses validateAttachment)", () => {
    store.setReplyFile(fileOf("evil.exe", 100, "application/octet-stream"));
    expect(store.replyFileError).toMatch(/invalid file type/i);
    expect(store.canReply).toBe(false);
  });
});
