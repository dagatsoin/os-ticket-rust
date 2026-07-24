import { describe, expect, it } from "vitest";
import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithProviders } from "../../test/renderWithProviders";
import { server, http, HttpResponse } from "../../test/mockServer";
import { RootStore } from "../../stores/RootStore";
import { DirectoryPage } from "./DirectoryPage";

/** TS-M4-B6 UI: read-only directory table + search + client-side dept filter. */

const PAYLOAD = {
  staff: [
    { id: 1, name: "Ada Lovelace", dept_name: "Support", email: "ada@x.io", phone: "5551234", phone_ext: "12", mobile: "5559999" },
    { id: 2, name: "Bob Stone", dept_name: "Sales", email: "bob@x.io", phone: "5555678", phone_ext: "", mobile: "" },
  ],
  pagination: { page: 1, per_page: 25, total: 2 },
};

function authedStore() {
  const store = new RootStore({ onUnauthorized: () => {} });
  store.staffAuth.user = { id: 1, username: "agent", isadmin: false };
  store.staffAuth.profileLoaded = "loaded";
  return store;
}

describe("DirectoryPage (TS-M4-B6)", () => {
  it("renders a read-only table with no edit/add controls", async () => {
    server.use(http.get("/api/staff/directory", () => HttpResponse.json(PAYLOAD)));
    renderWithProviders(<DirectoryPage />, { store: authedStore() });
    expect(await screen.findByText("Ada Lovelace")).toBeInTheDocument();
    expect(screen.getByText("Bob Stone")).toBeInTheDocument();
    // No add/edit/delete affordances.
    expect(screen.queryByText(/add staff/i)).not.toBeInTheDocument();
    expect(screen.queryByTestId("add-staff")).not.toBeInTheDocument();
    expect(screen.queryByRole("checkbox")).not.toBeInTheDocument();
  });

  it("filters client-side by the selected department", async () => {
    server.use(http.get("/api/staff/directory", () => HttpResponse.json(PAYLOAD)));
    const { store } = renderWithProviders(<DirectoryPage />, { store: authedStore() });
    await screen.findByText("Ada Lovelace");
    store.directory.setDeptFilter("Sales");
    expect(await screen.findByText("Bob Stone")).toBeInTheDocument();
    expect(screen.queryByText("Ada Lovelace")).not.toBeInTheDocument();
  });

  it("submits a search term to the backend", async () => {
    let seen: string | null = null;
    server.use(
      http.get("/api/staff/directory", ({ request }) => {
        seen = new URL(request.url).searchParams.get("q");
        return HttpResponse.json(PAYLOAD);
      }),
    );
    const user = userEvent.setup();
    renderWithProviders(<DirectoryPage />, { store: authedStore() });
    await screen.findByText("Ada Lovelace");
    await user.type(screen.getByTestId("directory-search-input"), "bob");
    await user.click(screen.getByTestId("directory-search-submit"));
    // last request carried q=bob
    expect(seen).toBe("bob");
  });
});
