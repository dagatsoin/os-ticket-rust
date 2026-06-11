import { beforeEach, describe, expect, it } from "vitest";
import { server, http, HttpResponse } from "../test/mockServer";
import { ApiClient } from "../api/apiClient";
import { StaffTicketStore } from "./StaffTicketStore";

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

    await store.reply(1, "Fixed it");

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
