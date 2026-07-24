import { describe, expect, it } from "vitest";
import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithProviders } from "../../test/renderWithProviders";
import { server, http, HttpResponse } from "../../test/mockServer";
import { RootStore } from "../../stores/RootStore";
import { GroupListPage } from "./GroupListPage";

/** TS-M4-B4 UI: list with counts, Add-group form (11 flags + dept matrix), name 422. */

const LIST = [
  { id: 3, name: "Support", enabled: true, member_count: 2, dept_count: 1, created: null, updated: null },
];
const SETTINGS = { tabs: {}, options: { departments: [{ id: 2, name: "Support" }, { id: 61, name: "Sales" }] } };

function baseHandlers() {
  return [
    http.get("/api/staff/admin/groups", () => HttpResponse.json(LIST)),
    http.get("/api/staff/admin/settings", () => HttpResponse.json(SETTINGS)),
  ];
}
function authedStore() {
  const store = new RootStore({ onUnauthorized: () => {} });
  store.staffAuth.user = { id: 1, username: "admin", isadmin: true };
  store.staffAuth.profileLoaded = "loaded";
  return store;
}

describe("GroupListPage (TS-M4-B4)", () => {
  it("renders the group list with member + dept counts", async () => {
    server.use(...baseHandlers());
    renderWithProviders(<GroupListPage />, { store: authedStore() });
    expect(await screen.findByText("Support")).toBeInTheDocument();
    expect(screen.getByTestId("group-members-3")).toHaveTextContent("2");
    expect(screen.getByTestId("group-depts-3")).toHaveTextContent("1");
  });

  it("shows all eleven Yes/No flags (incl. the four net-new) on the Add form", async () => {
    server.use(...baseHandlers());
    const user = userEvent.setup();
    renderWithProviders(<GroupListPage />, { store: authedStore() });
    await screen.findByText("Support");
    await user.click(screen.getByTestId("add-group"));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByTestId("perm-can_manage_faq")).toBeInTheDocument();
    expect(within(dialog).getByTestId("perm-can_manage_premade")).toBeInTheDocument();
    expect(within(dialog).getByTestId("perm-can_ban_emails")).toBeInTheDocument();
    expect(within(dialog).getByTestId("perm-can_view_staff_stats")).toBeInTheDocument();
    // the 7 pre-existing flags are present too (11 total)
    for (const k of ["can_create_tickets", "can_edit_tickets", "can_post_reply", "can_close_tickets", "can_assign_tickets", "can_transfer_tickets", "can_delete_tickets"]) {
      expect(within(dialog).getByTestId(`perm-${k}`)).toBeInTheDocument();
    }
    // dept matrix present
    expect(within(dialog).getByTestId("dept-checkbox-2")).toBeInTheDocument();
  });

  it("surfaces a too-short-name 422 inline under Name (dialog stays open)", async () => {
    server.use(
      ...baseHandlers(),
      http.post("/api/staff/admin/groups", () =>
        HttpResponse.json({ error: { message: "bad", fields: { name: "Group name must be at least 3 chars." } } }, { status: 422 }),
      ),
    );
    const user = userEvent.setup();
    renderWithProviders(<GroupListPage />, { store: authedStore() });
    await screen.findByText("Support");
    await user.click(screen.getByTestId("add-group"));
    const dialog = await screen.findByRole("dialog");
    await user.type(within(dialog).getByTestId("group-name"), "Ab");
    await user.click(within(dialog).getByTestId("group-save"));
    await waitFor(() => expect(screen.getByText("Group name must be at least 3 chars.")).toBeInTheDocument());
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });
});
