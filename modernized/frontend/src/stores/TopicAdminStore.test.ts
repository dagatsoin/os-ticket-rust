import { beforeEach, describe, expect, it } from "vitest";
import { server, http, HttpResponse } from "../test/mockServer";
import { RootStore } from "./RootStore";
import { decodeAssignTo, encodeStaff, encodeTeam } from "./TopicAdminStore";

/**
 * TS-M4-C6 business logic (TDD): paginated list meta, form-options, assignTo
 * encode/decode + staff↔team mutual exclusion, parent candidates (top-level,
 * not self), required-field pre-check, write body, 422 → inline, mass actions.
 */

const PAGE1 = {
  items: [
    {
      topic_id: 1,
      topic: "General",
      topic_pid: null,
      parent_topic: null,
      isactive: true,
      ispublic: true,
      priority_id: 2,
      priority: "Normal",
      dept_id: 1,
      dept_name: "Support",
      sla_id: null,
      staff_id: null,
      team_id: null,
      page_id: null,
      updated: null,
    },
    {
      topic_id: 2,
      topic: "Refund",
      topic_pid: 1,
      parent_topic: "General",
      isactive: true,
      ispublic: false,
      priority_id: 2,
      priority: "Normal",
      dept_id: 1,
      dept_name: "Support",
      sla_id: null,
      staff_id: 9,
      team_id: null,
      page_id: null,
      updated: null,
    },
  ],
  total: 30,
  page: 1,
  page_size: 25,
};

const OPTIONS = {
  priorities: [{ id: 2, name: "Normal" }],
  departments: [{ id: 1, name: "Support" }],
  sla: [{ id: 3, name: "Default SLA" }],
  pages: [{ id: 4, name: "Thank You" }],
  staff: [{ id: 9, name: "Jo" }],
  teams: [{ id: 7, name: "Tier 2" }],
  parent_topics: [{ id: 1, name: "General" }],
};

const DETAIL = {
  ...PAGE1.items[1],
  noautoresp: false,
  notes: "",
};

function mockList() {
  return http.get("/api/staff/admin/help-topics", () => HttpResponse.json(PAGE1));
}
function mockOptions() {
  return http.get("/api/staff/admin/help-topics/form-options", () => HttpResponse.json(OPTIONS));
}

describe("TopicAdminStore (TS-M4-C6)", () => {
  let root: RootStore;
  beforeEach(() => {
    root = new RootStore();
  });

  it("assignTo encode/decode helpers round-trip", () => {
    expect(encodeStaff(9)).toBe("s9");
    expect(encodeTeam(7)).toBe("t7");
    expect(decodeAssignTo("s9")).toEqual({ kind: "staff", id: 9 });
    expect(decodeAssignTo("t7")).toEqual({ kind: "team", id: 7 });
    expect(decodeAssignTo("")).toBeNull();
    expect(decodeAssignTo("x1")).toBeNull();
  });

  it("loads a paginated list + meta", async () => {
    server.use(mockList());
    await root.topicAdmin.loadList();
    expect(root.topicAdmin.rows).toHaveLength(2);
    expect(root.topicAdmin.pagination).toMatchObject({ page: 1, pageSize: 25, totalCount: 30, totalPages: 2 });
  });

  it("auto-assign is mutually exclusive (staff clears team, team clears staff)", async () => {
    server.use(mockOptions());
    root.topicAdmin.openCreate();
    await root.topicAdmin.loadOptions();
    const t = root.topicAdmin;
    t.setAssignStaff(9);
    expect(t.assignStaffId).toBe(9);
    expect(t.assignTeamId).toBeNull();
    t.setAssignTeam(7);
    expect(t.assignTeamId).toBe(7);
    expect(t.assignStaffId).toBeNull();
    t.clearAssign();
    expect(t.assignTo).toBe("");
  });

  it("edit decodes staff_id into the assignTo token", async () => {
    server.use(
      mockOptions(),
      http.get("/api/staff/admin/help-topics/2", () => HttpResponse.json(DETAIL)),
    );
    const t = root.topicAdmin;
    await t.openEdit(2);
    expect(t.assignTo).toBe("s9");
    expect(t.parentId).toBe(1);
    expect(t.formDirty).toBe(false);
  });

  it("parent candidates are top-level topics only, excluding self", async () => {
    server.use(
      mockOptions(),
      http.get("/api/staff/admin/help-topics/1", () => HttpResponse.json({ ...PAGE1.items[0], noautoresp: false, notes: "" })),
    );
    const t = root.topicAdmin;
    await t.openEdit(1);
    // editing topic 1 → it must not offer itself as a parent
    expect(t.parentCandidates.map((p) => p.id)).toEqual([]);
  });

  it("required dept / priority missing → inline pre-check errors (no request)", async () => {
    server.use(mockOptions());
    const t = root.topicAdmin;
    t.openCreate();
    await t.loadOptions();
    t.setTopic("Broken thing");
    let ok = await t.save();
    expect(ok).toBe(false);
    expect(t.fieldError("deptId")).toBe("Department selection required");
    t.setDeptId(1);
    ok = await t.save();
    expect(ok).toBe(false);
    expect(t.fieldError("priorityId")).toBe("Priority selection required");
  });

  it("create: sends {topic,deptId,priorityId,slaId,assignTo,pageId,…}", async () => {
    let body: Record<string, unknown> | undefined;
    server.use(
      mockList(),
      mockOptions(),
      http.post("/api/staff/admin/help-topics", async ({ request }) => {
        body = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json({ id: 9 });
      }),
    );
    const t = root.topicAdmin;
    t.openCreate();
    await t.loadOptions();
    t.setTopic("Billing question");
    t.setDeptId(1);
    t.setPriorityId(2);
    t.setSlaId(3);
    t.setPageId(4);
    t.setAssignTeam(7);
    const ok = await t.save();
    expect(ok).toBe(true);
    expect(body).toMatchObject({
      topic: "Billing question",
      deptId: 1,
      priorityId: 2,
      slaId: 3,
      pageId: 4,
      assignTo: "t7",
    });
    expect(root.snackbar.message).toBe("Billing question added successfully");
  });

  it("create: a 422 maps to inline field errors", async () => {
    server.use(
      mockOptions(),
      http.post("/api/staff/admin/help-topics", () =>
        HttpResponse.json({ error: { message: "bad", fields: { topic: "This topic already exists." } } }, { status: 422 }),
      ),
    );
    const t = root.topicAdmin;
    t.openCreate();
    t.setTopic("General");
    t.setDeptId(1);
    t.setPriorityId(2);
    const ok = await t.save();
    expect(ok).toBe(false);
    expect(t.fieldError("topic")).toContain("already exists");
  });

  it("mass disable: succeeds and clears the selection", async () => {
    server.use(
      mockList(),
      http.post("/api/staff/admin/help-topics/mass", () => HttpResponse.json({ affected: 1, message: "1 disabled" })),
    );
    const t = root.topicAdmin;
    await t.loadList();
    t.toggleSelect(2);
    const ok = await t.massAction("disable");
    expect(ok).toBe(true);
    expect(t.selectedIds).toEqual([]);
  });
});
