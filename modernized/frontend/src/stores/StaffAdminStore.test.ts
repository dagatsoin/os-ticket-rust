import { beforeEach, describe, expect, it } from "vitest";
import { server, http, HttpResponse } from "../test/mockServer";
import { RootStore } from "./RootStore";

/**
 * TS-M4-B2 business logic (TDD): list param serialization + pagination mapping,
 * mass-selection Set, create/edit payload (checkbox + password serialization,
 * partial edit sends only changed fields), 422 → inline errors, option loading.
 */

const LIST = {
  staff: [
    { id: 1, name: "Ada Lovelace", username: "agent", isactive: true, onvacation: false, group_name: "Support", dept_name: "Support", created: null, lastlogin: null },
    { id: 2, name: "Bob Stone", username: "agent2", isactive: true, onvacation: false, group_name: "Support", dept_name: "Support", created: null, lastlogin: null },
  ],
  pagination: { page: 1, per_page: 25, total: 2 },
};

const SETTINGS = {
  tabs: {},
  options: {
    departments: [{ id: 2, name: "Support" }],
    timezones: [{ id: 1, label: "UTC" }],
  },
};

const GROUPS = [{ id: 3, name: "Support", enabled: true, member_count: 2, dept_count: 1, created: null, updated: null }];

function mockList(capture?: (url: URL) => void) {
  return http.get("/api/staff/admin/staff", ({ request }) => {
    if (capture) capture(new URL(request.url));
    return HttpResponse.json(LIST);
  });
}

/** Options handlers (groups + settings) so eager loadOptions() calls don't 500. */
function mockOptions() {
  return [
    http.get("/api/staff/admin/groups", () => HttpResponse.json(GROUPS)),
    http.get("/api/staff/admin/settings", () => HttpResponse.json(SETTINGS)),
  ];
}

