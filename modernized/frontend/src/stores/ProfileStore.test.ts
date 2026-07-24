import { beforeEach, describe, expect, it } from "vitest";
import { server, http, HttpResponse } from "../test/mockServer";
import { RootStore } from "./RootStore";

/**
 * TS-M4-B6 business logic (TDD): snake→camel load mapping, camelCase update
 * payload, ordered password validation + 422 mapping, forced-change / vacation
 * banner derivation.
 */

const PROFILE = {
  id: 1,
  username: "agent",
  firstname: "Ada",
  lastname: "Lovelace",
  email: "agent@osticket.local",
  phone: "5551234",
  phone_ext: "12",
  mobile: "5559999",
  signature: "Regards",
  timezone_id: 1,
  daylight_saving: false,
  max_page_size: 25,
  auto_refresh_rate: 0,
  default_signature_type: "none",
  default_paper_size: "Letter",
  change_passwd: false,
  onvacation: false,
  dept_id: 2,
};

function mockGet(over: Record<string, unknown> = {}) {
  return http.get("/api/staff/profile", () => HttpResponse.json({ ...PROFILE, ...over }));
}

describe("ProfileStore (TS-M4-B6)", () => {
  let root: RootStore;
  beforeEach(() => {
    root = new RootStore();
  });

  it("loads and maps snake_case wire fields → camelCase form values", async () => {
    server.use(mockGet());
    await root.profile.load();
    expect(root.profile.form).toMatchObject({
      firstname: "Ada",
      phoneExt: "12",
      timezoneId: "1",
      maxPageSize: "25",
      defaultPaperSize: "Letter",
    });
    expect(root.profile.username).toBe("agent");
  });

  it("derives the forced-change + vacation banner flags", async () => {
    server.use(mockGet({ change_passwd: true, onvacation: true }));
    await root.profile.load();
    expect(root.profile.mustChangePassword).toBe(true);
    expect(root.profile.onVacation).toBe(true);
  });

  it("update sends a camelCase body and refreshes from the echo", async () => {
    let body: Record<string, unknown> | undefined;
    server.use(
      mockGet(),
      http.put("/api/staff/profile", async ({ request }) => {
        body = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json({ ...PROFILE, timezone_id: 5, max_page_size: 50 });
      }),
    );
    await root.profile.load();
    root.profile.setField("timezoneId", "5");
    root.profile.setField("maxPageSize", "50");
    const ok = await root.profile.save();
    expect(ok).toBe(true);
    expect(body).toMatchObject({ timezoneId: 5, maxPageSize: 50 });
    expect(root.profile.form?.timezoneId).toBe("5");
    expect(root.snackbar.message).toBe("Profile updated successfully");
  });

  it("password: ordered client validation stops before a request", async () => {
    server.use(mockGet());
    await root.profile.load();
    const p = root.profile;
    // blank new
    expect((await p.changePassword())).toBe(false);
    expect(p.passwordError("new")).toBe("New password required");
    // too short
    p.setPasswordField("new", "abc");
    await p.changePassword();
    expect(p.passwordError("new")).toBe("Must be at least 6 characters");
    // mismatch
    p.setPasswordField("new", "Agent999!");
    p.setPasswordField("confirm", "nope");
    await p.changePassword();
    expect(p.passwordError("confirm")).toBe("Password(s) do not match");
    // current required
    p.setPasswordField("confirm", "Agent999!");
    await p.changePassword();
    expect(p.passwordError("current")).toBe("Current password required");
  });

  it("password: server 422 (wrong current) maps to an inline error", async () => {
    server.use(
      mockGet(),
      http.put("/api/staff/profile/password", () =>
        HttpResponse.json({ error: { message: "Invalid current password!", fields: { current: "Invalid current password!" } } }, { status: 422 }),
      ),
    );
    await root.profile.load();
    const p = root.profile;
    p.setPasswordField("current", "wrong");
    p.setPasswordField("new", "Agent999!");
    p.setPasswordField("confirm", "Agent999!");
    const ok = await p.changePassword();
    expect(ok).toBe(false);
    expect(p.passwordError("current")).toBe("Invalid current password!");
  });

  it("password: success clears the forced-change flag", async () => {
    server.use(
      mockGet({ change_passwd: true }),
      http.put("/api/staff/profile/password", () => HttpResponse.json({ ok: true })),
    );
    await root.profile.load();
    expect(root.profile.mustChangePassword).toBe(true);
    const p = root.profile;
    p.setPasswordField("current", "Agent123!");
    p.setPasswordField("new", "Agent999!");
    p.setPasswordField("confirm", "Agent999!");
    const ok = await p.changePassword();
    expect(ok).toBe(true);
    expect(root.profile.mustChangePassword).toBe(false);
  });
});
