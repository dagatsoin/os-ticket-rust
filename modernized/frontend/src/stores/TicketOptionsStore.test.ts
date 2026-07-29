import { beforeEach, describe, expect, it } from "vitest";
import { server, http, HttpResponse } from "../test/mockServer";
import { RootStore } from "./RootStore";

/**
 * TicketOptionsStore business logic (TDD): loads GET /api/staff/ticket-options
 * once and exposes the six reference lists that power the staff Transfer / Assign
 * / Edit dialogs and the advanced-search filters. Covers the happy path, the
 * combined staff+team assignee derivation, error handling, and load-once caching.
 */

const PAYLOAD = {
  departments: [
    { id: 2, name: "Support" },
    { id: 61, name: "Sales" },
  ],
  agents: [
    { id: 1, name: "agent" },
    { id: 87, name: "alice_agent" },
  ],
  teams: [{ id: 3, name: "Tier 2" }],
  help_topics: [
    { id: 1, name: "General" },
    { id: 2, name: "Billing" },
  ],
  priorities: [
    { id: 4, name: "Emergency" },
    { id: 2, name: "Normal" },
  ],
  sla_plans: [{ id: 1, name: "Default" }],
};

function mock(capture?: () => void) {
  return http.get("/api/staff/ticket-options", () => {
    if (capture) capture();
    return HttpResponse.json(PAYLOAD);
  });
}

describe("TicketOptionsStore (staff ticket-options)", () => {
  let root: RootStore;
  beforeEach(() => {
    root = new RootStore();
  });

  it("loads and exposes the six reference lists", async () => {
    server.use(mock());
    await root.ticketOptions.load();

    expect(root.ticketOptions.loaded).toBe(true);
    expect(root.ticketOptions.departments).toEqual(PAYLOAD.departments);
    expect(root.ticketOptions.agents).toEqual(PAYLOAD.agents);
    expect(root.ticketOptions.teams).toEqual(PAYLOAD.teams);
    expect(root.ticketOptions.helpTopics).toEqual(PAYLOAD.help_topics);
    expect(root.ticketOptions.priorities).toEqual(PAYLOAD.priorities);
    expect(root.ticketOptions.slaPlans).toEqual(PAYLOAD.sla_plans);
  });

  it("derives combined assignees: staff prefixed s<id>, teams prefixed t<id>", async () => {
    server.use(mock());
    await root.ticketOptions.load();

    expect(root.ticketOptions.assignees).toEqual([
      { id: "s1", name: "agent", type: "staff" },
      { id: "s87", name: "alice_agent", type: "staff" },
      { id: "t3", name: "Tier 2", type: "team" },
    ]);
  });

  it("load() is a no-op after a successful load (cached, fetched once)", async () => {
    let hits = 0;
    server.use(mock(() => (hits += 1)));
    await root.ticketOptions.load();
    await root.ticketOptions.load();
    expect(hits).toBe(1);
  });

  it("forcing a reload with load(true) refetches", async () => {
    let hits = 0;
    server.use(mock(() => (hits += 1)));
    await root.ticketOptions.load();
    await root.ticketOptions.load(true);
    expect(hits).toBe(2);
  });

  it("surfaces an error and leaves lists empty on failure", async () => {
    server.use(
      http.get("/api/staff/ticket-options", () =>
        HttpResponse.json({ error: { message: "boom" } }, { status: 500 }),
      ),
    );
    await root.ticketOptions.load();

    expect(root.ticketOptions.error).toBe("boom");
    expect(root.ticketOptions.loaded).toBe(false);
    expect(root.ticketOptions.departments).toEqual([]);
    expect(root.ticketOptions.assignees).toEqual([]);
  });

  it("allows a retry after an error", async () => {
    server.use(
      http.get("/api/staff/ticket-options", () =>
        HttpResponse.json({ error: { message: "boom" } }, { status: 500 }),
      ),
    );
    await root.ticketOptions.load();
    expect(root.ticketOptions.loaded).toBe(false);

    server.use(mock());
    await root.ticketOptions.load();
    expect(root.ticketOptions.loaded).toBe(true);
    expect(root.ticketOptions.departments).toHaveLength(2);
  });
});