describe("StaffAdminStore (TS-M4-B2)", () => {
  let root: RootStore;
  beforeEach(() => {
    root = new RootStore();
  });

  it("loads the list and maps pagination {page,per_page,total} → PaginationMeta", async () => {
    server.use(mockList());
    await root.staffAdmin.loadList();
    expect(root.staffAdmin.staff).toHaveLength(2);
    expect(root.staffAdmin.pagination).toEqual({ page: 1, pageSize: 25, totalCount: 2, totalPages: 1 });
  });

  it("serializes filter/sort/page params into the querystring", async () => {
    let seen: URL | undefined;
    server.use(mockList((u) => (seen = u)));
    await root.staffAdmin.loadList({ q: "bulk_1", did: 2, sort: "username", order: "DESC", page: 3 });
    expect(seen?.searchParams.get("q")).toBe("bulk_1");
    expect(seen?.searchParams.get("did")).toBe("2");
    expect(seen?.searchParams.get("sort")).toBe("username");
    expect(seen?.searchParams.get("order")).toBe("DESC");
    expect(seen?.searchParams.get("page")).toBe("3");
  });

  it("toggles sort order on the same column and resets to ASC on a new column", async () => {
    server.use(mockList());
    await root.staffAdmin.loadList();
    await root.staffAdmin.setSort("username");
    expect(root.staffAdmin.params).toMatchObject({ sort: "username", order: "ASC" });
    await root.staffAdmin.setSort("username");
    expect(root.staffAdmin.params.order).toBe("DESC");
    await root.staffAdmin.setSort("name");
    expect(root.staffAdmin.params).toMatchObject({ sort: "name", order: "ASC" });
  });

  it("manages the mass-selection Set (toggle / select-all / clear)", async () => {
    server.use(mockList());
    await root.staffAdmin.loadList();
    root.staffAdmin.toggleSelect(1);
    expect(root.staffAdmin.selectedIds).toEqual([1]);
    expect(root.staffAdmin.someSelected).toBe(true);
    root.staffAdmin.toggleSelectAll();
    expect(root.staffAdmin.allSelected).toBe(true);
    root.staffAdmin.clearSelection();
    expect(root.staffAdmin.selectedIds).toEqual([]);
  });

  it("loads options: groups from /groups, departments + timezones from settings", async () => {
    server.use(
      http.get("/api/staff/admin/groups", () => HttpResponse.json(GROUPS)),
      http.get("/api/staff/admin/settings", () => HttpResponse.json(SETTINGS)),
    );
    await root.staffAdmin.loadOptions();
    expect(root.staffAdmin.groupOptions).toEqual([{ id: 3, name: "Support" }]);
    expect(root.staffAdmin.deptOptions).toEqual([{ id: 2, name: "Support" }]);
    expect(root.staffAdmin.timezoneOptions).toEqual([{ id: 1, label: "UTC" }]);
  });

  it("create: sends booleans + password fields and shows a success banner", async () => {
    let body: Record<string, unknown> | undefined;
    server.use(
      mockList(),
      ...mockOptions(),
      http.post("/api/staff/admin/staff", async ({ request }) => {
        body = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json({ id: 99 });
      }),
    );
    const s = root.staffAdmin;
    s.openCreate();
    s.setField("firstname", "Nadia");
    s.setField("lastname", "Ncreate");
    s.setField("username", "ncreate");
    s.setField("email", "ncreate@osticket.local");
    s.setField("groupId", "3");
    s.setField("deptId", "2");
    s.setField("password", "Temp123456!");
    s.setField("passwd2", "Temp123456!");
    const ok = await s.save();
    expect(ok).toBe(true);
    expect(body).toMatchObject({
      username: "ncreate",
      groupId: 3,
      deptId: 2,
      password: "Temp123456!",
      passwd2: "Temp123456!",
      isactive: true,
      isvisible: true,
      isadmin: false,
    });
    expect(root.snackbar.message).toBe("Nadia Ncreate added successfully");
  });

  it("create: a 422 maps error.fields onto inline field errors (dialog stays open)", async () => {
    server.use(
      ...mockOptions(),
      http.post("/api/staff/admin/staff", () =>
        HttpResponse.json({ error: { message: "bad", fields: { username: "Username already in use" } } }, { status: 422 }),
      ),
    );
    const s = root.staffAdmin;
    s.openCreate();
    s.setField("username", "agent");
    const ok = await s.save();
    expect(ok).toBe(false);
    expect(s.formOpen).toBe(true);
    expect(s.fieldError("username")).toBe("Username already in use");
  });

  it("edit: PUT carries only fields changed from the row baseline (partial update)", async () => {
    let body: Record<string, unknown> | undefined;
    server.use(
      mockList(),
      http.put("/api/staff/admin/staff/2", async ({ request }) => {
        body = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json({ id: 2 });
      }),
    );
    const s = root.staffAdmin;
    s.groupOptions = [{ id: 3, name: "Support" }];
    s.deptOptions = [
      { id: 2, name: "Support" },
      { id: 61, name: "Sales" },
    ];
    s.optionsLoaded = true;
    s.openEdit(LIST.staff[1]); // agent2, dept Support (id 2)
    s.setField("deptId", "61"); // change dept only
    const ok = await s.save();
    expect(ok).toBe(true);
    expect(body).toEqual({ deptId: 61 }); // ONLY the changed field
  });

  it("edit: clearing the admin flag surfaces the last-admin 422 inline on isadmin", async () => {
    server.use(
      mockList(),
      ...mockOptions(),
      http.put("/api/staff/admin/staff/1", () =>
        HttpResponse.json(
          { error: { message: "no", fields: { isadmin: "Cowardly refusing to remove or lock out the only active administrator" } } },
          { status: 422 },
        ),
      ),
    );
    const s = root.staffAdmin;
    s.openEdit(LIST.staff[0], { ownIsAdmin: true });
    s.setField("isadmin", false);
    const ok = await s.save();
    expect(ok).toBe(false);
    expect(s.fieldError("isadmin")).toContain("Cowardly refusing");
  });

  it("mass: blocks on empty selection without a request", async () => {
    const s = root.staffAdmin;
    const ok = await s.massAction("lock");
    expect(ok).toBe(false);
    expect(s.massError).toMatch(/at least one/);
  });

  it("mass: surfaces the self-action 422 message", async () => {
    server.use(
      mockList(),
      http.post("/api/staff/admin/staff/mass", () =>
        HttpResponse.json({ error: { message: "You can not disable/delete yourself - you could be the only admin!" } }, { status: 422 }),
      ),
    );
    const s = root.staffAdmin;
    await s.loadList();
    s.toggleSelect(1);
    const ok = await s.massAction("lock");
    expect(ok).toBe(false);
    expect(s.massError).toContain("You can not disable/delete yourself");
  });
});

/**
 * BS-030-14 (add a member from a staff profile). The Teams section in the staff
 * edit dialog derives the member's current teams from the team roster (there is
 * no GET single-staff endpoint), then adds/removes memberships and refetches.
 */
