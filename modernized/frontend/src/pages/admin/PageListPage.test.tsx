import { describe, expect, it } from "vitest";
import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithProviders } from "../../test/renderWithProviders";
import { server, http, HttpResponse } from "../../test/mockServer";
import { RootStore } from "../../stores/RootStore";
import { PageListPage } from "./PageListPage";

/** TS-M4-F2 UI: list (name/type/status/in-use) + sort/pagination + create + guarded delete. */

const ROWS = [
  { id: 1, name: "Welcome Landing", type: "landing", isactive: true, in_use: false, created: "2024-01-01T00:00:00Z", updated: null },
  { id: 2, name: "Offline Notice", type: "offline", isactive: true, in_use: true, created: "2024-02-01T00:00:00Z", updated: null },
];

function listPayload(items: unknown[], total = items.length) {
  return { items, pagination: { page: 1, per_page: 25, total } };
}
function mockList() {
  return http.get("/api/staff/admin/pages", () => HttpResponse.json(listPayload(ROWS)));
}
function authedStore() {
  const store = new RootStore({ onUnauthorized: () => {} });
  store.staffAuth.user = { id: 1, username: "admin", isadmin: true };
  store.staffAuth.profileLoaded = "loaded";
  return store;
}

describe("PageListPage (TS-M4-F2)", () => {
  it("renders name/type/status + in-use badge, sortable headers (AC-1)", async () => {
    server.use(mockList());
    renderWithProviders(<PageListPage />, { store: authedStore() });
    await screen.findByText("Welcome Landing");
    expect(screen.getByTestId("page-type-1")).toHaveTextContent("landing");
    expect(screen.getByTestId("page-sort-name")).toBeInTheDocument();
    expect(screen.getByTestId("page-inuse-2")).toHaveTextContent("In use");
  });

  it("creates a thank-you page and shows success (AC-2)", async () => {
    server.use(
      mockList(),
      http.post("/api/staff/admin/pages", () => HttpResponse.json({ id: 9 })),
    );
    const user = userEvent.setup();
    const store = authedStore();
    renderWithProviders(<PageListPage />, { store });
    await screen.findByText("Welcome Landing");
    await user.click(screen.getByTestId("add-page"));
    const dialog = await screen.findByRole("dialog");
    await user.type(within(dialog).getByTestId("page-name"), "Thanks Page");
    // select type = thank-you (MUI Select opens via its label; options render as role=option)
    await user.click(within(dialog).getByLabelText("Type"));
    await user.click(await screen.findByRole("option", { name: "thank-you" }));
    await user.type(within(dialog).getByTestId("page-body"), "Thank you");
    await user.click(within(dialog).getByTestId("page-save"));
    await waitFor(() => expect(store.snackbar.message).toBe("Thanks Page added successfully"));
  });

  it("duplicate name → inline error, dialog stays open (AC-3)", async () => {
    server.use(
      mockList(),
      http.post("/api/staff/admin/pages", () =>
        HttpResponse.json({ error: { message: "bad", fields: { name: "A page with this name already exists." } } }, { status: 422 }),
      ),
    );
    const user = userEvent.setup();
    renderWithProviders(<PageListPage />, { store: authedStore() });
    await screen.findByText("Welcome Landing");
    await user.click(screen.getByTestId("add-page"));
    const dialog = await screen.findByRole("dialog");
    await user.type(within(dialog).getByTestId("page-name"), "Welcome Landing");
    await user.click(within(dialog).getByTestId("page-save"));
    expect(await within(dialog).findByText("A page with this name already exists.")).toBeInTheDocument();
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });

  it("in-use page: selecting it disables both Delete and Disable (AC-4)", async () => {
    server.use(mockList());
    const user = userEvent.setup();
    renderWithProviders(<PageListPage />, { store: authedStore() });
    await screen.findByText("Offline Notice");
    await user.click(within(screen.getByTestId("page-check-2")).getByRole("checkbox"));
    expect(screen.getByTestId("page-mass-delete")).toBeDisabled();
    expect(screen.getByTestId("page-mass-disable")).toBeDisabled();
  });
});
