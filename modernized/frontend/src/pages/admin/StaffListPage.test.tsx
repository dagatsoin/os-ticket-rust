import { describe, expect, it } from "vitest";
import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithProviders } from "../../test/renderWithProviders";
import { server, http, HttpResponse } from "../../test/mockServer";
import { RootStore } from "../../stores/RootStore";
import { StaffListPage } from "./StaffListPage";

/** TS-M4-B2 UI: list render + filter, Add-staff form, duplicate-username inline error. */

const LIST = {
  staff: [
    { id: 1, name: "Ada Lovelace", username: "agent", isactive: true, onvacation: false, group_name: "Support", dept_name: "Support", created: null, lastlogin: null },
  ],
  pagination: { page: 1, per_page: 25, total: 1 },
};
const GROUPS = [{ id: 3, name: "Support", enabled: true, member_count: 1, dept_count: 1, created: null, updated: null }];
const SETTINGS = { tabs: {}, options: { departments: [{ id: 2, name: "Support" }], timezones: [{ id: 1, label: "UTC" }] } };

function baseHandlers() {
  return [
    http.get("/api/staff/admin/staff", () => HttpResponse.json(LIST)),
    http.get("/api/staff/admin/groups", () => HttpResponse.json(GROUPS)),
    http.get("/api/staff/admin/settings", () => HttpResponse.json(SETTINGS)),
  ];
}

function authedStore() {
  const store = new RootStore({ onUnauthorized: () => {} });
  store.staffAuth.user = { id: 1, username: "admin", isadmin: true };
  store.staffAuth.profileLoaded = "loaded";
  return store;
}

describe("StaffListPage (TS-M4-B2)", () => {
  it("renders the roster with the expected columns", async () => {
    server.use(...baseHandlers());
    renderWithProviders(<StaffListPage />, { store: authedStore() });
    expect(await screen.findByText("Ada Lovelace")).toBeInTheDocument();
    expect(screen.getByTestId("staff-sort-name")).toBeInTheDocument();
    expect(screen.getByTestId("staff-filter-input")).toBeInTheDocument();
  });

  it("opens the Add Staff dialog with the group + department selects", async () => {
    server.use(...baseHandlers());
    const user = userEvent.setup();
    renderWithProviders(<StaffListPage />, { store: authedStore() });
    await screen.findByText("Ada Lovelace");
    await user.click(screen.getByTestId("add-staff"));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByTestId("staff-username")).toBeInTheDocument();
    expect(within(dialog).getByTestId("staff-group")).toBeInTheDocument();
    expect(within(dialog).getByTestId("staff-dept")).toBeInTheDocument();
  });

  it("shows the member's current teams and an Add-to-team control in the edit dialog (BS-030-14)", async () => {
    const membership: Record<number, Set<number>> = { 10: new Set([1]), 11: new Set() };
    server.use(
      ...baseHandlers(),
      http.get("/api/staff/admin/teams", () =>
        HttpResponse.json([
          { id: 10, name: "Tier 1", isenabled: true, lead_id: null, lead_name: "", member_count: 1, updated: null },
          { id: 11, name: "Tier 2", isenabled: true, lead_id: null, lead_name: "", member_count: 0, updated: null },
        ]),
      ),
      http.get("/api/staff/admin/teams/:id", ({ params }) => {
        const id = Number(params.id);
        const members = Array.from(membership[id] ?? new Set<number>()).map((sid) => ({ staff_id: sid, name: "Ada Lovelace" }));
        return HttpResponse.json({ id, name: id === 10 ? "Tier 1" : "Tier 2", isenabled: true, lead_id: null, noalerts: false, notes: "", members });
      }),
    );
    const user = userEvent.setup();
    renderWithProviders(<StaffListPage />, { store: authedStore() });
    await screen.findByText("Ada Lovelace");
    await user.click(screen.getByTestId("staff-edit-1"));
    const dialog = await screen.findByRole("dialog");
    // Current membership renders as a removable chip.
    expect(await within(dialog).findByTestId("staff-team-chip-10")).toBeInTheDocument();
    // The Add-to-team picker + button are present (Tier 2 is the only available team).
    expect(within(dialog).getByTestId("staff-add-team-select")).toBeInTheDocument();
    expect(within(dialog).getByTestId("staff-add-team-btn")).toBeInTheDocument();
  });

  it("surfaces a duplicate-username 422 inline under Username (dialog stays open)", async () => {
    server.use(
      ...baseHandlers(),
      http.post("/api/staff/admin/staff", () =>
        HttpResponse.json({ error: { message: "bad", fields: { username: "Username already in use" } } }, { status: 422 }),
      ),
    );
    const user = userEvent.setup();
    renderWithProviders(<StaffListPage />, { store: authedStore() });
    await screen.findByText("Ada Lovelace");
    await user.click(screen.getByTestId("add-staff"));
    const dialog = await screen.findByRole("dialog");
    await user.type(within(dialog).getByTestId("staff-username"), "agent");
    await user.click(within(dialog).getByTestId("staff-save"));
    await waitFor(() => expect(screen.getByText("Username already in use")).toBeInTheDocument());
    expect(screen.getByRole("dialog")).toBeInTheDocument(); // still open
  });
});
