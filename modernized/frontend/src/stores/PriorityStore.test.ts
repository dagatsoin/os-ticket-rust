import { beforeEach, describe, expect, it } from "vitest";
import { server, http, HttpResponse } from "../test/mockServer";
import { RootStore } from "./RootStore";

/** TS-M4-D3 (TDD): thin read-only fetch; the backend returns the fixed set ordered. */

const PRIORITIES = [
  { priority_id: 1, priority: "Low", priority_desc: "Low", priority_color: "#DDFFDD", urgency: 4, ispublic: true },
  { priority_id: 2, priority: "Normal", priority_desc: "Normal", priority_color: "#FFFFF0", urgency: 3, ispublic: true },
  { priority_id: 3, priority: "High", priority_desc: "High", priority_color: "#FEE7E7", urgency: 2, ispublic: true },
  { priority_id: 4, priority: "Emergency", priority_desc: "Emergency", priority_color: "#FEE7E7", urgency: 1, ispublic: true },
];

describe("PriorityStore (TS-M4-D3)", () => {
  let root: RootStore;
  beforeEach(() => {
    root = new RootStore();
  });

  it("loads the fixed priority set in the order the backend returns it", async () => {
    server.use(http.get("/api/staff/admin/priorities", () => HttpResponse.json(PRIORITIES)));
    await root.priorities.loadList();
    expect(root.priorities.rows.map((r) => r.priority)).toEqual([
      "Low",
      "Normal",
      "High",
      "Emergency",
    ]);
  });

  it("surfaces a load error", async () => {
    server.use(
      http.get("/api/staff/admin/priorities", () =>
        HttpResponse.json({ error: { message: "boom" } }, { status: 500 }),
      ),
    );
    await root.priorities.loadList();
    expect(root.priorities.error).toBe("boom");
    expect(root.priorities.rows).toHaveLength(0);
  });
});
