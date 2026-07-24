import { describe, expect, it } from "vitest";
import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithProviders } from "../../test/renderWithProviders";
import { server, http, HttpResponse } from "../../test/mockServer";
import { RootStore } from "../../stores/RootStore";
import { CannedListPage } from "./CannedListPage";

/** TS-M4-H2 UI: list, Add form (title/dept/%{token} body/attachments), duplicate 422. */

const LIST = [
  { id: 1, title: "Welcome", dept_id: null, dept_name: null, isenabled: true, notes: "", attachment_count: 0, updated: null },
  { id: 2, title: "Refund", dept_id: 2, dept_name: "Support", isenabled: true, notes: "", attachment_count: 1, updated: null },
];
const DEPT_OPTIONS = [
  { id: 2, name: "Support" },
  { id: 3, name: "Sales" },
];

function baseHandlers() {
  return [
    http.get("/api/staff/canned-responses", () => HttpResponse.json(LIST)),
    http.get("/api/staff/canned-responses/dept-options", () => HttpResponse.json(DEPT_OPTIONS)),
  ];
}
/** A delegated (non-admin) premade manager. */
function premadeStore() {
  const store = new RootStore({ onUnauthorized: () => {} });
  store.staffAuth.user = { id: 5, username: "premade1", isadmin: false, can_manage_premade: true };
  store.staffAuth.profileLoaded = "loaded";
  return store;
}

describe("CannedListPage (TS-M4-H2)", () => {
  it("lists the seeded responses with title/dept/status/attachment count", async () => {
    server.use(...baseHandlers());
    renderWithProviders(<CannedListPage />, { store: premadeStore() });
    expect(await screen.findByText("Welcome")).toBeInTheDocument();
    expect(screen.getByTestId("canned-dept-2")).toHaveTextContent("Support");
    expect(screen.getByTestId("canned-attcount-2")).toHaveTextContent("1");
  });

  it("creates a response with a %{token} body (AC-2)", async () => {
    let captured: { fields: Record<string, string> } | undefined;
    const { readMultipart } = await import("../../test/mockServer");
    server.use(
      ...baseHandlers(),
      http.post("/api/staff/canned-responses", async ({ request }) => {
        captured = await readMultipart(request as Request);
        return HttpResponse.json({ id: 9 });
      }),
    );
    const user = userEvent.setup();
    const store = premadeStore();
    renderWithProviders(<CannedListPage />, { store });
    await screen.findByText("Welcome");
    await user.click(screen.getByTestId("add-canned"));
    const dialog = await screen.findByRole("dialog");
    await user.type(within(dialog).getByTestId("canned-title"), "Greeting");
    // userEvent.type treats "{" as special key syntax → escape it as "{{".
    await user.type(within(dialog).getByTestId("canned-body"), "Hello %{{ticket.name}");
    await user.click(within(dialog).getByTestId("canned-save"));
    await waitFor(() => expect(store.snackbar.message).toBe("Greeting added successfully"));
    expect(captured?.fields.title).toBe("Greeting");
    expect(captured?.fields.response).toBe("Hello %{ticket.name}");
  });

  it("populates the department dropdown from the premade-gated options endpoint", async () => {
    server.use(...baseHandlers());
    const user = userEvent.setup();
    renderWithProviders(<CannedListPage />, { store: premadeStore() });
    await screen.findByText("Welcome");
    await user.click(screen.getByTestId("add-canned"));
    const dialog = await screen.findByRole("dialog");
    await user.click(within(dialog).getByLabelText("Department"));
    expect(await screen.findByRole("option", { name: "Support" })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "Sales" })).toBeInTheDocument();
  });

  it("duplicate title → inline error, dialog stays open", async () => {
    server.use(
      ...baseHandlers(),
      http.post("/api/staff/canned-responses", () =>
        HttpResponse.json({ error: { message: "bad", fields: { title: "A canned response with this title already exists." } } }, { status: 422 }),
      ),
    );
    const user = userEvent.setup();
    renderWithProviders(<CannedListPage />, { store: premadeStore() });
    await screen.findByText("Welcome");
    await user.click(screen.getByTestId("add-canned"));
    const dialog = await screen.findByRole("dialog");
    await user.type(within(dialog).getByTestId("canned-title"), "Welcome");
    await user.click(within(dialog).getByTestId("canned-save"));
    expect(await within(dialog).findByText(/already exists/)).toBeInTheDocument();
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });
});
