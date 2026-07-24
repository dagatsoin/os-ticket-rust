import { describe, expect, it } from "vitest";
import { screen } from "@testing-library/react";
import { renderWithProviders } from "../test/renderWithProviders";
import { server, http, HttpResponse } from "../test/mockServer";
import { RootStore } from "../stores/RootStore";
import { AppRoutes } from "./router";

/**
 * AC-3 (unit level): the single router resolves all three branches to their placeholder
 * views. The [BROWSER] AC-3 is validated separately by qa-criterion-tester against the
 * running dev server; this keeps the contract green in CI.
 */
describe("AppRoutes — three branches", () => {
  it("resolves the public home at /", () => {
    renderWithProviders(<AppRoutes />, { route: "/" });
    expect(screen.getByText("Support Center")).toBeInTheDocument();
  });

  it("resolves the public open-ticket page at /open", () => {
    renderWithProviders(<AppRoutes />, { route: "/open" });
    expect(screen.getByText("Open a New Ticket")).toBeInTheDocument();
  });

  it("resolves the staff branch at /staff/login", () => {
    renderWithProviders(<AppRoutes />, { route: "/staff/login" });
    expect(screen.getByText("Staff Sign In")).toBeInTheDocument();
  });

  it("redirects the /staff index to the staff login when unauthenticated", () => {
    renderWithProviders(<AppRoutes />, { route: "/staff" });
    expect(screen.getByText("Staff Sign In")).toBeInTheDocument();
  });

  it("redirects /tickets to the client login when there is no session", async () => {
    server.use(
      http.get("/api/client/ticket", () =>
        HttpResponse.json({ error: { message: "unauthenticated" } }, { status: 401 }),
      ),
    );
    const store = new RootStore({ onUnauthorized: () => {} });
    renderWithProviders(<AppRoutes />, { route: "/tickets", store });
    expect(await screen.findByText("View Your Ticket")).toBeInTheDocument();
  });

  it("resolves the client login at /tickets/login", () => {
    renderWithProviders(<AppRoutes />, { route: "/tickets/login" });
    expect(screen.getByText("View Your Ticket")).toBeInTheDocument();
  });

  it("renders the not-found page for an unknown route (no blank screen)", () => {
    renderWithProviders(<AppRoutes />, { route: "/does-not-exist" });
    expect(screen.getByText("Page not found")).toBeInTheDocument();
  });
});
