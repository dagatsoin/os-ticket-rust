import { beforeEach, describe, expect, it } from "vitest";
import { server, http, HttpResponse } from "../test/mockServer";
import { readMultipart } from "../test/mockServer";
import { RootStore } from "./RootStore";

/**
 * TS-M4-H2 business logic (TDD): list, dept-options, form dirty, attachment
 * staging + client-side validation, the multipart write body (title/dept_id/
 * response/isenabled/notes/files/keep_file_ids), 422 → inline title error,
 * mass actions.
 */

const LIST = [
  { id: 1, title: "Welcome", dept_id: null, dept_name: null, isenabled: true, notes: "", attachment_count: 0, updated: null },
  { id: 2, title: "Refund", dept_id: 2, dept_name: "Support", isenabled: true, notes: "", attachment_count: 1, updated: null },
];

const DEPT_OPTIONS = [
  { id: 2, name: "Support" },
  { id: 3, name: "Sales" },
];

const DETAIL = {
  id: 2,
  title: "Refund",
  dept_id: 2,
  dept_name: "Support",
  isenabled: true,
  notes: "n",
  response: "Your refund %{ticket.name}",
  attachment_count: 1,
  updated: null,
  attachments: [{ id: 50, name: "policy.pdf", size: 1000, mime: "application/pdf" }],
};

function mockList() {
  return http.get("/api/staff/canned-responses", () => HttpResponse.json(LIST));
}
function mockOptions() {
  return http.get("/api/staff/canned-responses/dept-options", () => HttpResponse.json(DEPT_OPTIONS));
}

describe("CannedAdminStore (TS-M4-H2)", () => {
  let root: RootStore;
  beforeEach(() => {
    root = new RootStore();
  });

  it("loads the list", async () => {
    server.use(mockList());
    await root.canned.loadList();
    expect(root.canned.rows).toHaveLength(2);
    expect(root.canned.rows[1]).toMatchObject({ title: "Refund", dept_name: "Support", attachment_count: 1 });
  });

  it("loads dept options from the premade-gated endpoint (not the admin gate)", async () => {
    server.use(mockOptions());
    await root.canned.loadOptions();
    expect(root.canned.deptOptions.map((d) => d.name)).toEqual(["Support", "Sales"]);
  });

  it("tracks dirty state from the baseline", async () => {
    server.use(mockOptions());
    root.canned.openCreate();
    expect(root.canned.formDirty).toBe(false);
    root.canned.setTitle("Greeting");
    expect(root.canned.formDirty).toBe(true);
  });

  it("rejects a bad attachment client-side and blocks save", async () => {
    server.use(mockOptions());
    const s = root.canned;
    s.openCreate();
    const bad = new File(["x"], "virus.exe", { type: "application/octet-stream" });
    s.addFile(bad);
    expect(s.fileError).toBeDefined();
    expect(s.stagedFiles).toHaveLength(0);
    const ok = await s.save();
    expect(ok).toBe(false);
  });

  it("create: posts a multipart body with title/response/isenabled + a staged file", async () => {
    let captured: Awaited<ReturnType<typeof readMultipart>> | undefined;
    server.use(
      mockList(),
      mockOptions(),
      http.post("/api/staff/canned-responses", async ({ request }) => {
        captured = await readMultipart(request as Request);
        return HttpResponse.json({ id: 9 });
      }),
    );
    const s = root.canned;
    s.openCreate();
    s.setTitle("Greeting");
    s.setResponse("Hello %{ticket.name}");
    const good = new File(["hi"], "note.txt", { type: "text/plain" });
    s.addFile(good);
    expect(s.stagedFiles).toHaveLength(1);
    const ok = await s.save();
    expect(ok).toBe(true);
    expect(captured?.fields.title).toBe("Greeting");
    expect(captured?.fields.response).toBe("Hello %{ticket.name}");
    expect(captured?.fields.isenabled).toBe("1");
    expect(captured?.files.files?.name).toBe("note.txt");
    expect(root.snackbar.message).toBe("Greeting added successfully");
  });

  it("edit: loads detail + attachments; save sends keep_file_ids for retained files", async () => {
    let captured: Awaited<ReturnType<typeof readMultipart>> | undefined;
    server.use(
      mockList(),
      mockOptions(),
      http.get("/api/staff/canned-responses/2", () => HttpResponse.json(DETAIL)),
      http.put("/api/staff/canned-responses/2", async ({ request }) => {
        captured = await readMultipart(request as Request);
        return HttpResponse.json({ id: 2 });
      }),
    );
    const s = root.canned;
    await s.openEdit(2);
    expect(s.title).toBe("Refund");
    expect(s.response).toBe("Your refund %{ticket.name}");
    expect(s.keptAttachments.map((a) => a.id)).toEqual([50]);
    expect(s.formDirty).toBe(false);
    s.setResponse("Updated");
    const ok = await s.save();
    expect(ok).toBe(true);
    expect(captured?.fields["keep_file_ids[]"]).toBe("50");
    expect(captured?.fields.response).toBe("Updated");
  });

  it("edit: removing a persisted attachment drops it from keep_file_ids", async () => {
    server.use(
      mockOptions(),
      http.get("/api/staff/canned-responses/2", () => HttpResponse.json(DETAIL)),
    );
    const s = root.canned;
    await s.openEdit(2);
    expect(s.isKept(50)).toBe(true);
    s.removeExisting(50);
    expect(s.isKept(50)).toBe(false);
    expect(s.keptAttachments).toHaveLength(0);
    expect(s.formDirty).toBe(true);
  });

  it("create: duplicate title → inline field error", async () => {
    server.use(
      mockOptions(),
      http.post("/api/staff/canned-responses", () =>
        HttpResponse.json({ error: { message: "bad", fields: { title: "A canned response with this title already exists." } } }, { status: 422 }),
      ),
    );
    const s = root.canned;
    s.openCreate();
    s.setTitle("Welcome");
    const ok = await s.save();
    expect(ok).toBe(false);
    expect(s.formOpen).toBe(true);
    expect(s.fieldError("title")).toMatch(/already exists/);
  });

  it("mass disable calls the endpoint and clears selection", async () => {
    let called: Record<string, unknown> | undefined;
    server.use(
      mockList(),
      http.post("/api/staff/canned-responses/mass", async ({ request }) => {
        called = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json({ affected: 1, message: "disabled" });
      }),
    );
    const s = root.canned;
    await s.loadList();
    s.toggleSelect(1);
    const ok = await s.massAction("disable");
    expect(ok).toBe(true);
    expect(called).toMatchObject({ action: "disable", ids: [1] });
    expect(s.selectedIds).toHaveLength(0);
  });
});
