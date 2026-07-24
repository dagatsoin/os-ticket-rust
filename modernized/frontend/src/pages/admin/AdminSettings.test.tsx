import { describe, expect, it } from "vitest";
import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithProviders } from "../../test/renderWithProviders";
import { server, http, HttpResponse } from "../../test/mockServer";
import { RootStore } from "../../stores/RootStore";
import { AppRoutes } from "../../routes/router";

/**
 * TS-M4-A0/A2/A3 UI, driven through the full router (AppShell → guards →
 * AdminLayout → SettingsPage) against MSW. Covers nav composition, the tab strip,
 * per-tab save + success snackbar, inline 422 errors, and the Attachments screen
 * master-switch gating.
 */

const ADMIN_ME = { id: 1, username: "admin", name: "Admin", isadmin: true };

const SETTINGS = {
  tabs: {
    system: { helpdesk_title: "osTicket", helpdesk_url: "http://localhost", max_page_size: 25, isonline: "1" },
    tickets: { default_priority_id: 2, max_open_tickets: 10, enable_captcha: 0 },
    emails: { admin_email: "admin@osticket.local", default_email_id: 1, default_template_id: 1 },
    pages: { landing_page_id: "" },
    kb: { enable_kb: 1, enable_premade: 0 },
    autoresp: { ticket_autoresponder: 1 },
    alerts: { ticket_alert_active: 1, ticket_alert_admin: 1 },
    attach: { allow_attachments: 1, allowed_filetypes: ".pdf", max_file_size: 1048576 },
  },
  options: {
    departments: [{ id: 2, name: "Support" }],
    priorities: [
      { id: 2, name: "Normal" },
      { id: 3, name: "High" },
    ],
    sla_plans: [{ id: 1, name: "Default" }],
    help_topics: [{ id: 1, name: "General" }],
    email_accounts: [{ id: 1, email: "support@osticket.local", name: "Support" }],
    template_groups: [{ id: 1, name: "osTicket Default" }],
    timezones: [{ id: 1, label: "UTC" }],
    pages: [{ id: 5, name: "Welcome" }],
  },
};

function mockAdmin(putResolver?: Parameters<typeof http.put>[1]) {
  server.use(
    http.get("/api/staff/me", () => HttpResponse.json(ADMIN_ME)),
    http.get("/api/staff/admin/settings", () => HttpResponse.json(SETTINGS)),
  );
  if (putResolver) server.use(http.put("/api/staff/admin/settings", putResolver));
}

function renderAdmin(route = "/staff/admin") {
  const store = new RootStore({ onUnauthorized: () => {} });
  // renderWithProviders returns the same store on its result.
  return renderWithProviders(<AppRoutes />, { route, store });
}

