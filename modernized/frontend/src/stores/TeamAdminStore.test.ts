import { beforeEach, describe, expect, it } from "vitest";
import { server, http, HttpResponse } from "../test/mockServer";
import { RootStore } from "./RootStore";

/**
 * TS-M4-C4 business logic (TDD): list, member-detail load, remove-toggle, lead
 * reset when the lead is marked for removal, lead options exclude removed
 * members, write body ({name,isenabled,leadId,noalerts,notes,removeMemberIds}),
 * 422 → inline errors, mass actions.
 */

const LIST = [
  { id: 1, name: "Tier 2", isenabled: true, lead_id: 10, lead_name: "Ada", member_count: 2, updated: null },
  { id: 2, name: "Escalation", isenabled: false, lead_id: null, lead_name: null, member_count: 0, updated: null },
];

const DETAIL = {
  ...LIST[0],
  noalerts: false,
  notes: "",
  members: [
    { staff_id: 10, name: "Ada" },
    { staff_id: 11, name: "Ben" },
  ],
};

function mockList() {
  return http.get("/api/staff/admin/teams", () => HttpResponse.json(LIST));
}
function mockDetail() {
  return http.get("/api/staff/admin/teams/1", () => HttpResponse.json(DETAIL));
}

describe("TeamAdminStore (TS-M4-C4)", () => {
  let root: RootStore;
  beforeEach(() => {
    root = new RootStore();
  });

  it("loads the list with status / member count / lead", async () => {
    server.use(mockList());
    await root.teamAdmin.loadList();
    expect(root.teamAdmin.rows[0]).toMatchObject({ name: "Tier 2", member_count: 2, lead_name: "Ada" });
  });

  it("edit loads the member roster + lead (baseline not dirty)", async () => {
    server.use(mockDetail());
    const t = root.teamAdmin;
    await t.openEdit(1);
    expect(t.members).toHaveLength(2);
    expect(t.leadId).toBe(10);
    expect(t.formDirty).toBe(false);
  });

  it("marking the lead for removal resets the lead (BS-030-14)", async () => {
    server.use(mockDetail());
    const t = root.teamAdmin;
    await t.openEdit(1);
    expect(t.leadId).toBe(10);
    t.toggleRemoveMember(10); // remove the lead
    expect(t.leadId).toBeNull();
    expect(t.isMarkedForRemoval(10)).toBe(true);
  });

  it("lead candidates exclude members marked for removal", async () => {
    server.use(mockDetail());
    const t = root.teamAdmin;
    await t.openEdit(1);
    expect(t.leadCandidates.map((m) => m.staff_id)).toEqual([10, 11]);
    t.toggleRemoveMember(11);
    expect(t.leadCandidates.map((m) => m.staff_id)).toEqual([10]);
  });

  it("un-marking a member restores it as a lead candidate", async () => {
    server.use(mockDetail());
    const t = root.teamAdmin;
    await t.openEdit(1);
    t.toggleRemoveMember(11);
    t.toggleRemoveMember(11);
    expect(t.leadCandidates.map((m) => m.staff_id)).toEqual([10, 11]);
  });

  it("create: sends {name,isenabled,leadId,noalerts,notes,removeMemberIds}", async () => {
    let body: Record<string, unknown> | undefined;
    server.use(
      mockList(),
      http.post("/api/staff/admin/teams", async ({ request }) => {
        body = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json({ id: 5 });
      }),
    );
    const t = root.teamAdmin;
    t.openCreate();
    t.setName("Weekend");
    t.setIsenabled(true);
    t.setNoalerts(true);
    const ok = await t.save();
    expect(ok).toBe(true);
    expect(body).toMatchObject({ name: "Weekend", isenabled: true, noalerts: true, removeMemberIds: [] });
    expect(root.snackbar.message).toBe("Weekend added successfully");
  });

  it("edit: submits removeMemberIds for members marked for removal", async () => {
    let body: Record<string, unknown> | undefined;
    server.use(
      mockList(),
      mockDetail(),
      http.put("/api/staff/admin/teams/1", async ({ request }) => {
        body = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json({ id: 1 });
      }),
    );
    const t = root.teamAdmin;
    await t.openEdit(1);
    t.toggleRemoveMember(11);
    const ok = await t.save();
    expect(ok).toBe(true);
    expect(body?.removeMemberIds).toEqual([11]);
  });

  it("create: a 422 maps to an inline field error (dialog stays open)", async () => {
    server.use(
      http.post("/api/staff/admin/teams", () =>
        HttpResponse.json({ error: { message: "bad", fields: { name: "Team name must be at least 3 chars." } } }, { status: 422 }),
      ),
    );
    const t = root.teamAdmin;
    t.openCreate();
    t.setName("Ab");
    const ok = await t.save();
    expect(ok).toBe(false);
    expect(t.formOpen).toBe(true);
    expect(t.fieldError("name")).toContain("at least 3 chars");
  });

  it("mass enable: succeeds and clears the selection", async () => {
    server.use(
      mockList(),
      http.post("/api/staff/admin/teams/mass", () => HttpResponse.json({ affected: 1, message: "1 enabled" })),
    );
    const t = root.teamAdmin;
    await t.loadList();
    t.toggleSelect(2);
    const ok = await t.massAction("enable");
    expect(ok).toBe(true);
    expect(t.selectedIds).toEqual([]);
  });
});
