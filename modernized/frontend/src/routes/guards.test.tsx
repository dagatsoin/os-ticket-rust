import { describe, expect, it } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import { Route, Routes } from "react-router-dom";
import { renderWithProviders } from "../test/renderWithProviders";
import { server, http, HttpResponse } from "../test/mockServer";
import { RootStore } from "../stores/RootStore";
import { RequireAdmin, RequireAdminArea, RequireCapability, RequireStaff } from "./guards";

/**
 * TS-M4-A0 route guards. All three share the loading-spinner / login / denial gate;
 * these tests exercise each branch against a mocked GET /api/staff/me.
 */

function meOnce(profile: Record<string, unknown> | null) {
  return http.get("/api/staff/me", () =>
    profile === null
      ? HttpResponse.json({ error: { message: "no session" } }, { status: 401 })
      : HttpResponse.json(profile),
  );
}

function GuardRoutes({ children }: { children: React.ReactNode }) {
  return (
    <Routes>
      <Route path="/staff/login" element={<div>LOGIN PAGE</div>} />
      <Route path="/staff/tickets" element={<div>TICKETS PAGE</div>} />
      <Route path="/staff/admin" element={children} />
    </Routes>
  );
}

function renderGuard(children: React.ReactNode) {
  const store = new RootStore({ onUnauthorized: () => {} });
  // renderWithProviders returns the same store on its result.
  return renderWithProviders(<GuardRoutes>{children}</GuardRoutes>, {
    route: "/staff/admin",
    store,
  });
}

describe("RequireAdmin (TS-M4-A0)", () => {
  it("shows a spinner while the profile is still loading (no redirect flash)", () => {
    server.use(meOnce({ id: 1, isadmin: true }));
    renderGuard(
      <RequireAdmin>
        <div>ADMIN CONTENT</div>
      </RequireAdmin>,
    );
    // Synchronously after render the profile fetch has not resolved yet.
    expect(screen.getByTestId("admin-gate-spinner")).toBeInTheDocument();
  });

  it("renders the wrapped screen for an admin", async () => {
    server.use(meOnce({ id: 1, username: "admin", isadmin: true }));
    renderGuard(
      <RequireAdmin>
        <div>ADMIN CONTENT</div>
      </RequireAdmin>,
    );
    expect(await screen.findByText("ADMIN CONTENT")).toBeInTheDocument();
  });

  it("redirects an authenticated non-admin to /staff/tickets with a denial notice", async () => {
    server.use(meOnce({ id: 2, username: "agent", isadmin: false }));
    const { store } = renderGuard(
      <RequireAdmin>
        <div>ADMIN CONTENT</div>
      </RequireAdmin>,
    );
    expect(await screen.findByText("TICKETS PAGE")).toBeInTheDocument();
    expect(screen.queryByText("ADMIN CONTENT")).not.toBeInTheDocument();
    await waitFor(() => expect(store.snackbar.open).toBe(true));
    expect(store.snackbar.message).toMatch(/permission/i);
  });

  it("redirects an unauthenticated visitor to the staff login", async () => {
    server.use(meOnce(null));
    renderGuard(
      <RequireAdmin>
        <div>ADMIN CONTENT</div>
      </RequireAdmin>,
    );
    expect(await screen.findByText("LOGIN PAGE")).toBeInTheDocument();
  });
});

describe("RequireCapability (TS-M4-A0)", () => {
  it("admits a non-admin who holds the required flag", async () => {
    server.use(meOnce({ id: 3, isadmin: false, can_manage_faq: true }));
    renderGuard(
      <RequireCapability flag="can_manage_faq">
        <div>FAQ SCREEN</div>
      </RequireCapability>,
    );
    expect(await screen.findByText("FAQ SCREEN")).toBeInTheDocument();
  });

  it("denies a non-admin lacking the flag", async () => {
    server.use(meOnce({ id: 4, isadmin: false, can_manage_premade: true }));
    renderGuard(
      <RequireCapability flag="can_manage_faq">
        <div>FAQ SCREEN</div>
      </RequireCapability>,
    );
    expect(await screen.findByText("TICKETS PAGE")).toBeInTheDocument();
  });
});

describe("RequireStaff (TS-M4-B6)", () => {
  it("admits a plain authenticated agent (no capability required)", async () => {
    server.use(meOnce({ id: 7, username: "agent", isadmin: false }));
    renderGuard(
      <RequireStaff>
        <div>PROFILE SCREEN</div>
      </RequireStaff>,
    );
    expect(await screen.findByText("PROFILE SCREEN")).toBeInTheDocument();
  });

  it("redirects an unauthenticated visitor to the staff login", async () => {
    server.use(meOnce(null));
    renderGuard(
      <RequireStaff>
        <div>PROFILE SCREEN</div>
      </RequireStaff>,
    );
    expect(await screen.findByText("LOGIN PAGE")).toBeInTheDocument();
  });
});

describe("RequireAdminArea (TS-M4-A0)", () => {
  it("admits a delegated non-admin (any capability) into the shell", async () => {
    server.use(meOnce({ id: 5, isadmin: false, can_manage_faq: true }));
    renderGuard(
      <RequireAdminArea>
        <div>ADMIN SHELL</div>
      </RequireAdminArea>,
    );
    expect(await screen.findByText("ADMIN SHELL")).toBeInTheDocument();
  });

  it("denies a plain agent with no capabilities", async () => {
    server.use(meOnce({ id: 6, isadmin: false }));
    renderGuard(
      <RequireAdminArea>
        <div>ADMIN SHELL</div>
      </RequireAdminArea>,
    );
    expect(await screen.findByText("TICKETS PAGE")).toBeInTheDocument();
    expect(screen.queryByText("ADMIN SHELL")).not.toBeInTheDocument();
  });
});