describe("Admin System Settings UI (TS-M4-A2)", () => {
  it("renders the admin nav and a seven-tab System Settings screen with the System tab active", async () => {
    mockAdmin();
    renderAdmin();

    // System tab body (default active) shows the helpdesk name value.
    expect(await screen.findByDisplayValue("osTicket")).toBeInTheDocument();

    // The seven tabs are present.
    for (const label of [
      "System",
      "Ticket Settings",
      "Email",
      "Site Pages",
      "Knowledgebase",
      "Autoresponder",
      "Alerts & Notices",
    ]) {
      expect(screen.getByRole("tab", { name: label })).toBeInTheDocument();
    }

    // Data-driven admin nav (a sample of the entries).
    const nav = screen.getByRole("navigation");
    expect(within(nav).getByText("System Settings")).toBeInTheDocument();
    expect(within(nav).getByText("Staff")).toBeInTheDocument();
    expect(within(nav).getByText("FAQ Categories")).toBeInTheDocument();
  });

  it("edits a System-tab field and saves → success snackbar (AC-1/AC-2)", async () => {
    let body: { tab: string; values: Record<string, unknown> } | undefined;
    mockAdmin(async ({ request }) => {
      body = (await request.json()) as typeof body;
      return HttpResponse.json({ tab: body!.tab, saved: true });
    });
    const user = userEvent.setup();
    renderAdmin();

    const name = await screen.findByLabelText("Helpdesk Name");
    await user.clear(name);
    await user.type(name, "Acme Helpdesk");
    await user.click(screen.getByTestId("save-system"));

    expect(await screen.findByText("System settings updated")).toBeInTheDocument();
    expect(body?.tab).toBe("system");
    expect(body?.values.helpdesk_title).toBe("Acme Helpdesk");
    expect(body?.values.isonline).toBe("1"); // unmodelled key preserved
  });

  it("changes a Ticket-tab select and saves → success snackbar (AC-3)", async () => {
    let body: { tab: string; values: Record<string, unknown> } | undefined;
    mockAdmin(async ({ request }) => {
      body = (await request.json()) as typeof body;
      return HttpResponse.json({ tab: body!.tab, saved: true });
    });
    const user = userEvent.setup();
    renderAdmin();

    await screen.findByDisplayValue("osTicket");
    await user.click(screen.getByRole("tab", { name: "Ticket Settings" }));

    const priority = await screen.findByRole("combobox", { name: /default priority/i });
    await user.click(priority);
    await user.click(await screen.findByRole("option", { name: "High" }));

    await user.click(screen.getByTestId("save-tickets"));

    expect(await screen.findByText("Ticket Settings settings updated")).toBeInTheDocument();
    expect(body?.tab).toBe("tickets");
    expect(body?.values.default_priority_id).toBe("3");
  });

  it("shows an inline error on a 422 and does not navigate away (AC-4)", async () => {
    mockAdmin(() =>
      HttpResponse.json(
        { error: { message: "Validation failed", fields: { max_open_tickets: "Enter a number" } } },
        { status: 422 },
      ),
    );
    const user = userEvent.setup();
    renderAdmin();

    await screen.findByDisplayValue("osTicket");
    await user.click(screen.getByRole("tab", { name: "Ticket Settings" }));

    const maxOpen = await screen.findByLabelText("Max Open Tickets");
    await user.clear(maxOpen);
    await user.type(maxOpen, "abc");
    await user.click(screen.getByTestId("save-tickets"));

    expect(await screen.findByText("Enter a number")).toBeInTheDocument();
    // Still on the Ticket tab with the entered value; no success banner.
    expect(screen.getByLabelText("Max Open Tickets")).toHaveValue("abc");
    expect(screen.queryByText(/settings updated/i)).not.toBeInTheDocument();
  });

  it("lists the seeded email account + template on the Email tab (US-M4-A2 AC-1)", async () => {
    mockAdmin();
    const user = userEvent.setup();
    renderAdmin();

    await screen.findByDisplayValue("osTicket");
    await user.click(screen.getByRole("tab", { name: "Email" }));

    const email = await screen.findByRole("combobox", { name: /default email/i });
    await user.click(email);
    expect(await screen.findByRole("option", { name: "support@osticket.local" })).toBeInTheDocument();
  });
});

describe("Attachments settings screen (TS-M4-A3 / US-M4-A2 AC-3)", () => {
  it("gates the sub-fields behind the Allow Attachments master switch", async () => {
    mockAdmin(async ({ request }) => {
      const body = (await request.json()) as { tab: string; values: Record<string, unknown> };
      return HttpResponse.json({ tab: body.tab, saved: true });
    });
    const user = userEvent.setup();
    renderAdmin("/staff/admin/settings/attachments");

    const filetypes = await screen.findByTestId("field-allowed_filetypes");
    // Master switch on → sub-field enabled.
    expect(filetypes).not.toBeDisabled();

    // Toggle master OFF → sub-fields disabled (BS-032.6).
    await user.click(screen.getByTestId("field-allow_attachments"));
    await waitFor(() => expect(screen.getByTestId("field-allowed_filetypes")).toBeDisabled());
    expect(screen.getByTestId("field-max_file_size")).toBeDisabled();

    // Toggle back ON → re-enabled, then save.
    await user.click(screen.getByTestId("field-allow_attachments"));
    await waitFor(() => expect(screen.getByTestId("field-allowed_filetypes")).not.toBeDisabled());
  });
});
