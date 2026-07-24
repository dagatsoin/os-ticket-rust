import { beforeEach, describe, expect, it } from "vitest";
import { server, http, HttpResponse } from "../test/mockServer";
import { RootStore } from "./RootStore";
import { DEFAULT_SLA_DELETE_MESSAGE } from "./SlaAdminStore";

/**
 * TS-M4-D2 business logic (TDD): list + client-side Date-Added sort toggle,
 * form dirty tracking, 422 → inline field errors, mass actions, and the
 * default-SLA delete-protection predicate + guard (FS-032.12).
 */

const LIST = [
  {
    id: 1,
    name: "Default SLA",
    grace_period: 24,
    isactive: true,
    enable_priority_escalation: false,
    transient: false,
    disable_overdue_alerts: false,
    notes: "",
    created: "2024-01-01T00:00:00Z",
    updated: null,
    is_default: true,
  },
  {
    id: 2,
    name: "Silver SLA",
    grace_period: 8,
    isactive: true,
    enable_priority_escalation: false,
    transient: false,
    disable_overdue_alerts: false,
    notes: "",
    created: "2024-03-01T00:00:00Z",
    updated: null,
    is_default: false,
  },
];

function mockList() {
  return http.get("/api/staff/admin/sla", () => HttpResponse.json(LIST));
}

describe("SlaAdminStore (TS-M4-D2)", () => {
  let root: RootStore;
  beforeEach(() => {
    root = new RootStore();
  });

  it("loads the SLA list", async () => {
    server.use(mockList());
    await root.slaAdmin.loadList();
    expect(root.slaAdmin.rows).toHaveLength(2);
    expect(root.slaAdmin.rows[0]).toMatchObject({ name: "Default SLA", is_default: true });
  });

  it("sorts by Date Added and toggles order on repeat clicks (KL-032.10 modernised)", async () => {
    server.use(mockList());
    await root.slaAdmin.loadList();
    root.slaAdmin.setSort("created"); // ASC
    expect(root.slaAdmin.order).toBe("ASC");
    expect(root.slaAdmin.sortedRows.map((r) => r.id)).toEqual([1, 2]);
    root.slaAdmin.setSort("created"); // toggle → DESC
    expect(root.slaAdmin.order).toBe("DESC");
    expect(root.slaAdmin.sortedRows.map((r) => r.id)).toEqual([2, 1]);
  });

  it("tracks dirty state from the baseline", () => {
    root.slaAdmin.openCreate();
    expect(root.slaAdmin.formDirty).toBe(false);
    root.slaAdmin.setName("Gold SLA");
    expect(root.slaAdmin.formDirty).toBe(true);
  });

  it("create: sends the reconciled field set and shows a success banner", async () => {
    let body: Record<string, unknown> | undefined;
    server.use(
      mockList(),
      http.post("/api/staff/admin/sla", async ({ request }) => {
        body = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json({ id: 9 });
      }),
    );
    const s = root.slaAdmin;
    s.openCreate();
    s.setName("Gold SLA");
    s.setGracePeriod("24");
    s.setEnablePriorityEscalation(true);
    const ok = await s.save();
    expect(ok).toBe(true);
    // camelCase body keys — match the backend SlaWriteRequest (serde camelCase);
    // snake_case keys deserialize as absent and wrongly trip "grace period required".
    expect(body).toMatchObject({
      name: "Gold SLA",
      gracePeriod: 24,
      isactive: true,
      enablePriorityEscalation: true,
      transient: false,
      disableOverdueAlerts: false,
    });
    expect(body).not.toHaveProperty("grace_period");
    expect(root.snackbar.message).toBe("Gold SLA added successfully");
  });

  it("create: a 422 maps to inline field errors (dialog stays open)", async () => {
    server.use(
      http.post("/api/staff/admin/sla", () =>
        HttpResponse.json(
          { error: { message: "bad", fields: { name: "SLA name is required.", grace_period: "Grace period is required." } } },
          { status: 422 },
        ),
      ),
    );
    const s = root.slaAdmin;
    s.openCreate();
    const ok = await s.save();
    expect(ok).toBe(false);
    expect(s.formOpen).toBe(true);
    expect(s.fieldError("name")).toBe("SLA name is required.");
    expect(s.fieldError("grace_period")).toBe("Grace period is required.");
  });

  it("edit: loads a row into the form with a clean baseline", () => {
    const s = root.slaAdmin;
    s.rows = LIST;
    s.openEdit(LIST[1]);
    expect(s.name).toBe("Silver SLA");
    expect(s.gracePeriod).toBe("8");
    expect(s.isEditing).toBe(true);
    expect(s.formDirty).toBe(false);
  });

  it("default-delete-protection: selecting the default blocks Delete client-side (no round trip)", async () => {
    server.use(mockList());
    await root.slaAdmin.loadList();
    const s = root.slaAdmin;
    s.toggleSelect(1); // the default SLA
    expect(s.selectionHasDefault).toBe(true);
    expect(s.canDeleteSelection).toBe(false);
    const ok = await s.massAction("delete");
    expect(ok).toBe(false);
    expect(s.massError).toBe(DEFAULT_SLA_DELETE_MESSAGE);
  });

  it("mass delete of a non-default plan calls the endpoint and clears selection", async () => {
    let called: Record<string, unknown> | undefined;
    server.use(
      mockList(),
      http.post("/api/staff/admin/sla/mass", async ({ request }) => {
        called = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json({ affected: 1, message: "1 SLA plan(s) deleted" });
      }),
    );
    const s = root.slaAdmin;
    await s.loadList();
    s.toggleSelect(2); // Silver (non-default)
    const ok = await s.massAction("delete");
    expect(ok).toBe(true);
    expect(called).toMatchObject({ action: "delete", ids: [2] });
    expect(s.selectedIds).toHaveLength(0);
  });

  it("mass activate calls the endpoint", async () => {
    let called: Record<string, unknown> | undefined;
    server.use(
      mockList(),
      http.post("/api/staff/admin/sla/mass", async ({ request }) => {
        called = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json({ affected: 1, message: "activated" });
      }),
    );
    const s = root.slaAdmin;
    await s.loadList();
    s.toggleSelect(2);
    await s.massAction("activate");
    expect(called).toMatchObject({ action: "activate", ids: [2] });
  });
});
