import { beforeEach, describe, expect, it } from "vitest";
import { server, http, HttpResponse } from "../test/mockServer";
import { RootStore } from "./RootStore";

/**
 * TS-M4-A2/A3 business logic (TDD): load/coerce, per-tab dirty tracking, inline
 * 422 field errors, checkbox 0/1-by-presence serialization (BS-032.4), the
 * attachments master switch (BS-032.6), and success/error snackbar wiring.
 */

const PAYLOAD = {
  tabs: {
    system: { helpdesk_title: "osTicket", max_page_size: 25, isonline: "1" },
    tickets: { default_priority_id: 2, max_open_tickets: 0, enable_captcha: 0, log_ticket_activity: "1" },
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

function mockGet() {
  return http.get("/api/staff/admin/settings", () => HttpResponse.json(PAYLOAD));
}

describe("AdminSettingsStore (TS-M4-A2/A3)", () => {
  let root: RootStore;

  beforeEach(() => {
    root = new RootStore();
  });

  it("loads options and coerces per-tab values (checkbox → bool, others → string)", async () => {
    server.use(mockGet());
    await root.adminSettings.load();

    expect(root.adminSettings.loaded).toBe(true);
    expect(root.adminSettings.options.priorities).toHaveLength(2);
    expect(root.adminSettings.options.email_accounts[0].email).toBe("support@osticket.local");

    expect(root.adminSettings.values("system").helpdesk_title).toBe("osTicket");
    expect(root.adminSettings.values("system").max_page_size).toBe("25"); // string for the input
    expect(root.adminSettings.values("kb").enable_kb).toBe(true); // checkbox → bool
    expect(root.adminSettings.values("kb").enable_premade).toBe(false);
    expect(root.adminSettings.values("attach").allow_attachments).toBe(true);
  });

  it("tracks dirty state per tab", async () => {
    server.use(mockGet());
    await root.adminSettings.load();

    expect(root.adminSettings.isDirty("system")).toBe(false);
    root.adminSettings.setValue("system", "helpdesk_name", "Acme Helpdesk");
    expect(root.adminSettings.isDirty("system")).toBe(true);
    expect(root.adminSettings.isDirty("tickets")).toBe(false); // untouched tab stays clean
  });

  it("saves the System tab, sends { tab, values }, fires a success snackbar, clears dirty", async () => {
    let body: { tab: string; values: Record<string, unknown> } | undefined;
    server.use(
      mockGet(),
      http.put("/api/staff/admin/settings", async ({ request }) => {
        body = (await request.json()) as typeof body;
        return HttpResponse.json({ tab: body!.tab, saved: true });
      }),
    );
    await root.adminSettings.load();

    root.adminSettings.setValue("system", "helpdesk_title", "Acme Helpdesk");
    await root.adminSettings.save("system");

    expect(body?.tab).toBe("system");
    expect(body?.values.helpdesk_title).toBe("Acme Helpdesk");
    // Full-tab merge: unmodelled keys are preserved so the backend doesn't zero them.
    expect(body?.values.isonline).toBe("1");
    expect(root.snackbar.open).toBe(true);
    expect(root.snackbar.severity).toBe("success");
    expect(root.snackbar.message).toBe("System settings updated");
    expect(root.adminSettings.isDirty("system")).toBe(false); // baseline re-seeded
  });

  it("serializes edited checkboxes as explicit 1/0 (BS-032.4)", async () => {
    let body: { tab: string; values: Record<string, unknown> } | undefined;
    server.use(
      mockGet(),
      http.put("/api/staff/admin/settings", async ({ request }) => {
        body = (await request.json()) as typeof body;
        return HttpResponse.json({ tab: body!.tab, saved: 1 });
      }),
    );
    await root.adminSettings.load();

    // enable_kb starts checked; turn it OFF. enable_premade starts off; turn it ON.
    root.adminSettings.setValue("kb", "enable_kb", false);
    root.adminSettings.setValue("kb", "enable_premade", true);
    await root.adminSettings.save("kb");

    // Explicit 0/1 (verified live: the backend zeros absent checkbox keys, so we
    // must send 0 rather than omit — otherwise the loaded "1" would survive).
    expect(body?.values).toEqual({ enable_kb: 0, enable_premade: 1 });
    expect(root.snackbar.message).toBe("Knowledgebase settings updated");
  });

  it("preserves unmodelled tab keys on save so the backend doesn't zero them", async () => {
    let body: { tab: string; values: Record<string, unknown> } | undefined;
    server.use(
      mockGet(),
      http.put("/api/staff/admin/settings", async ({ request }) => {
        body = (await request.json()) as typeof body;
        return HttpResponse.json({ tab: body!.tab, saved: 1 });
      }),
    );
    await root.adminSettings.load();

    // The tickets tab has an unmodelled checkbox `log_ticket_activity: "1"` in the
    // payload; editing only the priority must not drop it from the submission.
    root.adminSettings.setValue("tickets", "default_priority_id", "3");
    await root.adminSettings.save("tickets");

    expect(body?.values.default_priority_id).toBe("3");
    expect(body?.values.log_ticket_activity).toBe("1"); // preserved verbatim
  });

  it("sends a non-numeric ticket value verbatim so the server can reject it (FS-032.2)", async () => {
    let body: { tab: string; values: Record<string, unknown> } | undefined;
    server.use(
      mockGet(),
      http.put("/api/staff/admin/settings", async ({ request }) => {
        body = (await request.json()) as typeof body;
        return HttpResponse.json(
          { error: { message: "Invalid", fields: { max_open_tickets: "Enter a number" } } },
          { status: 422 },
        );
      }),
    );
    await root.adminSettings.load();

    root.adminSettings.setValue("tickets", "max_open_tickets", "abc");
    await root.adminSettings.save("tickets");

    expect(body?.values.max_open_tickets).toBe("abc");
  });

  it("maps a 422 to inline field errors, keeps values, shows no success banner", async () => {
    server.use(
      mockGet(),
      http.put("/api/staff/admin/settings", () =>
        HttpResponse.json(
          { error: { message: "Validation failed", fields: { max_open_tickets: "Enter a number" } } },
          { status: 422 },
        ),
      ),
    );
    await root.adminSettings.load();

    root.adminSettings.setValue("tickets", "max_open_tickets", "abc");
    await root.adminSettings.save("tickets");

    expect(root.adminSettings.fieldError("tickets", "max_open_tickets")).toBe("Enter a number");
    expect(root.adminSettings.values("tickets").max_open_tickets).toBe("abc"); // preserved
    expect(root.snackbar.open).toBe(false); // no success banner
  });

  it("clears a field's inline error as soon as it is edited", async () => {
    server.use(
      mockGet(),
      http.put("/api/staff/admin/settings", () =>
        HttpResponse.json(
          { error: { message: "Validation failed", fields: { max_open_tickets: "Enter a number" } } },
          { status: 422 },
        ),
      ),
    );
    await root.adminSettings.load();
    root.adminSettings.setValue("tickets", "max_open_tickets", "abc");
    await root.adminSettings.save("tickets");
    expect(root.adminSettings.fieldError("tickets", "max_open_tickets")).toBeTruthy();

    root.adminSettings.setValue("tickets", "max_open_tickets", "10");
    expect(root.adminSettings.fieldError("tickets", "max_open_tickets")).toBeUndefined();
  });

  it("exposes the attachments master switch and reflects toggles (BS-032.6)", async () => {
    server.use(mockGet());
    await root.adminSettings.load();

    expect(root.adminSettings.attachmentsEnabled).toBe(true);
    root.adminSettings.setValue("attach", "allow_attachments", false);
    expect(root.adminSettings.attachmentsEnabled).toBe(false);
  });

  it("surfaces an error snackbar on a non-422 save failure", async () => {
    server.use(
      mockGet(),
      http.put("/api/staff/admin/settings", () =>
        HttpResponse.json({ error: { message: "Server error" } }, { status: 500 }),
      ),
    );
    await root.adminSettings.load();
    root.adminSettings.setValue("system", "helpdesk_name", "X");
    await root.adminSettings.save("system");

    expect(root.snackbar.severity).toBe("error");
    expect(root.snackbar.open).toBe(true);
  });

  it("records a load error instead of throwing", async () => {
    server.use(
      http.get("/api/staff/admin/settings", () =>
        HttpResponse.json({ error: { message: "boom" } }, { status: 500 }),
      ),
    );
    await root.adminSettings.load();
    expect(root.adminSettings.loaded).toBe(false);
    expect(root.adminSettings.loadError).toBeTruthy();
  });
});
