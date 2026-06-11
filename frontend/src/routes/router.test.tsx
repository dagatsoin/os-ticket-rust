import { describe, expect, it } from "vitest";
import { screen } from "@testing-library/react";
import { renderWithProviders } from "../test/renderWithProviders";
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

  it("resolves the staff dashboard at /staff", () => {
    renderWithProviders(<AppRoutes />, { route: "/staff" });
    expect(screen.getByText("Staff Dashboard")).toBeInTheDocument();
  });

  it("resolves the client portal branch at /tickets", () => {
    renderWithProviders(<AppRoutes />, { route: "/tickets" });
    expect(screen.getByText("My Tickets")).toBeInTheDocument();
  });

  it("resolves the client login at /tickets/login", () => {
    renderWithProviders(<AppRoutes />, { route: "/tickets/login" });
    expect(screen.getByText("Client Sign In")).toBeInTheDocument();
  });

  it("renders the not-found page for an unknown route (no blank screen)", () => {
    renderWithProviders(<AppRoutes />, { route: "/does-not-exist" });
    expect(screen.getByText("Page not found")).toBeInTheDocument();
  });
});
