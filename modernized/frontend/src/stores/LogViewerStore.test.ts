import { beforeEach, describe, expect, it } from "vitest";
import { server, http, HttpResponse } from "../test/mockServer";
import { RootStore } from "./RootStore";

/**
 * TS-M4-G3 business logic (TDD): query-param serialization (type + date span),
 * filter apply resets paging, sort toggle, detail fetch, selection + bulk
 * delete, and the purge trigger.
 */

function listPayload(items: unknown[], total = items.length, page = 1) {
  return { items, total, page, per_page: 25 };
}

const ROWS = [
  { id: 1, log_type: "Error", title: "Login failure", created: "2024-05-01T00:00:00Z", ip_address: "127.0.0.1" },
  { id: 2, log_type: "Debug", title: "Group created", created: "2024-05-02T00:00:00Z", ip_address: "127.0.0.1" },
];

describe("LogViewerStore (TS-M4-G3)", () => {
  let root: RootStore;
  beforeEach(() => {
    root = new RootStore();
  });

  it("serializes sort/order/page/per_page and omits empty filters", async () => {
    let url: URL | undefined;
    server.use(
      http.get("/api/staff/admin/logs", ({ request }) => {
        url = new URL(request.url);
        return HttpResponse.json(listPayload(ROWS));
      }),
    );
    await root.logs.loadList();
    expect(url?.searchParams.get("sort")).toBe("created");
    expect(url?.searchParams.get("order")).toBe("DESC");
    expect(url?.searchParams.has("type")).toBe(false);
    expect(url?.searchParams.has("from")).toBe(false);
    expect(root.logs.rows).toHaveLength(2);
  });

  it("applyFilters sends type + date span and resets to page 1", async () => {
    let url: URL | undefined;
    server.use(
      http.get("/api/staff/admin/logs", ({ request }) => {
        url = new URL(request.url);
        return HttpResponse.json(listPayload([ROWS[0]], 1));
      }),
    );
    const s = root.logs;
    s.page = 5;
    s.setFilterType("Error");
    s.setFilterFrom("2024-05-01");
    s.setFilterTo("2024-05-01");
    await s.applyFilters();
    expect(url?.searchParams.get("type")).toBe("Error");
    expect(url?.searchParams.get("from")).toBe("2024-05-01");
    expect(url?.searchParams.get("to")).toBe("2024-05-01");
    expect(url?.searchParams.get("page")).toBe("1");
    expect(s.page).toBe(1);
  });

  it("setSort toggles order and reloads", async () => {
    server.use(http.get("/api/staff/admin/logs", () => HttpResponse.json(listPayload(ROWS))));
    await root.logs.loadList();
    await root.logs.setSort("created"); // same key → toggle DESC→ASC
    expect(root.logs.order).toBe("ASC");
  });

  it("openDetail fetches the full body (content AJAX, FS-033.8)", async () => {
    server.use(
      http.get("/api/staff/admin/logs/1", () =>
        HttpResponse.json({ ...ROWS[0], log: "Full body text here" }),
      ),
    );
    await root.logs.openDetail(1);
    expect(root.logs.detailOpen).toBe(true);
    expect(root.logs.detail?.log).toBe("Full body text here");
  });

  it("bulk delete posts the selected ids and clears selection", async () => {
    let body: Record<string, unknown> | undefined;
    server.use(
      http.get("/api/staff/admin/logs", () => HttpResponse.json(listPayload(ROWS))),
      http.post("/api/staff/admin/logs/delete", async ({ request }) => {
        body = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json({ affected: 2 });
      }),
    );
    const s = root.logs;
    await s.loadList();
    s.toggleSelect(1);
    s.toggleSelect(2);
    const ok = await s.deleteSelected();
    expect(ok).toBe(true);
    expect(body).toMatchObject({ ids: [1, 2] });
    expect(s.selectedIds).toHaveLength(0);
  });

  it("purge triggers the sweep endpoint and reloads", async () => {
    let purged = false;
    server.use(
      http.get("/api/staff/admin/logs", () => HttpResponse.json(listPayload([ROWS[1]], 1))),
      http.post("/api/dev/purge-logs", () => {
        purged = true;
        return HttpResponse.json({ affected: 1 });
      }),
    );
    const ok = await root.logs.purge();
    expect(ok).toBe(true);
    expect(purged).toBe(true);
    expect(root.snackbar.severity).toBe("success");
  });
});
