import { beforeEach, describe, expect, it } from "vitest";
import { server, http, HttpResponse } from "../test/mockServer";
import { RootStore } from "./RootStore";
import { GROUP_FLAG_KEYS } from "./GroupAdminStore";

/**
 * TS-M4-B4 business logic (TDD): eleven-flag booleans, dept-matrix Set +
 * select-all/none, dirty tracking, full-replace write payload ({flags,deptIds}),
 * 422 → inline errors, mass-action outcome handling (delete-with-members block).
 */

const LIST = [
  { id: 3, name: "Support", enabled: true, member_count: 2, dept_count: 1, created: null, updated: null },
  { id: 4, name: "Empty", enabled: true, member_count: 0, dept_count: 0, created: null, updated: null },
];

const DETAIL = {
  id: 3,
  name: "Support",
  enabled: true,
  notes: "",
  flags: { can_create_tickets: true, can_post_reply: true, can_manage_faq: false },
  dept_ids: [2],
};

const SETTINGS = { tabs: {}, options: { departments: [{ id: 2, name: "Support" }, { id: 61, name: "Sales" }] } };

function mockList() {
  return http.get("/api/staff/admin/groups", () => HttpResponse.json(LIST));
}
function mockSettings() {
  return http.get("/api/staff/admin/settings", () => HttpResponse.json(SETTINGS));
}

describe("GroupAdminStore (TS-M4-B4)", () => {
  let root: RootStore;
  beforeEach(() => {
    root = new RootStore();
  });

  it("exposes the canonical eleven flags including the four net-new ones", () => {
    expect(GROUP_FLAG_KEYS).toHaveLength(11);
    for (const k of ["can_manage_faq", "can_manage_premade", "can_ban_emails", "can_view_staff_stats"]) {
      expect(GROUP_FLAG_KEYS).toContain(k);
    }
  });

  it("loads the list with member/dept counts", async () => {
    server.use(mockList());
    await root.groupAdmin.loadList();
    expect(root.groupAdmin.groups[0]).toMatchObject({ name: "Support", member_count: 2, dept_count: 1 });
  });

  it("create form defaults all eleven flags to false", async () => {
    server.use(mockSettings());
    root.groupAdmin.openCreate();
    for (const k of GROUP_FLAG_KEYS) expect(root.groupAdmin.flags[k]).toBe(false);
  });

  it("dept matrix: toggle, select-all, select-none", async () => {
    server.use(mockSettings());
    root.groupAdmin.openCreate();
    await root.groupAdmin.loadOptions();
    root.groupAdmin.toggleDept(2);
    expect(root.groupAdmin.isDeptChecked(2)).toBe(true);
    root.groupAdmin.selectAllDepts();
    expect(root.groupAdmin.deptIds.size).toBe(2);
    root.groupAdmin.selectNoDepts();
    expect(root.groupAdmin.isDeptChecked(2)).toBe(false);
  });

  it("tracks dirty state from the baseline", async () => {
    server.use(mockSettings());
    root.groupAdmin.openCreate();
    expect(root.groupAdmin.formDirty).toBe(false);
    root.groupAdmin.setName("Faq Managers");
    expect(root.groupAdmin.formDirty).toBe(true);
  });

  it("create: sends {name,enabled,flags(11),deptIds,notes} full-replace body", async () => {
    let body: Record<string, unknown> | undefined;
    server.use(
      mockList(),
      mockSettings(),
      http.post("/api/staff/admin/groups", async ({ request }) => {
        body = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json({ id: 7 });
      }),
    );
    const g = root.groupAdmin;
    g.openCreate();
    await g.loadOptions();
    g.setName("Faq Managers");
    g.setFlag("can_manage_faq", true);
    g.toggleDept(2);
    const ok = await g.save();
    expect(ok).toBe(true);
    expect(body).toMatchObject({ name: "Faq Managers", enabled: true, deptIds: [2] });
    const flags = body!.flags as Record<string, boolean>;
    expect(Object.keys(flags)).toHaveLength(11);
    expect(flags.can_manage_faq).toBe(true);
    expect(flags.can_create_tickets).toBe(false);
    expect(root.snackbar.message).toBe("Faq Managers added successfully");
  });

  it("edit: loads the detail into flags + dept Set (full-replace baseline)", async () => {
    server.use(
      mockSettings(),
      http.get("/api/staff/admin/groups/3", () => HttpResponse.json(DETAIL)),
    );
    const g = root.groupAdmin;
    await g.openEdit(3);
    expect(g.name).toBe("Support");
    expect(g.flags.can_create_tickets).toBe(true);
    expect(g.flags.can_manage_faq).toBe(false);
    expect(g.isDeptChecked(2)).toBe(true);
    expect(g.formDirty).toBe(false);
  });

  it("create: a 422 on name maps to an inline field error (dialog stays open)", async () => {
    server.use(
      mockSettings(),
      http.post("/api/staff/admin/groups", () =>
        HttpResponse.json({ error: { message: "bad", fields: { name: "Group name must be at least 3 chars." } } }, { status: 422 }),
      ),
    );
    const g = root.groupAdmin;
    g.openCreate();
    g.setName("Ab");
    const ok = await g.save();
    expect(ok).toBe(false);
    expect(g.formOpen).toBe(true);
    expect(g.fieldError("name")).toBe("Group name must be at least 3 chars.");
  });

  it("mass delete: affected=0 surfaces the 'Unable to delete' block as an error", async () => {
    server.use(
      mockList(),
      http.post("/api/staff/admin/groups/mass", () =>
        HttpResponse.json({ affected: 0, message: "Unable to delete selected groups" }),
      ),
    );
    const g = root.groupAdmin;
    await g.loadList();
    g.toggleSelect(3);
    const ok = await g.massAction("delete");
    expect(ok).toBe(false);
    expect(g.massError).toBe("Unable to delete selected groups");
  });

  it("mass disable: surfaces the self-group 422 block", async () => {
    server.use(
      mockList(),
      http.post("/api/staff/admin/groups/mass", () =>
        HttpResponse.json(
          { error: { message: "As an admin, you can't disable/delete a group you belong to - you might lockout all admins!" } },
          { status: 422 },
        ),
      ),
    );
    const g = root.groupAdmin;
    await g.loadList();
    g.toggleSelect(3);
    const ok = await g.massAction("disable");
    expect(ok).toBe(false);
    expect(g.massError).toContain("lockout all admins");
  });
});
