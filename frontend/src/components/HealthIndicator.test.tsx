import { describe, expect, it } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import { server, http, HttpResponse } from "../test/mockServer";
import { renderWithProviders } from "../test/renderWithProviders";
import { HealthIndicator } from "./HealthIndicator";

/** AC-6: the shell health indicator reflects /api/health ok / down (MSW harness). */
describe("HealthIndicator", () => {
  it("shows 'Backend OK' when /api/health is healthy", async () => {
    server.use(
      http.get("/api/health", () => HttpResponse.json({ status: "ok", db: "up" })),
    );

    renderWithProviders(<HealthIndicator />);

    await waitFor(() => {
      expect(screen.getByTestId("health-indicator")).toHaveAttribute("data-status", "ok");
    });
    expect(screen.getByText("Backend OK")).toBeInTheDocument();
  });

  it("shows 'Backend down' when /api/health fails", async () => {
    server.use(
      http.get("/api/health", () =>
        HttpResponse.json({ error: { message: "db down" } }, { status: 503 }),
      ),
    );

    renderWithProviders(<HealthIndicator />);

    await waitFor(() => {
      expect(screen.getByTestId("health-indicator")).toHaveAttribute("data-status", "down");
    });
    expect(screen.getByText("Backend down")).toBeInTheDocument();
  });
});
