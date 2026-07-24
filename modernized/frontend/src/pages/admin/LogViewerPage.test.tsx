import { describe, expect, it } from "vitest";
import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithProviders } from "../../test/renderWithProviders";
import { server, http, HttpResponse } from "../../test/mockServer";
import { RootStore } from "../../stores/RootStore";
import { LogViewerPage } from "./LogViewerPage";

/** TS-M4-G3 UI: table, filters, detail dialog, bulk delete, Purge now. */

const ROWS = [
  { id: 1, log_type: "Error", title: "Login failure", created: "2024-05-01T00:00:00Z", ip_address: "127.0.0.1" },
  { id: 2, log_type: "Debug", title: "Group created", created: "2024-05-02T00:00:00Z", ip_address: "127.0.0.1" },
];

function listPayload(items: unknown[], total = items.length) {
  return { items, total, page: 1, per_page: 25 };
}
function mockList(items: unknown[] = ROWS) {
  return http.get("/api/staff/admin/logs", () => HttpResponse.json(listPayload(items)));
}
function authedStore() {
  const store = new RootStore({ onUnauthorized: () => {} });
  store.staffAuth.user = { id: 1, username: "admin", isadmin: true };
  store.staffAuth.profileLoaded = "loaded";
  return store;
}

describe("LogViewerPage (TS-M4-G3)", () => {
  it("renders rows with type/title/date + a visible Purge now (AC-1/AC-5)", async () => {
    server.use(mockList());
    renderWithProviders(<LogViewerPage />, { store: authedStore() });
    await screen.findByText("Login failure");
    expect(screen.getByTestId("log-type-1")).toHaveTextContent("Error");
    expect(screen.getByTestId("logs-purge")).toBeInTheDocument();
    expect(screen.getByTestId("log-from")).toBeInTheDocument();
    expect(screen.getByTestId("log-to")).toBeInTheDocument();
  });

  it("filter by type narrows the list on Apply (AC-2)", async () => {
    let sawType: string | null = null;
    server.use(
      http.get("/api/staff/admin/logs", ({ request }) => {
        const t = new URL(request.url).searchParams.get("type");
        sawType = t;
        return HttpResponse.json(listPayload(t === "Error" ? [ROWS[0]] : ROWS));
      }),
    );
    const user = userEvent.setup();
    renderWithProviders(<LogViewerPage />, { store: authedStore() });
    await screen.findByText("Login failure");
    await user.click(screen.getByLabelText("Type"));
    await user.click(await screen.findByRole("option", { name: "Error" }));
    await user.click(screen.getByTestId("log-apply"));
    await waitFor(() => expect(sawType).toBe("Error"));
    await waitFor(() => expect(screen.queryByText("Group created")).not.toBeInTheDocument());
  });

  it("row click opens a detail dialog with the full body (AC-3)", async () => {
    server.use(
      mockList(),
      http.get("/api/staff/admin/logs/1", () => HttpResponse.json({ ...ROWS[0], log: "Full body text here" })),
    );
    const user = userEvent.setup();
    renderWithProviders(<LogViewerPage />, { store: authedStore() });
    await screen.findByText("Login failure");
    await user.click(screen.getByTestId("log-title-1"));
    expect(await screen.findByTestId("log-detail-body")).toHaveTextContent("Full body text here");
  });

  it("bulk delete removes selected rows (AC-4)", async () => {
    let deletedIds: number[] | undefined;
    server.use(
      http.get("/api/staff/admin/logs", () =>
        HttpResponse.json(listPayload(deletedIds ? [ROWS[1]] : ROWS)),
      ),
      http.post("/api/staff/admin/logs/delete", async ({ request }) => {
        deletedIds = ((await request.json()) as { ids: number[] }).ids;
        return HttpResponse.json({ affected: deletedIds.length });
      }),
    );
    const user = userEvent.setup();
    renderWithProviders(<LogViewerPage />, { store: authedStore() });
    await screen.findByText("Login failure");
    await user.click(within(screen.getByTestId("log-check-1")).getByRole("checkbox"));
    await user.click(screen.getByTestId("log-bulk-delete"));
    await user.click(await screen.findByTestId("log-bulk-delete-confirm"));
    await waitFor(() => expect(deletedIds).toEqual([1]));
    await waitFor(() => expect(screen.queryByText("Login failure")).not.toBeInTheDocument());
  });

  it("Purge now triggers the sweep (AC-5)", async () => {
    let purged = false;
    server.use(
      http.get("/api/staff/admin/logs", () => HttpResponse.json(listPayload(purged ? [ROWS[1]] : ROWS))),
      http.post("/api/dev/purge-logs", () => {
        purged = true;
        return HttpResponse.json({ affected: 1 });
      }),
    );
    const user = userEvent.setup();
    renderWithProviders(<LogViewerPage />, { store: authedStore() });
    await screen.findByText("Login failure");
    await user.click(screen.getByTestId("logs-purge"));
    await user.click(await screen.findByTestId("log-purge-confirm"));
    await waitFor(() => expect(purged).toBe(true));
  });
});
