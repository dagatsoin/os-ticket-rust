import { describe, expect, it } from "vitest";
import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithProviders } from "../../test/renderWithProviders";
import { server, http, HttpResponse } from "../../test/mockServer";
import { RootStore } from "../../stores/RootStore";
import { HelpTopicListPage } from "./HelpTopicListPage";

/** TS-M4-C6 UI: paginated "Parent / Child" list, auto-assign exclusion, top-level parents. */

const PAGE1 = {
  items: [
    { topic_id: 1, topic: "General", topic_pid: null, parent_topic: null, isactive: true, ispublic: true, priority_id: 2, priority: "Normal", dept_id: 1, dept_name: "Support", sla_id: null, staff_id: null, team_id: null, page_id: null, updated: null },
    { topic_id: 2, topic: "Refund", topic_pid: 1, parent_topic: "General", isactive: true, ispublic: false, priority_id: 2, priority: "Normal", dept_id: 1, dept_name: "Support", sla_id: null, staff_id: null, team_id: null, page_id: null, updated: null },
  ],
  total: 2,
  page: 1,
  page_size: 25,
};
const OPTIONS = {
  priorities: [{ id: 2, name: "Normal" }],
  departments: [{ id: 1, name: "Support" }],
  sla: [{ id: 3, name: "Default SLA" }],
  pages: [{ id: 4, name: "Thank You" }],
  staff: [{ id: 9, name: "Jo" }],
  teams: [{ id: 7, name: "Tier 2" }],
  parent_topics: [{ id: 1, name: "General" }],
};

function base() {
  return [
    http.get("/api/staff/admin/help-topics", () => HttpResponse.json(PAGE1)),
    http.get("/api/staff/admin/help-topics/form-options", () => HttpResponse.json(OPTIONS)),
  ];
}
function adminStore() {
  const store = new RootStore({ onUnauthorized: () => {} });
  store.staffAuth.user = { id: 1, username: "admin", isadmin: true };
  store.staffAuth.profileLoaded = "loaded";
  return store;
}

describe("HelpTopicListPage (TS-M4-C6)", () => {
  it("AC-1: renders the paginated list; a nested topic shows 'Parent / Child'", async () => {
    server.use(...base());
    renderWithProviders(<HelpTopicListPage />, { store: adminStore() });
    expect(await screen.findByText("General")).toBeInTheDocument();
    expect(screen.getByText("General / Refund")).toBeInTheDocument();
  });

  it("AC-3: auto-assign staff/team are mutually exclusive", async () => {
    server.use(...base());
    const user = userEvent.setup();
    const store = adminStore();
    renderWithProviders(<HelpTopicListPage />, { store });
    await screen.findByText("General");
    await user.click(screen.getByTestId("add-help-topic"));
    const dialog = await screen.findByRole("dialog");
    await waitFor(() => expect(store.topicAdmin.optionsLoaded).toBe(true));
    // choose a staff
    await user.click(within(dialog).getByLabelText("Auto-Assign To"));
    await user.click(await screen.findByRole("option", { name: "Jo" }));
    expect(store.topicAdmin.assignStaffId).toBe(9);
    expect(store.topicAdmin.assignTeamId).toBeNull();
    // choose a team → staff clears
    await user.click(within(dialog).getByLabelText("Auto-Assign To"));
    await user.click(await screen.findByRole("option", { name: "Tier 2" }));
    expect(store.topicAdmin.assignTeamId).toBe(7);
    expect(store.topicAdmin.assignStaffId).toBeNull();
  });

  it("AC-4: parent select offers top-level topics only", async () => {
    server.use(...base());
    const user = userEvent.setup();
    const store = adminStore();
    renderWithProviders(<HelpTopicListPage />, { store });
    await screen.findByText("General");
    await user.click(screen.getByTestId("add-help-topic"));
    const dialog = await screen.findByRole("dialog");
    await waitFor(() => expect(store.topicAdmin.optionsLoaded).toBe(true));
    await user.click(within(dialog).getByLabelText("Parent Topic"));
    // "General" (top-level) is offered; the child "Refund" is not.
    expect(await screen.findByRole("option", { name: "General" })).toBeInTheDocument();
    expect(screen.queryByRole("option", { name: "Refund" })).not.toBeInTheDocument();
  });

  it("AC-2: creates with dept + priority + SLA override", async () => {
    let body: Record<string, unknown> | undefined;
    server.use(
      ...base(),
      http.post("/api/staff/admin/help-topics", async ({ request }) => {
        body = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json({ id: 9 });
      }),
    );
    const user = userEvent.setup();
    const store = adminStore();
    renderWithProviders(<HelpTopicListPage />, { store });
    await screen.findByText("General");
    await user.click(screen.getByTestId("add-help-topic"));
    const dialog = await screen.findByRole("dialog");
    await waitFor(() => expect(store.topicAdmin.optionsLoaded).toBe(true));
    await user.type(within(dialog).getByTestId("topic-text"), "Billing question");
    await user.click(within(dialog).getByLabelText(/Department/));
    await user.click(await screen.findByRole("option", { name: "Support" }));
    await user.click(within(dialog).getByLabelText(/^Priority/));
    await user.click(await screen.findByRole("option", { name: "Normal" }));
    await user.click(within(dialog).getByLabelText("SLA Override"));
    await user.click(await screen.findByRole("option", { name: "Default SLA" }));
    await user.click(within(dialog).getByTestId("topic-save"));
    await waitFor(() => expect(store.snackbar.message).toBe("Billing question added successfully"));
    expect(body).toMatchObject({ topic: "Billing question", deptId: 1, priorityId: 2, slaId: 3 });
  });
});
