import { beforeEach, describe, expect, it } from "vitest";
import { server, http, HttpResponse } from "../test/mockServer";
import { RootStore } from "./RootStore";

/**
 * TS-M4-C2 business logic (TDD): list + default-row protection, form-options,
 * camelCase write body, group-access matrix Set, required-field pre-check,
 * default-private inline block, 422 → inline errors, delete-block, mass actions.
 */

const LIST = [
  {
    id: 1,
    name: "Support",
    ispublic: true,
    is_default: true,
    manager_id: null,
    manager_name: null,
    email_id: 5,
    tpl_id: 1,
    sla_id: 2,
    sla_name: "Default SLA",
    user_count: 3,
    updated: null,
  },
  {
    id: 2,
    name: "Sales",
    ispublic: false,
    is_default: false,
    manager_id: 9,
    manager_name: "Jo Manager",
    email_id: 5,
    tpl_id: 1,
    sla_id: null,
    sla_name: null,
    user_count: 0,
    updated: null,
  },
];

const OPTIONS = {
  email_accounts: [{ id: 5, email: "support@osticket.local", name: "Support" }],
  template_groups: [{ id: 1, name: "osTicket Default" }],
  sla: [{ id: 2, name: "Default SLA" }],
  staff: [{ id: 9, name: "Jo Manager" }],
  groups: [
    { id: 3, name: "Agents" },
    { id: 4, name: "Managers" },
  ],
};

const DETAIL_DEFAULT = {
  ...LIST[0],
  group_ids: [3],
  ticket_auto_response: true,
  message_auto_response: true,
  autoresp_email_id: 5,
  dept_signature: "Regards",
  group_membership: 0,
};

function mockList() {
  return http.get("/api/staff/admin/departments", () => HttpResponse.json(LIST));
}
function mockOptions() {
  return http.get("/api/staff/admin/departments/form-options", () => HttpResponse.json(OPTIONS));
}

