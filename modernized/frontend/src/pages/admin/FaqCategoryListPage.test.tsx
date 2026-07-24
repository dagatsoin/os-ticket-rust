import { describe, expect, it } from "vitest";
import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithProviders } from "../../test/renderWithProviders";
import { server, http, HttpResponse } from "../../test/mockServer";
import { RootStore } from "../../stores/RootStore";
import { FaqCategoryListPage } from "./FaqCategoryListPage";

/** TS-M4-E2 UI: list + create + edit-type + delete, reachable by a can_manage_faq non-admin. */

const LIST = [
  { id: 1, name: "Getting Started", ispublic: true, description: "Basics", notes: "", updated: null },
];
const DETAIL = LIST[0];

function base() {
  return [http.get("/api/staff/faq-categories", () => HttpResponse.json(LIST))];
}
/** A delegated (non-admin) FAQ manager. */
function faqStore() {
  const store = new RootStore({ onUnauthorized: () => {} });
  store.staffAuth.user = { id: 5, username: "faqmgr", isadmin: false, can_manage_faq: true };
  store.staffAuth.profileLoaded = "loaded";
  return store;
}

describe("FaqCategoryListPage (TS-M4-E2)", () => {
  it("lists categories with name / type / description", async () => {
    server.use(...base());
    renderWithProviders(<FaqCategoryListPage />, { store: faqStore() });
    expect(await screen.findByText("Getting Started")).toBeInTheDocument();
    expect(screen.getByTestId("faq-type-1")).toHaveTextContent("Public");
    expect(screen.getByTestId("faq-desc-1")).toHaveTextContent("Basics");
  });

  it("AC-2: creates a category (public)", async () => {
    let body: Record<string, unknown> | undefined;
    server.use(
      ...base(),
      http.post("/api/staff/faq-categories", async ({ request }) => {
        body = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json({ id: 9 });
      }),
    );
    const user = userEvent.setup();
    const store = faqStore();
    renderWithProviders(<FaqCategoryListPage />, { store });
    await screen.findByText("Getting Started");
    await user.click(screen.getByTestId("add-faq-category"));
    const dialog = await screen.findByRole("dialog");
    await user.type(within(dialog).getByTestId("faq-name"), "Billing");
    await user.type(within(dialog).getByTestId("faq-description"), "Payment questions");
    await user.click(within(dialog).getByTestId("faq-save"));
    await waitFor(() => expect(store.snackbar.message).toBe("Billing added successfully"));
    expect(body).toMatchObject({ name: "Billing", ispublic: true, description: "Payment questions" });
  });

  it("AC-3: edits the type and deletes from the list", async () => {
    let putBody: Record<string, unknown> | undefined;
    server.use(
      ...base(),
      http.get("/api/staff/faq-categories/1", () => HttpResponse.json(DETAIL)),
      http.put("/api/staff/faq-categories/1", async ({ request }) => {
        putBody = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json({ id: 1 });
      }),
      http.delete("/api/staff/faq-categories/1", () => HttpResponse.json({ ok: true })),
    );
    const user = userEvent.setup();
    const store = faqStore();
    renderWithProviders(<FaqCategoryListPage />, { store });
    await screen.findByText("Getting Started");
    // edit type public → private
    await user.click(screen.getByTestId("faq-edit-1"));
    const dialog = await screen.findByRole("dialog");
    await waitFor(() => expect(within(dialog).getByTestId("faq-name")).toHaveValue("Getting Started"));
    await user.click(within(dialog).getByTestId("faq-ispublic")); // public → off
    await user.click(within(dialog).getByTestId("faq-save"));
    await waitFor(() => expect(store.snackbar.message).toBe("Getting Started updated successfully"));
    expect(putBody).toMatchObject({ ispublic: false });
    // delete
    await user.click(screen.getByTestId("faq-delete-1"));
    await user.click(await screen.findByTestId("faq-delete-confirm"));
    await waitFor(() => expect(store.snackbar.message).toBe("FAQ category deleted"));
  });
});
