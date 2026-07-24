import { beforeEach, describe, expect, it } from "vitest";
import { server, http, HttpResponse } from "../test/mockServer";
import { RootStore } from "./RootStore";

/**
 * TS-M4-B6 directory business logic (TDD): server-side search + pagination
 * params, dept-option derivation, client-side department filtering.
 */

const PAYLOAD = {
  staff: [
    { id: 1, name: "Ada Lovelace", dept_name: "Support", email: "ada@x.io", phone: "5551234", phone_ext: "12", mobile: "5559999" },
    { id: 2, name: "Bob Stone", dept_name: "Sales", email: "bob@x.io", phone: "5555678", phone_ext: "", mobile: "" },
  ],
  pagination: { page: 1, per_page: 25, total: 2 },
};

function mock(capture?: (u: URL) => void) {
  return http.get("/api/staff/directory", ({ request }) => {
    if (capture) capture(new URL(request.url));
    return HttpResponse.json(PAYLOAD);
  });
}

describe("DirectoryStore (TS-M4-B6)", () => {
  let root: RootStore;
  beforeEach(() => {
    root = new RootStore();
  });

  it("loads rows and maps pagination", async () => {
    server.use(mock());
    await root.directory.loadList();
    expect(root.directory.rows).toHaveLength(2);
    expect(root.directory.pagination).toEqual({ page: 1, pageSize: 25, totalCount: 2, totalPages: 1 });
  });

  it("search sets q + resets page and serializes the querystring", async () => {
    let seen: URL | undefined;
    server.use(mock((u) => (seen = u)));
    root.directory.page = 3;
    await root.directory.search("bob");
    expect(seen?.searchParams.get("q")).toBe("bob");
    expect(seen?.searchParams.get("page")).toBe("1");
  });

  it("derives dept options from the loaded rows and filters client-side by name", async () => {
    server.use(mock());
    await root.directory.loadList();
    expect(root.directory.deptOptions).toEqual(["Sales", "Support"]);
    root.directory.setDeptFilter("Sales");
    expect(root.directory.visibleRows).toHaveLength(1);
    expect(root.directory.visibleRows[0].name).toBe("Bob Stone");
    root.directory.setDeptFilter("");
    expect(root.directory.visibleRows).toHaveLength(2);
  });

  it("setSort toggles order and resets to page 1", async () => {
    server.use(mock());
    await root.directory.loadList();
    await root.directory.setSort("email");
    expect(root.directory.sort).toBe("email");
    expect(root.directory.order).toBe("ASC");
    await root.directory.setSort("email");
    expect(root.directory.order).toBe("DESC");
  });
});
