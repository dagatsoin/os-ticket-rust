import { describe, expect, it } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Route, Routes } from "react-router-dom";
import { renderWithProviders } from "../test/renderWithProviders";
import { server, http, HttpResponse } from "../test/mockServer";
import { RootStore } from "../stores/RootStore";
import { StaffLoginPage, StaffQueuePage, StaffTicketDetailPage } from "./StaffArea";

/** A minimal staff router used to assert navigation between the staff screens. */
function StaffRoutes() {
  return (
    <Routes>
      <Route path="/staff/login" element={<StaffLoginPage />} />
      <Route path="/staff/tickets" element={<StaffQueuePage />} />
      <Route path="/staff/tickets/:id" element={<StaffTicketDetailPage />} />
    </Routes>
  );
}

const ME = { id: 5, username: "agent", name: "Agent Smith", deptId: 1 };

describe("Staff UI (TS-M1-C4)", () => {
  it("AC-1: login posts credentials, success routes to the queue", async () => {
    server.use(
      http.post("/api/staff/login", () => HttpResponse.json({ ok: true, csrfToken: "c" })),
      http.get("/api/staff/me", () => HttpResponse.json(ME)),
      http.get("/api/staff/tickets", () => HttpResponse.json([])),
    );
    const user = userEvent.setup();
    renderWithProviders(<StaffRoutes />, { route: "/staff/login" });

    await user.type(screen.getByLabelText(/username/i), "agent");
    await user.type(screen.getByLabelText(/password/i), "Agent123!");
    await user.click(screen.getByRole("button", { name: /sign in/i }));

    expect(await screen.findByText(/open tickets/i)).toBeInTheDocument();
  });

  it("AC-1: login failure shows a generic error and stays on the login page", async () => {
    server.use(
      http.post("/api/staff/login", () =>
        HttpResponse.json(
          { error: { message: "Invalid username or password" } },
          { status: 401 },
        ),
      ),
    );
    const user = userEvent.setup();
    // Router-aware redirect hook so a 401 doesn't hit jsdom's window.location.
    const store = new RootStore({ onUnauthorized: () => {} });
    renderWithProviders(<StaffRoutes />, { route: "/staff/login", store });

    await user.type(screen.getByLabelText(/username/i), "agent");
    await user.type(screen.getByLabelText(/password/i), "wrong");
    await user.click(screen.getByRole("button", { name: /sign in/i }));

    expect(await screen.findByText(/invalid username or password/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/username/i)).toBeInTheDocument();
  });

  it("AC-2: queue renders number + subject + email rows from the list route", async () => {
    server.use(
      http.get("/api/staff/tickets", () =>
        HttpResponse.json([
          { id: 2, number: 200002, subject: "Printer down", email: "b@x.io", created: "2026-06-02T10:00:00Z" },
          { id: 1, number: 100001, subject: "Login issue", email: "a@x.io", created: "2026-06-01T10:00:00Z" },
        ]),
      ),
    );
    renderWithProviders(<StaffRoutes />, { route: "/staff/tickets" });

    expect(await screen.findByText("Printer down")).toBeInTheDocument();
    expect(screen.getByText("Login issue")).toBeInTheDocument();
    expect(screen.getByText("200002")).toBeInTheDocument();
    expect(screen.getByText("a@x.io")).toBeInTheDocument();
  });

  it("AC-3: detail renders thread in order; reply refetches from the reply response", async () => {
    server.use(
      http.get("/api/staff/tickets/1", () =>
        HttpResponse.json({
          id: 1, number: 100001, subject: "Login issue", email: "a@x.io", name: "Alice",
          status: "open", created: "2026-06-01T10:00:00Z",
          entries: [
            { id: 10, threadType: "M", poster: "Alice", body: "Cannot log in" },
            { id: 11, threadType: "N", poster: "Agent", body: "checking SSO" },
          ],
        }),
      ),
      http.post("/api/staff/tickets/1/reply", () =>
        HttpResponse.json({
          id: 1, number: 100001, subject: "Login issue", email: "a@x.io", name: "Alice",
          status: "open", created: "2026-06-01T10:00:00Z",
          entries: [
            { id: 10, threadType: "M", poster: "Alice", body: "Cannot log in" },
            { id: 11, threadType: "N", poster: "Agent", body: "checking SSO" },
            { id: 12, threadType: "R", poster: "Agent", body: "Try a reset" },
          ],
        }),
      ),
    );
    const user = userEvent.setup();
    renderWithProviders(<StaffRoutes />, { route: "/staff/tickets/1" });

    // Initial thread (2 entries, in order), staff see the internal N note.
    await waitFor(() => expect(screen.getByText("Cannot log in")).toBeInTheDocument());
    expect(screen.getByText("checking SSO")).toBeInTheDocument();
    expect(screen.queryByText("Try a reset")).not.toBeInTheDocument();

    await user.type(screen.getByLabelText(/reply/i), "Try a reset");
    await user.click(screen.getByRole("button", { name: /send reply/i }));

    // The new R entry appears, sourced from the reply response.
    expect(await screen.findByText("Try a reset")).toBeInTheDocument();
    expect(screen.getAllByTestId("thread-entry")).toHaveLength(3);
  });

  it("AC-4: a 401 from the queue route redirects to /staff/login", async () => {
    server.use(
      http.get("/api/staff/tickets", () =>
        HttpResponse.json({ error: { message: "unauthenticated" } }, { status: 401 }),
      ),
    );
    // Capture the redirect via the apiClient's onUnauthorized hook.
    const redirects: string[] = [];
    const guarded = new RootStore({ onUnauthorized: (p) => redirects.push(p) });
    renderWithProviders(<StaffRoutes />, { route: "/staff/tickets", store: guarded });

    await waitFor(() => expect(redirects).toContain("/staff/login"));
  });
});
