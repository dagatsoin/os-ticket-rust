import { describe, expect, it } from "vitest";
import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Route, Routes } from "react-router-dom";
import { renderWithProviders } from "../test/renderWithProviders";
import { server, http, HttpResponse } from "../test/mockServer";
import { RootStore } from "../stores/RootStore";
import { ClientLoginPage, ClientTicketsPage } from "./ClientPortal";

function ClientRoutes() {
  return (
    <Routes>
      <Route path="/tickets/login" element={<ClientLoginPage />} />
      <Route path="/tickets" element={<ClientTicketsPage />} />
    </Routes>
  );
}

describe("Client portal (TS-M1-D2)", () => {
  it("AC-1: login posts ticket#/email; success routes to the thread view", async () => {
    server.use(
      http.post("/api/client/login", () => HttpResponse.json({ ok: true, csrfToken: "c" })),
      http.get("/api/client/ticket", () =>
        HttpResponse.json({
          number: 123456, subject: "Login issue", status: "open",
          created: "2026-06-01T10:00:00Z",
          entries: [{ id: 10, threadType: "M", poster: "Alice", body: "I cannot log in" }],
        }),
      ),
    );
    const user = userEvent.setup();
    renderWithProviders(<ClientRoutes />, { route: "/tickets/login" });

    await user.type(screen.getByLabelText(/ticket number/i), "123456");
    await user.type(screen.getByLabelText(/email/i), "alice@example.com");
    await user.click(screen.getByRole("button", { name: /view ticket/i }));

    expect(await screen.findByText("I cannot log in")).toBeInTheDocument();
    expect(screen.getByText(/ticket #123456/i)).toBeInTheDocument();
  });

  it("AC-1: login failure shows a sensible generic error and stays on login", async () => {
    server.use(
      http.post("/api/client/login", () =>
        HttpResponse.json(
          { error: { message: "Authentication error - try again!" } },
          { status: 401 },
        ),
      ),
    );
    const user = userEvent.setup();
    // Router-aware redirect so a 401 doesn't reach jsdom's window.location.
    const store = new RootStore({ onUnauthorized: () => {} });
    renderWithProviders(<ClientRoutes />, { route: "/tickets/login", store });

    await user.type(screen.getByLabelText(/ticket number/i), "000000");
    await user.type(screen.getByLabelText(/email/i), "nobody@example.com");
    await user.click(screen.getByRole("button", { name: /view ticket/i }));

    expect(await screen.findByText(/authentication error - try again/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/ticket number/i)).toBeInTheDocument();
  });

  it("AC-2: thread renders M/R oldest-first and is read-only (no reply box)", async () => {
    const store = new RootStore();
    store.clientPortal.ticket = {
      number: 123456, subject: "Login issue", status: "open",
      created: "2026-06-01T10:00:00Z",
      entries: [
        { id: 10, threadType: "M", poster: "Alice", body: "I cannot log in" },
        { id: 12, threadType: "R", poster: "Agent", body: "Try a reset" },
      ],
    };
    renderWithProviders(<ClientRoutes />, { route: "/tickets", store });

    const entries = await screen.findAllByTestId("thread-entry");
    expect(entries).toHaveLength(2);
    // Oldest-first: the customer message precedes the staff response.
    expect(entries[0]).toHaveTextContent("I cannot log in");
    expect(entries[1]).toHaveTextContent("Try a reset");
    // Read-only: no reply composer / slot.
    expect(screen.queryByTestId("thread-reply-slot")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /reply/i })).not.toBeInTheDocument();
  });

  it("AC-3: an internal note (N) is never rendered, even if present in the data", async () => {
    const store = new RootStore();
    // Inject an N entry to prove the view never surfaces it (defence in depth;
    // the route already excludes N).
    store.clientPortal.ticket = {
      number: 123456, subject: "Login issue", status: "open",
      created: "2026-06-01T10:00:00Z",
      entries: [
        { id: 10, threadType: "M", poster: "Alice", body: "I cannot log in" },
        // eslint-disable-next-line @typescript-eslint/no-explicit-any -- forcing an N to prove it is filtered
        { id: 11, threadType: "N" as any, poster: "Agent", body: "SECRET internal note" },
        { id: 12, threadType: "R", poster: "Agent", body: "Try a reset" },
      ],
    };
    renderWithProviders(<ClientRoutes />, { route: "/tickets", store });

    await screen.findByText("I cannot log in");
    expect(screen.getByText("Try a reset")).toBeInTheDocument();
    // The N note body must NOT appear anywhere.
    expect(screen.queryByText("SECRET internal note")).not.toBeInTheDocument();
    expect(screen.getAllByTestId("thread-entry")).toHaveLength(2);
  });
});
