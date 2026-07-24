import { describe, expect, it } from "vitest";
import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithProviders } from "../../test/renderWithProviders";
import { server, http, HttpResponse } from "../../test/mockServer";
import { RootStore } from "../../stores/RootStore";
import { DeptListPage } from "./DeptListPage";

/** TS-M4-C2 UI: list + default-row protection, Email/Template/SLA selects, inline errors. */

const LIST = [
  { id: 1, name: "Support", ispublic: true, is_default: true, manager_id: null, manager_name: null, email_id: 5, tpl_id: 1, sla_id: 2, sla_name: "Default SLA", user_count: 3, updated: null },
  { id: 2, name: "Sales", ispublic: true, is_default: false, manager_id: 9, manager_name: "Jo Manager", email_id: 5, tpl_id: 1, sla_id: null, sla_name: null, user_count: 0, updated: null },
];
const OPTIONS = {
  email_accounts: [{ id: 5, email: "support@osticket.local", name: "Support" }],
  template_groups: [{ id: 1, name: "osTicket Default" }],
  sla: [{ id: 2, name: "Default SLA" }],
  staff: [{ id: 9, name: "Jo Manager" }],
  groups: [{ id: 3, name: "Agents" }],
};
const DETAIL_DEFAULT = { ...LIST[0], group_ids: [3], ticket_auto_response: true, message_auto_response: true, autoresp_email_id: 5, dept_signature: "", group_membership: 0 };

function base() {
  return [
    http.get("/api/staff/admin/departments", () => HttpResponse.json(LIST)),
    http.get("/api/staff/admin/departments/form-options", () => HttpResponse.json(OPTIONS)),
  ];
}
function adminStore() {
  const store = new RootStore({ onUnauthorized: () => {} });
  store.staffAuth.user = { id: 1, username: "admin", isadmin: true };
  store.staffAuth.profileLoaded = "loaded";
  return store;
}

describe("DeptListPage (TS-M4-C2)", () => {
  it("AC-1: renders the list; the default (Support) row checkbox is disabled", async () => {
    server.use(...base());
    renderWithProviders(<DeptListPage />, { store: adminStore() });
    expect(await screen.findByText("Support (default)")).toBeInTheDocument();
    expect(within(screen.getByTestId("dept-check-1")).getByRole("checkbox")).toBeDisabled();
    expect(within(screen.getByTestId("dept-check-2")).getByRole("checkbox")).not.toBeDisabled();
    expect(screen.getByTestId("dept-users-1")).toHaveTextContent("3");
  });

  it("AC-2: the form lists Email / Template / SLA options and saves", async () => {
    let body: Record<string, unknown> | undefined;
    server.use(
      ...base(),
      http.post("/api/staff/admin/departments", async ({ request }) => {
        body = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json({ id: 7 });
      }),
    );
    const user = userEvent.setup();
    const store = adminStore();
    renderWithProviders(<DeptListPage />, { store });
    await screen.findByText("Support (default)");
    await user.click(screen.getByTestId("add-department"));
    const dialog = await screen.findByRole("dialog");
    await user.type(within(dialog).getByTestId("dept-name"), "Billing");
    await user.click(within(dialog).getByLabelText(/Email/));
    await user.click(await screen.findByRole("option", { name: "support@osticket.local" }));
    await user.click(within(dialog).getByLabelText(/Template/));
    await user.click(await screen.findByRole("option", { name: "osTicket Default" }));
    await user.click(within(dialog).getByLabelText("SLA Plan"));
    await user.click(await screen.findByRole("option", { name: "Default SLA" }));
    await user.click(within(dialog).getByTestId("dept-save"));
    await waitFor(() => expect(store.snackbar.message).toBe("Billing added successfully"));
    expect(body).toMatchObject({ name: "Billing", emailId: 5, tplId: 1, slaId: 2 });
  });

  it("AC-3: missing Email then Template surface inline errors", async () => {
    server.use(...base());
    const user = userEvent.setup();
    renderWithProviders(<DeptListPage />, { store: adminStore() });
    await screen.findByText("Support (default)");
    await user.click(screen.getByTestId("add-department"));
    const dialog = await screen.findByRole("dialog");
    await user.type(within(dialog).getByTestId("dept-name"), "Billing");
    await user.click(within(dialog).getByTestId("dept-save"));
    expect(await within(dialog).findByText("Email selection required")).toBeInTheDocument();
    await user.click(within(dialog).getByLabelText(/Email/));
    await user.click(await screen.findByRole("option", { name: "support@osticket.local" }));
    await user.click(within(dialog).getByTestId("dept-save"));
    expect(await within(dialog).findByText("Template selection required")).toBeInTheDocument();
  });

  it("AC-4: setting the default department private is rejected inline", async () => {
    server.use(
      ...base(),
      http.get("/api/staff/admin/departments/1", () => HttpResponse.json(DETAIL_DEFAULT)),
    );
    const user = userEvent.setup();
    renderWithProviders(<DeptListPage />, { store: adminStore() });
    await screen.findByText("Support (default)");
    await user.click(screen.getByTestId("dept-edit-1"));
    const dialog = await screen.findByRole("dialog");
    // wait for detail to load (name prefilled)
    await waitFor(() => expect(within(dialog).getByTestId("dept-name")).toHaveValue("Support"));
    await user.click(within(dialog).getByTestId("dept-ispublic")); // toggle public → off
    await user.click(within(dialog).getByTestId("dept-save"));
    expect(await within(dialog).findByText("System default department cannot be private")).toBeInTheDocument();
  });
});
