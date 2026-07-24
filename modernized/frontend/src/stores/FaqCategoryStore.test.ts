import { beforeEach, describe, expect, it } from "vitest";
import { server, http, HttpResponse } from "../test/mockServer";
import { RootStore } from "./RootStore";

/**
 * TS-M4-E2 business logic (TDD): list (base path /api/staff/faq-categories, NOT
 * /admin), create/edit body ({name,ispublic,description,notes}), dirty tracking,
 * 422 → inline errors, delete, mass actions.
 */

const LIST = [
  { id: 1, name: "Getting Started", ispublic: true, description: "Basics", notes: "", updated: null },
  { id: 2, name: "Internal", ispublic: false, description: "Staff only", notes: "n", updated: null },
];

const DETAIL = LIST[1];

function mockList() {
  return http.get("/api/staff/faq-categories", () => HttpResponse.json(LIST));
}

describe("FaqCategoryStore (TS-M4-E2)", () => {
  let root: RootStore;
  beforeEach(() => {
    root = new RootStore();
  });

  it("loads the list from the non-admin base path", async () => {
    server.use(mockList());
    await root.faqCategories.loadList();
    expect(root.faqCategories.rows[0]).toMatchObject({ name: "Getting Started", ispublic: true });
    expect(root.faqCategories.rows[1]).toMatchObject({ name: "Internal", ispublic: false });
  });

  it("create: sends {name,ispublic,description,notes}", async () => {
    let body: Record<string, unknown> | undefined;
    server.use(
      mockList(),
      http.post("/api/staff/faq-categories", async ({ request }) => {
        body = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json({ id: 9 });
      }),
    );
    const f = root.faqCategories;
    f.openCreate();
    f.setName("Billing");
    f.setIspublic(true);
    f.setDescription("Payment questions");
    const ok = await f.save();
    expect(ok).toBe(true);
    expect(body).toMatchObject({ name: "Billing", ispublic: true, description: "Payment questions" });
    expect(root.snackbar.message).toBe("Billing added successfully");
  });

  it("edit loads the detail; toggling type marks the form dirty", async () => {
    server.use(
      mockList(),
      http.get("/api/staff/faq-categories/2", () => HttpResponse.json(DETAIL)),
      http.put("/api/staff/faq-categories/2", () => HttpResponse.json({ id: 2 })),
    );
    const f = root.faqCategories;
    await f.openEdit(2);
    expect(f.name).toBe("Internal");
    expect(f.ispublic).toBe(false);
    expect(f.formDirty).toBe(false);
    f.setIspublic(true);
    expect(f.formDirty).toBe(true);
    const ok = await f.save();
    expect(ok).toBe(true);
    expect(root.snackbar.message).toBe("Internal updated successfully");
  });

  it("create: a 422 maps to an inline field error (dialog stays open)", async () => {
    server.use(
      http.post("/api/staff/faq-categories", () =>
        HttpResponse.json({ error: { message: "bad", fields: { name: "A category with this name already exists." } } }, { status: 422 }),
      ),
    );
    const f = root.faqCategories;
    f.openCreate();
    f.setName("Getting Started");
    const ok = await f.save();
    expect(ok).toBe(false);
    expect(f.formOpen).toBe(true);
    expect(f.fieldError("name")).toContain("already exists");
  });

  it("delete: removes a category", async () => {
    server.use(
      mockList(),
      http.delete("/api/staff/faq-categories/2", () => HttpResponse.json({ ok: true })),
    );
    const f = root.faqCategories;
    const ok = await f.deleteOne(2);
    expect(ok).toBe(true);
    expect(root.snackbar.message).toBe("FAQ category deleted");
  });

  it("mass makepublic: succeeds and clears the selection", async () => {
    server.use(
      mockList(),
      http.post("/api/staff/faq-categories/mass", () => HttpResponse.json({ affected: 1, message: "1 updated" })),
    );
    const f = root.faqCategories;
    await f.loadList();
    f.toggleSelect(2);
    const ok = await f.massAction("makepublic");
    expect(ok).toBe(true);
    expect(f.selectedIds).toEqual([]);
  });
});