describe("DeptAdminStore (TS-M4-C2)", () => {
  let root: RootStore;
  beforeEach(() => {
    root = new RootStore();
  });

  it("loads the list with public flag / manager / user count", async () => {
    server.use(mockList());
    await root.deptAdmin.loadList();
    expect(root.deptAdmin.rows[0]).toMatchObject({ name: "Support", is_default: true, user_count: 3 });
    expect(root.deptAdmin.rows[1]).toMatchObject({ name: "Sales", manager_name: "Jo Manager" });
  });

  it("protects the default row from selection (BS-030-05)", async () => {
    server.use(mockList());
    await root.deptAdmin.loadList();
    const d = root.deptAdmin;
    d.toggleSelect(1); // default → ignored
    expect(d.selectedIds).toEqual([]);
    d.toggleSelect(2);
    expect(d.selectedIds).toEqual([2]);
    d.toggleSelectAll(); // only selectable rows → all selectable already selected → clears
    expect(d.selectedIds).toEqual([]);
    d.toggleSelectAll();
    expect(d.selectedIds).toEqual([2]);
  });

  it("loads form-options (email accounts / templates / sla / staff / groups)", async () => {
    server.use(mockOptions());
    await root.deptAdmin.loadOptions();
    expect(root.deptAdmin.options.email_accounts[0].email).toBe("support@osticket.local");
    expect(root.deptAdmin.options.template_groups[0].name).toBe("osTicket Default");
    expect(root.deptAdmin.options.groups).toHaveLength(2);
  });

  it("group-access matrix: toggle / select-all / select-none", async () => {
    server.use(mockOptions());
    root.deptAdmin.openCreate();
    await root.deptAdmin.loadOptions();
    const d = root.deptAdmin;
    d.toggleGroup(3);
    expect(d.isGroupChecked(3)).toBe(true);
    d.selectAllGroups();
    expect(d.groupIds.size).toBe(2);
    d.selectNoGroups();
    expect(d.isGroupChecked(3)).toBe(false);
  });

  it("create: sends the camelCase body {emailId,tplId,slaId,managerId,groupIds,…}", async () => {
    let body: Record<string, unknown> | undefined;
    server.use(
      mockList(),
      mockOptions(),
      http.post("/api/staff/admin/departments", async ({ request }) => {
        body = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json({ id: 7 });
      }),
    );
    const d = root.deptAdmin;
    d.openCreate();
    await d.loadOptions();
    d.setName("Billing");
    d.setEmailId(5);
    d.setTplId(1);
    d.setSlaId(2);
    d.setManagerId(9);
    d.toggleGroup(3);
    const ok = await d.save();
    expect(ok).toBe(true);
    expect(body).toMatchObject({
      name: "Billing",
      ispublic: true,
      emailId: 5,
      tplId: 1,
      slaId: 2,
      managerId: 9,
      groupIds: [3],
    });
    // Regression: group_membership must be a NUMBER, never a boolean — the backend
    // DeptWriteRequest.group_membership is Option<i32> and serde rejects bool→i32.
    expect(typeof body!.groupMembership).toBe("number");
    expect(body!.groupMembership).toBe(0);
    expect(root.snackbar.message).toBe("Billing added successfully");
  });

  it("required Email / Template missing → inline pre-check errors (no request)", async () => {
    server.use(mockOptions());
    const d = root.deptAdmin;
    d.openCreate();
    await d.loadOptions();
    d.setName("Billing");
    let ok = await d.save();
    expect(ok).toBe(false);
    expect(d.fieldError("emailId")).toBe("Email selection required");
    d.setEmailId(5);
    ok = await d.save();
    expect(ok).toBe(false);
    expect(d.fieldError("tplId")).toBe("Template selection required");
  });

  it("editing the default department private → inline block (BS-030-04)", async () => {
    server.use(
      mockOptions(),
      http.get("/api/staff/admin/departments/1", () => HttpResponse.json(DETAIL_DEFAULT)),
    );
    const d = root.deptAdmin;
    await d.openEdit(1);
    expect(d.editingDefault).toBe(true);
    d.setIspublic(false);
    const ok = await d.save();
    expect(ok).toBe(false);
    expect(d.fieldError("ispublic")).toBe("System default department cannot be private");
  });

  it("edit loads group_ids into the matrix", async () => {
    server.use(
      mockOptions(),
      http.get("/api/staff/admin/departments/1", () => HttpResponse.json(DETAIL_DEFAULT)),
    );
    const d = root.deptAdmin;
    await d.openEdit(1);
    expect(d.isGroupChecked(3)).toBe(true);
    expect(d.formDirty).toBe(false);
  });

  it("create: a 422 maps to inline field errors (dialog stays open)", async () => {
    server.use(
      mockOptions(),
      http.post("/api/staff/admin/departments", () =>
        HttpResponse.json(
          { error: { message: "bad", fields: { name: "A department with this name already exists." } } },
          { status: 422 },
        ),
      ),
    );
    const d = root.deptAdmin;
    d.openCreate();
    d.setName("Support");
    d.setEmailId(5);
    d.setTplId(1);
    const ok = await d.save();
    expect(ok).toBe(false);
    expect(d.formOpen).toBe(true);
    expect(d.fieldError("name")).toContain("already exists");
  });

  it("delete with home staff → blocked message surfaced", async () => {
    server.use(
      mockList(),
      http.delete("/api/staff/admin/departments/2", () =>
        HttpResponse.json({ error: { message: "Department has agents assigned as their home department." } }, { status: 409 }),
      ),
    );
    const d = root.deptAdmin;
    const ok = await d.deleteOne(2);
    expect(ok).toBe(false);
    expect(root.snackbar.message).toContain("home department");
  });

  it("mass makeprivate: succeeds and clears the selection", async () => {
    server.use(
      mockList(),
      http.post("/api/staff/admin/departments/mass", () => HttpResponse.json({ affected: 1, message: "1 updated" })),
    );
    const d = root.deptAdmin;
    await d.loadList();
    d.toggleSelect(2);
    const ok = await d.massAction("makeprivate");
    expect(ok).toBe(true);
    expect(d.selectedIds).toEqual([]);
  });

  it("mass delete: affected=0 surfaces the block message", async () => {
    server.use(
      mockList(),
      http.post("/api/staff/admin/departments/mass", () =>
        HttpResponse.json({ affected: 0, message: "Departments with staff cannot be deleted" }),
      ),
    );
    const d = root.deptAdmin;
    await d.loadList();
    d.toggleSelect(2);
    const ok = await d.massAction("delete");
    expect(ok).toBe(false);
    expect(d.massError).toBe("Departments with staff cannot be deleted");
  });
});
