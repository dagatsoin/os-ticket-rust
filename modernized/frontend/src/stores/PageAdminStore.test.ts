import { beforeEach, describe, expect, it } from "vitest";
import { server, http, HttpResponse } from "../test/mockServer";
import { RootStore } from "./RootStore";

/**
 * TS-M4-F2 business logic (TDD): list param serialization + pagination, sort
 * toggle (resets to page 1), form dirty, 422 → inline (duplicate name), mass
 * actions, and the in-use delete/disable guard predicate (BS-033.9).
 */

function listPayload(items: unknown[], total = items.length, page = 1) {
  return { items, pagination: { page, per_page: 25, total } };
}

const ROWS = [
  { id: 1, name: "Welcome Landing", type: "landing", isactive: true, in_use: false, created: "2024-01-01T00:00:00Z", updated: null },
  { id: 2, name: "Offline Notice", type: "offline", isactive: true, in_use: true, created: "2024-02-01T00:00:00Z", updated: null },
];

describe("PageAdminStore (TS-M4-F2)", () => {
  let root: RootStore;
  beforeEach(() => {
    root = new RootStore();
  });

  it("loads the list and pagination meta, serializing sort/order/page/per_page", async () => {
    let url: URL | undefined;
    server.use(
      http.get("/api/staff/admin/pages", ({ request }) => {
        url = new URL(request.url);
        return HttpResponse.json(listPayload(ROWS, 30));
      }),
    );
    await root.pageAdmin.loadList();
    expect(url?.searchParams.get("sort")).toBe("name");
    expect(url?.searchParams.get("order")).toBe("ASC");
    expect(url?.searchParams.get("page")).toBe("1");
    expect(url?.searchParams.get("per_page")).toBe("25");
    expect(root.pageAdmin.rows).toHaveLength(2);
    expect(root.pageAdmin.pagination.totalCount).toBe(30);
    expect(root.pageAdmin.pagination.totalPages).toBe(2);
  });

  it("setSort toggles order and resets to page 1", async () => {
    server.use(http.get("/api/staff/admin/pages", () => HttpResponse.json(listPayload(ROWS))));
    await root.pageAdmin.loadList();
    root.pageAdmin.page = 3;
    await root.pageAdmin.setSort("name"); // same key → toggle to DESC, page → 1
    expect(root.pageAdmin.order).toBe("DESC");
    expect(root.pageAdmin.page).toBe(1);
  });

  it("create: sends {name,type,body,isactive,notes} and shows success", async () => {
    let body: Record<string, unknown> | undefined;
    server.use(
      http.get("/api/staff/admin/pages", () => HttpResponse.json(listPayload(ROWS))),
      http.post("/api/staff/admin/pages", async ({ request }) => {
        body = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json({ id: 9 });
      }),
    );
    const s = root.pageAdmin;
    s.openCreate();
    s.setName("Thanks Page");
    s.setType("thank-you");
    s.setBody("Thank you");
    const ok = await s.save();
    expect(ok).toBe(true);
    expect(body).toMatchObject({ name: "Thanks Page", type: "thank-you", body: "Thank you", isactive: true });
    expect(root.snackbar.message).toBe("Thanks Page added successfully");
  });

  it("create: duplicate name → inline field error (BS-033.10)", async () => {
    server.use(
      http.post("/api/staff/admin/pages", () =>
        HttpResponse.json({ error: { message: "bad", fields: { name: "A page with this name already exists." } } }, { status: 422 }),
      ),
    );
    const s = root.pageAdmin;
    s.openCreate();
    s.setName("Welcome Landing");
    const ok = await s.save();
    expect(ok).toBe(false);
    expect(s.formOpen).toBe(true);
    expect(s.fieldError("name")).toBe("A page with this name already exists.");
  });

  it("edit: loads the detail (body) into the form with a clean baseline", async () => {
    server.use(
      http.get("/api/staff/admin/pages/1", () =>
        HttpResponse.json({ ...ROWS[0], body: "Welcome to support", notes: "n" }),
      ),
    );
    const s = root.pageAdmin;
    await s.openEdit(1);
    expect(s.name).toBe("Welcome Landing");
    expect(s.body).toBe("Welcome to support");
    expect(s.formDirty).toBe(false);
  });

  it("in-use guard: selecting a bound page blocks delete AND disable client-side", async () => {
    server.use(http.get("/api/staff/admin/pages", () => HttpResponse.json(listPayload(ROWS))));
    const s = root.pageAdmin;
    await s.loadList();
    s.toggleSelect(2); // in_use: true
    expect(s.selectionHasInUse).toBe(true);
    expect(await s.massAction("delete")).toBe(false);
    expect(s.massError).toMatch(/in use cannot be deleted/i);
    expect(await s.massAction("disable")).toBe(false);
    expect(s.massError).toMatch(/in use cannot be disabled/i);
  });

  it("mass delete of a free page calls the endpoint", async () => {
    let called: Record<string, unknown> | undefined;
    server.use(
      http.get("/api/staff/admin/pages", () => HttpResponse.json(listPayload(ROWS))),
      http.post("/api/staff/admin/pages/mass", async ({ request }) => {
        called = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json({ affected: 1, message: "1 page(s) deleted" });
      }),
    );
    const s = root.pageAdmin;
    await s.loadList();
    s.toggleSelect(1); // free
    const ok = await s.massAction("delete");
    expect(ok).toBe(true);
    expect(called).toMatchObject({ action: "delete", ids: [1] });
  });
});
