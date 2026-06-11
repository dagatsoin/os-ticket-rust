import { describe, expect, it } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithProviders } from "../test/renderWithProviders";
import { server, http, HttpResponse } from "../test/mockServer";
import { OpenTicketPage } from "./OpenTicketPage";

/**
 * TS-M1-B3 component tests (AC-1/AC-2/AC-3). The store is unit-tested separately
 * (OpenTicketStore.test.ts, AC-4); these assert the rendered form wiring.
 */
describe("OpenTicketPage", () => {
  it("AC-1: shows validation errors for empty required fields / invalid email and gates submit", async () => {
    const user = userEvent.setup();
    renderWithProviders(<OpenTicketPage />);

    const submit = screen.getByRole("button", { name: /submit ticket/i });
    expect(submit).toBeDisabled();

    // Enter an invalid email -> inline error appears.
    await user.type(screen.getByLabelText(/email/i), "nope");
    expect(await screen.findByText(/valid email address/i)).toBeInTheDocument();

    // Blur the empty subject -> required error.
    await user.click(screen.getByLabelText(/subject/i));
    await user.tab();
    expect(await screen.findByText(/this field is required/i)).toBeInTheDocument();
    expect(submit).toBeDisabled();
  });

  it("AC-2: maps a backend 422 response onto its fields", async () => {
    server.use(
      http.post("/api/tickets", () =>
        HttpResponse.json(
          {
            error: {
              message: "Please correct the highlighted fields",
              fields: { email: "That email is blocked", subject: "Subject too short" },
            },
          },
          { status: 422 },
        ),
      ),
    );

    const user = userEvent.setup();
    renderWithProviders(<OpenTicketPage />);

    await user.type(screen.getByLabelText(/name/i), "Jane Doe");
    await user.type(screen.getByLabelText(/email/i), "jane@example.com");
    await user.type(screen.getByLabelText(/subject/i), "Help");
    await user.type(screen.getByLabelText(/message/i), "It is broken");
    await user.click(screen.getByRole("button", { name: /submit ticket/i }));

    expect(await screen.findByText("That email is blocked")).toBeInTheDocument();
    expect(await screen.findByText("Subject too short")).toBeInTheDocument();
    // No confirmation rendered on a 422.
    expect(screen.queryByText(/ticket .* created/i)).not.toBeInTheDocument();
  });

  it("AC-3: a successful submit renders the confirmation with the returned ticket number", async () => {
    server.use(
      http.post("/api/tickets", () =>
        HttpResponse.json({ ticketNumber: 987654 }, { status: 201 }),
      ),
    );

    const user = userEvent.setup();
    renderWithProviders(<OpenTicketPage />);

    await user.type(screen.getByLabelText(/name/i), "Jane Doe");
    await user.type(screen.getByLabelText(/email/i), "jane@example.com");
    await user.type(screen.getByLabelText(/subject/i), "Help");
    await user.type(screen.getByLabelText(/message/i), "It is broken");
    await user.click(screen.getByRole("button", { name: /submit ticket/i }));

    await waitFor(() => expect(screen.getByText("987654")).toBeInTheDocument());
    expect(screen.getByText(/ticket .* created/i)).toBeInTheDocument();
  });
});
