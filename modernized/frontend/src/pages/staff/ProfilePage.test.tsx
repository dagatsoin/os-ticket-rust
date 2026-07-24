import { describe, expect, it } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithProviders } from "../../test/renderWithProviders";
import { server, http, HttpResponse } from "../../test/mockServer";
import { RootStore } from "../../stores/RootStore";
import { ProfilePage } from "./ProfilePage";

/** TS-M4-B6 UI: username read-only, forced-change banner, password change flow. */

const PROFILE = {
  id: 1, username: "agent", firstname: "Ada", lastname: "Lovelace", email: "agent@x.io",
  phone: "5551234", phone_ext: "12", mobile: "5559999", signature: "",
  timezone_id: 1, daylight_saving: false, max_page_size: 25, auto_refresh_rate: 0,
  default_signature_type: "none", default_paper_size: "Letter", change_passwd: false, onvacation: false, dept_id: 2,
};

function authedStore() {
  const store = new RootStore({ onUnauthorized: () => {} });
  store.staffAuth.user = { id: 1, username: "agent", isadmin: false };
  store.staffAuth.profileLoaded = "loaded";
  return store;
}

describe("ProfilePage (TS-M4-B6)", () => {
  it("renders with a read-only (disabled) username field", async () => {
    server.use(http.get("/api/staff/profile", () => HttpResponse.json(PROFILE)));
    renderWithProviders(<ProfilePage />, { store: authedStore() });
    const username = (await screen.findByTestId("profile-username")) as HTMLInputElement;
    expect(username).toBeDisabled();
    expect(username.value).toBe("agent");
  });

  it("shows the forced-password-change banner when flagged", async () => {
    server.use(http.get("/api/staff/profile", () => HttpResponse.json({ ...PROFILE, change_passwd: true })));
    renderWithProviders(<ProfilePage />, { store: authedStore() });
    expect(await screen.findByTestId("profile-forced-banner")).toHaveTextContent(
      "You must change your password to continue!",
    );
  });

  it("changes the password end to end (success)", async () => {
    let body: Record<string, unknown> | undefined;
    server.use(
      http.get("/api/staff/profile", () => HttpResponse.json(PROFILE)),
      http.put("/api/staff/profile/password", async ({ request }) => {
        body = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json({ ok: true });
      }),
    );
    const user = userEvent.setup();
    const { store } = renderWithProviders(<ProfilePage />, { store: authedStore() });
    await screen.findByTestId("profile-username");
    await user.type(screen.getByTestId("pw-current"), "Agent123!");
    await user.type(screen.getByTestId("pw-new"), "Agent999!");
    await user.type(screen.getByTestId("pw-confirm"), "Agent999!");
    await user.click(screen.getByTestId("pw-save"));
    await waitFor(() => expect(body).toEqual({ current: "Agent123!", new: "Agent999!", confirm: "Agent999!" }));
    await waitFor(() => expect(store.snackbar.message).toBe("Password changed successfully"));
  });

  it("shows an inline error when the new password is too short", async () => {
    server.use(http.get("/api/staff/profile", () => HttpResponse.json(PROFILE)));
    const user = userEvent.setup();
    renderWithProviders(<ProfilePage />, { store: authedStore() });
    await screen.findByTestId("profile-username");
    await user.type(screen.getByTestId("pw-new"), "abc");
    await user.click(screen.getByTestId("pw-save"));
    expect(await screen.findByText("Must be at least 6 characters")).toBeInTheDocument();
  });
});