describe("StaffAdminStore team membership (BS-030-14)", () => {
  let root: RootStore;
  beforeEach(() => {
    root = new RootStore();
  });

  const TEAMS = [
    { id: 10, name: "Tier 1", isenabled: true, lead_id: null, lead_name: "", member_count: 1, updated: null },
    { id: 11, name: "Tier 2", isenabled: true, lead_id: null, lead_name: "", member_count: 0, updated: null },
  ];

  /**
   * Team GET handlers backed by a mutable membership map (teamId → Set<staffId>)
   * so add/remove + refetch reflect the mutation, like the live backend.
   */
  function teamHandlers(membership: Record<number, Set<number>>) {
    return [
      http.get("/api/staff/admin/teams", () => HttpResponse.json(TEAMS)),
      http.get("/api/staff/admin/teams/:id", ({ params }) => {
        const id = Number(params.id);
        const team = TEAMS.find((t) => t.id === id);
        const members = Array.from(membership[id] ?? new Set<number>()).map((sid) => ({
          staff_id: sid,
          name: `Staff ${sid}`,
        }));
        return HttpResponse.json({
          id,
          name: team?.name ?? "",
          isenabled: true,
          lead_id: null,
          noalerts: false,
          notes: "",
          members,
        });
      }),
    ];
  }

  it("loadStaffTeams derives the member's teams and the available (not-yet-member) teams", async () => {
    server.use(...teamHandlers({ 10: new Set([2]), 11: new Set() }));
    await root.staffAdmin.loadStaffTeams(2);
    expect(root.staffAdmin.staffTeams).toEqual([{ id: 10, name: "Tier 1" }]);
    expect(root.staffAdmin.availableTeams).toEqual([{ id: 11, name: "Tier 2" }]);
  });

  it("addToTeam posts {teamId}, refetches, and surfaces a success snackbar", async () => {
    const membership: Record<number, Set<number>> = { 10: new Set([2]), 11: new Set() };
    let body: unknown;
    server.use(
      ...teamHandlers(membership),
      http.post("/api/staff/admin/staff/2/teams", async ({ request }) => {
        body = await request.json();
        membership[11].add(2);
        return HttpResponse.json({ ok: true });
      }),
    );
    await root.staffAdmin.loadStaffTeams(2);
    const ok = await root.staffAdmin.addToTeam(2, 11);
    expect(ok).toBe(true);
    expect(body).toEqual({ teamId: 11 });
    expect(root.staffAdmin.staffTeams.map((t) => t.id).sort((a, b) => a - b)).toEqual([10, 11]);
    expect(root.staffAdmin.availableTeams).toEqual([]);
    expect(root.snackbar.severity).toBe("success");
    expect(root.snackbar.message).toMatch(/Tier 2/);
  });

  it("removeFromTeam deletes and drops the team from the member's list", async () => {
    const membership: Record<number, Set<number>> = { 10: new Set([2]), 11: new Set([2]) };
    server.use(
      ...teamHandlers(membership),
      http.delete("/api/staff/admin/staff/2/teams/11", () => {
        membership[11].delete(2);
        return HttpResponse.json({ ok: true, removed: 1 });
      }),
    );
    await root.staffAdmin.loadStaffTeams(2);
    expect(root.staffAdmin.staffTeams).toHaveLength(2);
    const ok = await root.staffAdmin.removeFromTeam(2, 11);
    expect(ok).toBe(true);
    expect(root.staffAdmin.staffTeams.map((t) => t.id)).toEqual([10]);
    expect(root.snackbar.severity).toBe("success");
  });

  it("addToTeam surfaces a server error via the snackbar and returns false", async () => {
    server.use(
      ...teamHandlers({ 10: new Set([2]), 11: new Set() }),
      http.post("/api/staff/admin/staff/2/teams", () =>
        HttpResponse.json({ error: { message: "Could not add team membership" } }, { status: 500 }),
      ),
    );
    await root.staffAdmin.loadStaffTeams(2);
    const ok = await root.staffAdmin.addToTeam(2, 11);
    expect(ok).toBe(false);
    expect(root.snackbar.severity).toBe("error");
    expect(root.snackbar.message).toContain("Could not add team membership");
  });

  it("addToTeam surfaces a 422 validation message via the snackbar", async () => {
    server.use(
      ...teamHandlers({ 10: new Set([2]), 11: new Set() }),
      http.post("/api/staff/admin/staff/2/teams", () =>
        HttpResponse.json({ error: { message: "Team is disabled" } }, { status: 422 }),
      ),
    );
    await root.staffAdmin.loadStaffTeams(2);
    const ok = await root.staffAdmin.addToTeam(2, 11);
    expect(ok).toBe(false);
    expect(root.staffAdmin.teamsError).toContain("Team is disabled");
    expect(root.snackbar.message).toContain("Team is disabled");
  });

  it("loadStaffTeams surfaces a load failure without throwing", async () => {
    server.use(
      http.get("/api/staff/admin/teams", () =>
        HttpResponse.json({ error: { message: "Team list failed" } }, { status: 500 }),
      ),
    );
    await root.staffAdmin.loadStaffTeams(2);
    expect(root.staffAdmin.staffTeams).toEqual([]);
    expect(root.staffAdmin.teamsError).toContain("Team list failed");
  });
});
