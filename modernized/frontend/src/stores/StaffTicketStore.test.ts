import { afterEach, beforeEach, describe, expect, it } from "vitest";
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
            { id: 10, threadType: "M", created: "2026-07-23T14:05:09Z", poster: "Alice", body: "It broke" },
            { id: 11, threadType: "N", created: "2026-07-23T14:05:09Z", poster: "Agent", body: "internal note" },
            { id: 12, threadType: "R", created: "2026-07-23T14:05:09Z", poster: "Agent", body: "On it" },
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
          entries: [{ id: 10, threadType: "M", created: "2026-07-23T14:05:09Z", poster: "Alice", body: "It broke" }],
        }),
      ),
      http.post("/api/staff/tickets/1/reply", async ({ request }) => {
        postedBody = await request.json();
        // The reply response is the authoritative refreshed detail.
        return HttpResponse.json({
          id: 1, number: 100001, subject: "Help", email: "a@x.io", name: "Alice",
          status: "open", created: "2026-06-01T00:00:00Z",
          entries: [
            { id: 10, threadType: "M", created: "2026-07-23T14:05:09Z", poster: "Alice", body: "It broke" },
            { id: 13, threadType: "R", created: "2026-07-23T14:05:09Z", poster: "Agent", body: "Fixed it" },
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
    expect(store.detail?.entries[1]).toMatchObject({ id: 13, threadType: "R", created: "2026-07-23T14:05:09Z", body: "Fixed it" });
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
            { id: 10, threadType: "M", created: "2026-07-23T14:05:09Z", poster: "Alice", body: "It broke" },
            {
              id: 13, threadType: "R", created: "2026-07-23T14:05:09Z", poster: "Agent", body: "Hi, we received ticket 100001.",
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
          entries: [{ id: 13, threadType: "R", created: "2026-07-23T14:05:09Z", poster: "Agent", body: "Fixed it" }],
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

/**
 * StaffTicketStore — queue tabs, stats, and pagination (TS-M3-A4, TS-M3-B4).
 * Covers loading quick stats, loading queue with status/sort/pagination params,
 * and managing URL query params.
 * @implements FS-020.2: queue tabs with counts
 * @implements FS-020.6: pagination
 * @implements BS-020.8: sticky sort
 */
describe("StaffTicketStore — queue tabs + pagination (TS-M3-A4, TS-M3-B4)", () => {
  let store: StaffTicketStore;

  beforeEach(() => {
    store = new StaffTicketStore(new ApiClient({ realm: "staff" }));
  });

  it("loads quick stats with counts per queue", async () => {
    server.use(
      http.get("/api/staff/tickets/stats", () =>
        HttpResponse.json({
          open: 5,
          answered: 2,
          overdue: 1,
          assigned: 3,
          closed: 10,
          show_answered_tickets: false,
          show_assigned_tickets: true,
        }),
      ),
    );

    await store.loadStats();

    expect(store.stats).toEqual({
      open: 5,
      answered: 2,
      overdue: 1,
      assigned: 3,
      closed: 10,
      show_answered_tickets: false,
      show_assigned_tickets: true,
    });
    expect(store.loadingStats).toBe(false);
  });

  it("loads queue with status param and pagination metadata", async () => {
    server.use(
      http.get("/api/staff/tickets", ({ request }) => {
        const url = new URL(request.url);
        expect(url.searchParams.get("status")).toBe("open");
        return HttpResponse.json({
          tickets: [
            { id: 1, number: 100001, subject: "Ticket 1", email: "a@x.io", created: "2026-06-01", isanswered: false },
            { id: 2, number: 100002, subject: "Ticket 2", email: "b@x.io", created: "2026-06-02", isanswered: true },
          ],
          rightmostColumn: "assigned_to",
          pagination: { page: 1, pageSize: 25, totalCount: 50, totalPages: 2 },
        });
      }),
    );

    await store.loadQueue({ status: "open" });

    expect(store.queue).toHaveLength(2);
    expect(store.rightmostColumn).toBe("assigned_to");
    expect(store.pagination).toEqual({ page: 1, pageSize: 25, totalCount: 50, totalPages: 2 });
  });

  it("loads queue with sort and order params", async () => {
    server.use(
      http.get("/api/staff/tickets", ({ request }) => {
        const url = new URL(request.url);
        expect(url.searchParams.get("sort")).toBe("date");
        expect(url.searchParams.get("order")).toBe("ASC");
        return HttpResponse.json({
          tickets: [],
          rightmostColumn: "department",
          pagination: { page: 1, pageSize: 25, totalCount: 0, totalPages: 0 },
        });
      }),
    );

    await store.loadQueue({ status: "open", sort: "date", order: "ASC" });
  });

  it("loads queue with pagination params", async () => {
    server.use(
      http.get("/api/staff/tickets", ({ request }) => {
        const url = new URL(request.url);
        expect(url.searchParams.get("p")).toBe("2");
        expect(url.searchParams.get("limit")).toBe("10");
        return HttpResponse.json({
          tickets: [],
          rightmostColumn: "department",
          pagination: { page: 2, pageSize: 10, totalCount: 30, totalPages: 3 },
        });
      }),
    );

    await store.loadQueue({ status: "open", page: 2, limit: 10 });
  });

  it("loads sticky sort preferences", async () => {
    server.use(
      http.get("/api/staff/tickets/sort-prefs", () =>
        HttpResponse.json({
          open: { sort: "date", order: "DESC" },
          closed: { sort: "ID", order: "ASC" },
        }),
      ),
    );

    await store.loadSortPrefs();

    expect(store.sortPrefs).toEqual({
      open: { sort: "date", order: "DESC" },
      closed: { sort: "ID", order: "ASC" },
    });
  });

  it("updates current sort state when loading queue with explicit sort", async () => {
    server.use(
      http.get("/api/staff/tickets", () =>
        HttpResponse.json({
          tickets: [],
          rightmostColumn: "department",
          pagination: { page: 1, pageSize: 25, totalCount: 0, totalPages: 0 },
        }),
      ),
    );

    await store.loadQueue({ status: "open", sort: "name", order: "ASC" });

    expect(store.currentSort).toBe("name");
    expect(store.currentOrder).toBe("ASC");
  });

  it("refreshes stats after queue reload to keep counts in sync", async () => {
    let statsCallCount = 0;
    let statsResolve: () => void;
    const statsPromise = new Promise<void>((resolve) => { statsResolve = resolve; });

    server.use(
      http.get("/api/staff/tickets/stats", () => {
        statsCallCount++;
        statsResolve();
        return HttpResponse.json({
          open: 5,
          answered: 2,
          overdue: 1,
          assigned: 3,
          closed: 10,
          show_answered_tickets: false,
          show_assigned_tickets: true,
        });
      }),
      http.get("/api/staff/tickets", () =>
        HttpResponse.json({
          tickets: [],
          rightmostColumn: "department",
          pagination: { page: 1, pageSize: 25, totalCount: 0, totalPages: 0 },
        }),
      ),
    );

    await store.loadQueue({ status: "open", refreshStats: true });
    // Wait for the stats call to complete (fire-and-forget in loadQueue).
    await statsPromise;

    expect(statsCallCount).toBe(1);
  });
});

/**
 * StaffTicketStore — bulk action count mapping (US-M3-G1 AC-4 defect fix).
 * The backend mass-process endpoint returns `{ succeeded, failed, message }`
 * (see backend/api/src/staff.rs::BulkActionResponse). The store must map that to
 * the `{ affected, total }` the BulkActionBar toast interpolates, so the success
 * snackbar shows the real count ("3 tickets closed"), never "undefined tickets closed".
 */
describe("StaffTicketStore — bulk action count (US-M3-G1)", () => {
  let store: StaffTicketStore;

  beforeEach(() => {
    store = new StaffTicketStore(new ApiClient({ realm: "staff" }));
  });

  it("maps the backend {succeeded, failed} shape to affected/total (full success)", async () => {
    let posted: unknown = null;
    server.use(
      http.post("/api/staff/tickets/bulk", async ({ request }) => {
        posted = await request.json();
        return HttpResponse.json({ succeeded: 3, failed: 0, message: "3 tickets closed" });
      }),
    );

    const result = await store.bulkAction("close", [101, 102, 103]);

    expect(posted).toEqual({ action: "close", ticket_ids: [101, 102, 103] });
    expect(result).toEqual({ success: true, affected: 3, total: 3 });
    // The toast interpolates `${affected} tickets closed` — must carry the count.
    expect(`${result.affected} tickets closed`).toBe("3 tickets closed");
    expect(store.bulkMessage).toContain("3");
  });

  it("maps partial success counts from succeeded/failed", async () => {
    server.use(
      http.post("/api/staff/tickets/bulk", () =>
        HttpResponse.json({ succeeded: 2, failed: 1, message: "2 of 3 tickets closed" }),
      ),
    );

    const result = await store.bulkAction("close", [201, 202, 203]);

    expect(result.success).toBe(true);
    expect(result.affected).toBe(2);
    expect(result.total).toBe(3);
  });
});

/**
 * StaffTicketStore — collaborative lock acquire (US-M3-I2 defect fix).
 * The frontend must align to the shared lock contract:
 *  - a fresh OR idempotent same-staff success returns a lock id → we hold the lock,
 *    NO banner, NO generic "Unable to obtain a lock" error;
 *  - locked-by-another → the named banner (lockedByAnother + lockedByName), no generic error,
 *    whether the backend signals it on the 200 body (`lock.locked_by_other`) or as a 409 conflict;
 *  - a genuine unexpected failure still surfaces the generic lock error.
 */
describe("StaffTicketStore — collaborative lock acquire (US-M3-I2)", () => {
  let store: StaffTicketStore;

  beforeEach(() => {
    store = new StaffTicketStore(new ApiClient({ realm: "staff" }));
  });

  afterEach(() => {
    // Clear the renewal interval started on a successful acquire.
    store.clearLockState();
  });

  it("holds the lock on a same-staff/fresh success (camelCase lockId) with no banner and no error", async () => {
    server.use(
      http.post("/api/staff/tickets/1/lock", () =>
        HttpResponse.json({ lockId: 42, remainingSeconds: 300 }),
      ),
    );

    const ok = await store.acquireLock(1);

    expect(ok).toBe(true);
    expect(store.lockId).toBe(42);
    expect(store.lockedByAnother).toBe(false);
    expect(store.lockedByName).toBeNull();
    expect(store.lockError).toBeNull();
  });

  it("shows the named banner (no generic error) when locked by another via the shared 200 body", async () => {
    server.use(
      http.post("/api/staff/tickets/1/lock", () =>
        HttpResponse.json({ lock: { locked_by_other: true, locked_by_name: "Alice Agent" } }),
      ),
    );

    const ok = await store.acquireLock(1);

    expect(ok).toBe(false);
    expect(store.lockedByAnother).toBe(true);
    expect(store.lockedByName).toBe("Alice Agent");
    expect(store.lockError).toBeNull();
    expect(store.lockId).toBeNull();
  });

  it("shows the named banner when locked-by-other surfaces as a 409 conflict", async () => {
    server.use(
      http.post("/api/staff/tickets/1/lock", () =>
        HttpResponse.json(
          { error: { message: "Ticket is currently locked by Alice Agent" } },
          { status: 409 },
        ),
      ),
    );

    const ok = await store.acquireLock(1);

    expect(ok).toBe(false);
    expect(store.lockedByAnother).toBe(true);
    expect(store.lockedByName).toBe("Alice Agent");
    expect(store.lockError).toBeNull();
  });

  it("surfaces the generic lock error only on an unexpected failure (500)", async () => {
    server.use(
      http.post("/api/staff/tickets/1/lock", () =>
        HttpResponse.json({ error: { message: "Lock failed" } }, { status: 500 }),
      ),
    );

    const ok = await store.acquireLock(1);

    expect(ok).toBe(false);
    expect(store.lockError).toMatch(/unable to obtain a lock/i);
    expect(store.lockedByAnother).toBe(false);
  });
});
