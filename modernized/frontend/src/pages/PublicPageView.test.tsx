import { describe, expect, it } from "vitest";
import { screen } from "@testing-library/react";
import { Route, Routes } from "react-router-dom";
import { renderWithProviders } from "../test/renderWithProviders";
import { server, http, HttpResponse } from "../test/mockServer";
import { PublicPageView } from "./PublicPageView";

function PageRoutes() {
  return (
    <Routes>
      <Route path="/pages/:slug" element={<PublicPageView />} />
    </Routes>
  );
}

describe("PublicPageView (/pages/:slug)", () => {
  it("renders the page name as a title and the HTML body", async () => {
    server.use(
      http.get("/api/pages/terms", () =>
        HttpResponse.json({
          name: "Terms Of Service",
          body: "<p>Please be <strong>nice</strong>.</p>",
        }),
      ),
    );
    renderWithProviders(<PageRoutes />, { route: "/pages/terms" });

    expect(
      await screen.findByRole("heading", { name: "Terms Of Service" }),
    ).toBeInTheDocument();
    const body = screen.getByTestId("page-body");
    expect(body).toHaveTextContent("Please be nice.");
    // Body is rendered as HTML (trusted author), not escaped text.
    expect(body.querySelector("strong")).not.toBeNull();
  });

  it("shows a not-found state on a 404", async () => {
    server.use(
      http.get("/api/pages/missing", () =>
        HttpResponse.json({ error: { message: "Page not found" } }, { status: 404 }),
      ),
    );
    renderWithProviders(<PageRoutes />, { route: "/pages/missing" });

    expect(await screen.findByTestId("page-not-found")).toBeInTheDocument();
  });
});
