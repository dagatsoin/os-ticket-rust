import { describe, expect, it } from "vitest";
import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithProviders } from "../../test/renderWithProviders";
import { server, http, HttpResponse } from "../../test/mockServer";
import { RootStore } from "../../stores/RootStore";
import { SlaListPage } from "./SlaListPage";

/** TS-M4-D2 UI: list + sortable Date Added + mass actions + default delete guard. */

const LIST = [
  { id: 1, name: "Default SLA", grace_period: 24, isactive: true, enable_priority_escalation: false, transient: false, disable_overdue_alerts: false, notes: "", created: "2024-01-01T00:00:00Z", updated: null, is_default: true },
  { id: 2, name: "Silver SLA", grace_period: 8, isactive: true, enable_priority_escalation: false, transient: false, disable_overdue_alerts: false, notes: "", created: "2024-03-01T00:00:00Z", updated: null, is_default: false },
];

function mockList() {
  return http.get("/api/staff/admin/sla", () => HttpResponse.json(LIST));
}
function authedStore() {
  const store = new RootStore({ onUnauthorized: () => {} });
  store.staffAuth.user = { id: 1, username: "admin", isadmin: true };
  store.staffAuth.profileLoaded = "loaded";
  return store;
}

describe("SlaListPage (TS-M4-D2)", () => {
  it("renders name / grace / status / Date Added and mass actions (AC-1)", async () => {
    server.use(mockList());
    renderWithProviders(<SlaListPage />, { store: authedStore() });
    expect(await screen.findByText("Silver SLA")).toBeInTheDocument();
    expect(screen.getByTestId("sla-grace-2")).toHaveTextContent("8");
    expect(screen.getByTestId("sla-sort-created")).toBeInTheDocument();
    expect(screen.getByTestId("sla-mass-activate")).toBeInTheDocument();
    expect(screen.getByTestId("sla-mass-disable")).toBeInTheDocument();
    expect(screen.getByTestId("sla-mass-delete")).toBeInTheDocument();
  });

  it("Date Added header toggles row order (AC-1, KL-032.10)", async () => {
    server.use(mockList());
    const user = userEvent.setup();
    const store = authedStore();
    renderWithProviders(<SlaListPage />, { store });
    await screen.findByText("Silver SLA");
    // default sort is created DESC → Silver (2024-03) first
    let rows = screen.getAllByTestId("sla-row");
    expect(within(rows[0]).getByText("Silver SLA")).toBeInTheDocument();
    await user.click(screen.getByTestId("sla-sort-created")); // → ASC
    rows = screen.getAllByTestId("sla-row");
    expect(within(rows[0]).getByText("Default SLA")).toBeInTheDocument();
  });

  it("creates an SLA plan and shows a success banner (AC-2)", async () => {
    server.use(
      mockList(),
      http.post("/api/staff/admin/sla", () => HttpResponse.json({ id: 9 })),
    );
    const user = userEvent.setup();
    const store = authedStore();
    renderWithProviders(<SlaListPage />, { store });
    await screen.findByText("Silver SLA");
    await user.click(screen.getByTestId("add-sla"));
    const dialog = await screen.findByRole("dialog");
    await user.type(within(dialog).getByTestId("sla-name"), "Gold SLA");
    await user.type(within(dialog).getByTestId("sla-grace"), "24");
    await user.click(within(dialog).getByTestId("sla-save"));
    await waitFor(() => expect(store.snackbar.message).toBe("Gold SLA added successfully"));
  });

  it("default SLA delete is blocked: its checkbox is disabled (AC-3)", async () => {
    server.use(mockList());
    renderWithProviders(<SlaListPage />, { store: authedStore() });
    await screen.findByText("Default SLA");
    const defaultCheckbox = within(screen.getByTestId("sla-check-1")).getByRole("checkbox");
    expect(defaultCheckbox).toBeDisabled();
  });
});
