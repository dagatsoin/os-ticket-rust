import { describe, expect, it } from "vitest";
import { screen } from "@testing-library/react";
import { renderWithProviders } from "../../test/renderWithProviders";
import { server, http, HttpResponse } from "../../test/mockServer";
import { RootStore } from "../../stores/RootStore";
import { PrioritiesPage } from "./PrioritiesPage";

/** TS-M4-D3 UI: read-only priority set, NO CRUD controls (KL-032.1 preserved). */

const PRIORITIES = [
  { priority_id: 1, priority: "Low", priority_desc: "Low", priority_color: "#DDFFDD", urgency: 4, ispublic: true },
  { priority_id: 2, priority: "Normal", priority_desc: "Normal", priority_color: "#FFFFF0", urgency: 3, ispublic: true },
  { priority_id: 3, priority: "High", priority_desc: "High", priority_color: "#FEE7E7", urgency: 2, ispublic: true },
  { priority_id: 4, priority: "Emergency", priority_desc: "Emergency", priority_color: "#FEE7E7", urgency: 1, ispublic: true },
];

function authedStore() {
  const store = new RootStore({ onUnauthorized: () => {} });
  store.staffAuth.user = { id: 1, username: "admin", isadmin: true };
  store.staffAuth.profileLoaded = "loaded";
  return store;
}

describe("PrioritiesPage (TS-M4-D3)", () => {
  it("renders the fixed set in the backend order with rank + colour (AC-1)", async () => {
    server.use(http.get("/api/staff/admin/priorities", () => HttpResponse.json(PRIORITIES)));
    renderWithProviders(<PrioritiesPage />, { store: authedStore() });
    await screen.findByText("Low");
    const rows = screen.getAllByTestId("priority-row");
    expect(rows).toHaveLength(4);
    expect(screen.getByTestId("priority-name-1")).toHaveTextContent("Low");
    expect(screen.getByTestId("priority-name-4")).toHaveTextContent("Emergency");
    expect(screen.getByTestId("priority-rank-1")).toHaveTextContent("4");
    expect(screen.getByTestId("priority-swatch-1")).toBeInTheDocument();
  });

  it("has NO add / edit / delete / mass controls (read-only, KL-032.1)", async () => {
    server.use(http.get("/api/staff/admin/priorities", () => HttpResponse.json(PRIORITIES)));
    renderWithProviders(<PrioritiesPage />, { store: authedStore() });
    await screen.findByText("Low");
    expect(screen.queryByRole("button", { name: /add/i })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /delete/i })).not.toBeInTheDocument();
    expect(screen.queryByRole("checkbox")).not.toBeInTheDocument();
  });
});
