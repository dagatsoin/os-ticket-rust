import { describe, expect, it } from "vitest";
import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithProviders } from "../../test/renderWithProviders";
import { server, http, HttpResponse } from "../../test/mockServer";
import { RootStore } from "../../stores/RootStore";
import { TeamListPage } from "./TeamListPage";

/** TS-M4-C4 UI: list + member counts, remove-only roster (no add control), lead reset. */

const LIST = [
  { id: 1, name: "Tier 2", isenabled: true, lead_id: 10, lead_name: "Ada", member_count: 2, updated: null },
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

function base() {
  return [
    http.get("/api/staff/admin/teams", () => HttpResponse.json(LIST)),
    http.get("/api/staff/admin/teams/1", () => HttpResponse.json(DETAIL)),
  ];
}
function adminStore() {
  const store = new RootStore({ onUnauthorized: () => {} });
  store.staffAuth.user = { id: 1, username: "admin", isadmin: true };
  store.staffAuth.profileLoaded = "loaded";
  return store;
}

describe("TeamListPage (TS-M4-C4)", () => {
  it("AC-1: the team table renders name / status / member count", async () => {
    server.use(...base());
    renderWithProviders(<TeamListPage />, { store: adminStore() });
    expect(await screen.findByText("Tier 2")).toBeInTheDocument();
    expect(screen.getByTestId("team-members-1")).toHaveTextContent("2");
    expect(screen.getByTestId("team-status-1")).toHaveTextContent("Active");
  });

  it("AC-3: the roster exposes only per-member remove checkboxes (no add-member control)", async () => {
    server.use(...base());
    const user = userEvent.setup();
    renderWithProviders(<TeamListPage />, { store: adminStore() });
    await screen.findByText("Tier 2");
    await user.click(screen.getByTestId("team-edit-1"));
    const dialog = await screen.findByRole("dialog");
    await waitFor(() => expect(within(dialog).getByTestId("team-remove-10")).toBeInTheDocument());
    expect(within(dialog).getByTestId("team-remove-11")).toBeInTheDocument();
    // No add-member affordance.
    expect(within(dialog).queryByTestId("team-add-member")).not.toBeInTheDocument();
    expect(within(dialog).queryByRole("button", { name: /add member/i })).not.toBeInTheDocument();
  });

  it("marking the lead for removal clears the lead badge", async () => {
    server.use(...base());
    const user = userEvent.setup();
    renderWithProviders(<TeamListPage />, { store: adminStore() });
    await screen.findByText("Tier 2");
    await user.click(screen.getByTestId("team-edit-1"));
    const dialog = await screen.findByRole("dialog");
    await waitFor(() => expect(within(dialog).getByTestId("team-lead-badge-10")).toBeInTheDocument());
    await user.click(within(dialog).getByTestId("team-remove-10"));
    await waitFor(() => expect(within(dialog).queryByTestId("team-lead-badge-10")).not.toBeInTheDocument());
  });
});
