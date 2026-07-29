import { describe, expect, it, vi } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Route, Routes } from "react-router-dom";
import { renderWithProviders } from "../test/renderWithProviders";
import { server, http, HttpResponse, readMultipart } from "../test/mockServer";
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
            { id: 10, threadType: "M", created: "2026-07-23T14:05:09Z", poster: "Alice", body: "Cannot log in" },
            { id: 11, threadType: "N", created: "2026-07-23T14:05:09Z", poster: "Agent", body: "checking SSO" },
          ],
        }),
      ),
      http.get("/api/staff/tickets/1/canned", () => HttpResponse.json([])),
      http.post("/api/staff/tickets/1/reply", () =>
        HttpResponse.json({
          id: 1, number: 100001, subject: "Login issue", email: "a@x.io", name: "Alice",
          status: "open", created: "2026-06-01T10:00:00Z",
          entries: [
            { id: 10, threadType: "M", created: "2026-07-23T14:05:09Z", poster: "Alice", body: "Cannot log in" },
            { id: 11, threadType: "N", created: "2026-07-23T14:05:09Z", poster: "Agent", body: "checking SSO" },
            { id: 12, threadType: "R", created: "2026-07-23T14:05:09Z", poster: "Agent", body: "Try a reset" },
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
    // Each entry renders its per-entry `created` timestamp (formatted from the
    // RFC3339 value the backend now sends); the fixture year appears on each.
    for (const entry of screen.getAllByTestId("thread-entry")) {
      expect(entry).toHaveTextContent("2026");
    }

    await user.type(screen.getByLabelText(/reply/i), "Try a reset");
    await user.click(screen.getByRole("button", { name: /send reply/i }));

    // The new R entry appears, sourced from the reply response.
    expect(await screen.findByText("Try a reset")).toBeInTheDocument();
    expect(screen.getAllByTestId("thread-entry")).toHaveLength(3);
  });

  // --- TS-M2-B2: clickable chips + answered badge in staff detail ---

  it("B2: a thread entry with an attachment renders a clickable chip that downloads via the staff route", async () => {
    let hitUrl: string | null = null;
    server.use(
      http.get("/api/staff/tickets/1", () =>
        HttpResponse.json({
          id: 1, number: 100001, subject: "Login issue", email: "a@x.io", name: "Alice",
          status: "open", created: "2026-06-01T10:00:00Z", isanswered: false,
          entries: [
            {
              id: 10, threadType: "M", created: "2026-07-23T14:05:09Z", poster: "Alice", body: "Cannot log in",
              attachments: [{ id: 99, name: "invoice.pdf", size: 100, mime: "application/pdf" }],
            },
          ],
        }),
      ),
      http.get("/api/staff/tickets/1/canned", () => HttpResponse.json([])),
      http.get("/api/staff/tickets/1/attachments/99", ({ request }) => {
        hitUrl = new URL(request.url).pathname;
        return new HttpResponse("pdf-bytes", { status: 200 });
      }),
    );
    // Suppress the synthetic anchor click's jsdom navigation no-op.
    const clickSpy = vi
      .spyOn(HTMLAnchorElement.prototype, "click")
      .mockImplementation(() => {});

    renderWithProviders(<StaffRoutes />, { route: "/staff/tickets/1" });

    const chip = await screen.findByTestId("attachment-chip");
    expect(chip).toHaveTextContent("invoice.pdf");
    // The detail header shows the Unanswered badge.
    expect(screen.getByText(/unanswered/i)).toBeInTheDocument();

    await userEvent.click(chip);
    await waitFor(() => expect(hitUrl).toBe("/api/staff/tickets/1/attachments/99"));
    clickSpy.mockRestore();
  });

  // --- TS-M2-D3: reply composer (canned dropdown + own-file input) ---

  function detailHandler() {
    return http.get("/api/staff/tickets/1", () =>
      HttpResponse.json({
        id: 1, number: 100001, subject: "Login issue", email: "a@x.io", name: "Alice",
        status: "open", created: "2026-06-01T10:00:00Z", isanswered: false,
        entries: [{ id: 10, threadType: "M", created: "2026-07-23T14:05:09Z", poster: "Alice", body: "Cannot log in" }],
      }),
    );
  }

  it("D3-1/2/3/4: canned dropdown lists responses, fills the textarea (substituted), carries a read-only chip, and has an own-file input", async () => {
    server.use(
      detailHandler(),
      http.get("/api/staff/tickets/1/canned", () =>
        HttpResponse.json([{ id: 7, title: "Acknowledge receipt" }]),
      ),
      http.get("/api/staff/tickets/1/canned/7", () =>
        HttpResponse.json({
          body: "Hi, we received ticket 100001.",
          attachments: [{ id: 50, name: "policy.txt", size: 12, mime: "text/plain" }],
        }),
      ),
    );
    renderWithProviders(<StaffRoutes />, { route: "/staff/tickets/1" });

    // D3-4: the own-file input is present.
    expect(await screen.findByTestId("reply-file-input")).toBeInTheDocument();

    // D3-1: open the dropdown; the enabled response is listed.
    await userEvent.click(screen.getByLabelText(/canned response/i));
    const option = await screen.findByRole("option", { name: "Acknowledge receipt" });
    await userEvent.click(option);

    // D3-2: the textarea is filled with the substituted body (no literal %{...}).
    await waitFor(() => {
      const textarea = screen.getByTestId("reply-textarea") as HTMLTextAreaElement;
      expect(textarea.value).toBe("Hi, we received ticket 100001.");
    });
    expect((screen.getByTestId("reply-textarea") as HTMLTextAreaElement).value).not.toMatch(/%\{/);

    // D3-3: a read-only carried chip appears, marked "from canned response".
    expect(screen.getByLabelText("policy.txt (from canned response)")).toBeInTheDocument();
  });

  it("D3: selecting a canned response then sending posts body + cannedId as multipart", async () => {
    let captured: Awaited<ReturnType<typeof readMultipart>> | null = null;
    server.use(
      detailHandler(),
      http.get("/api/staff/tickets/1/canned", () =>
        HttpResponse.json([{ id: 7, title: "Acknowledge receipt" }]),
      ),
      http.get("/api/staff/tickets/1/canned/7", () =>
        HttpResponse.json({ body: "Substituted body 100001.", attachments: [] }),
      ),
      http.post("/api/staff/tickets/1/reply", async ({ request }) => {
        captured = await readMultipart(request);
        return HttpResponse.json({
          id: 1, number: 100001, subject: "Login issue", email: "a@x.io", name: "Alice",
          status: "open", created: "2026-06-01T10:00:00Z", isanswered: true,
          entries: [
            { id: 10, threadType: "M", created: "2026-07-23T14:05:09Z", poster: "Alice", body: "Cannot log in" },
            { id: 13, threadType: "R", created: "2026-07-23T14:05:09Z", poster: "Agent", body: "Substituted body 100001." },
          ],
        });
      }),
    );
    renderWithProviders(<StaffRoutes />, { route: "/staff/tickets/1" });

    await userEvent.click(await screen.findByLabelText(/canned response/i));
    await userEvent.click(await screen.findByRole("option", { name: "Acknowledge receipt" }));
    await waitFor(() =>
      expect((screen.getByTestId("reply-textarea") as HTMLTextAreaElement).value).toBe(
        "Substituted body 100001.",
      ),
    );

    await userEvent.click(screen.getByRole("button", { name: /send reply/i }));

    await waitFor(() => expect(captured).not.toBeNull());
    expect(captured!.fields.body).toBe("Substituted body 100001.");
    expect(captured!.fields.cannedId).toBe("7");
    // After the post the new R entry is shown (refreshed from the response).
    expect(await screen.findByText("Substituted body 100001.")).toBeInTheDocument();
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
