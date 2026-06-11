import { describe, expect, it } from "vitest";
import { fireEvent, screen, waitFor } from "@testing-library/react";
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

  // --- TS-M2-A4: /open file input + pre-check + confirmation chip ---

  it("A4-1: renders a file input with helper text naming the allowed types + 1 MB cap", () => {
    renderWithProviders(<OpenTicketPage />);
    expect(screen.getByTestId("open-ticket-file-input")).toBeInTheDocument();
    const helper = screen.getByTestId("open-ticket-file-helper");
    expect(helper).toHaveTextContent(/\.pdf/);
    expect(helper).toHaveTextContent(/1 MB/i);
  });

  it("A4-3: a disallowed type shows an inline error and blocks submission", async () => {
    const user = userEvent.setup();
    renderWithProviders(<OpenTicketPage />);

    await user.type(screen.getByLabelText(/name/i), "Jane Doe");
    await user.type(screen.getByLabelText(/email/i), "jane@example.com");
    await user.type(screen.getByLabelText(/subject/i), "Help");
    await user.type(screen.getByLabelText(/message/i), "It is broken");

    const input = screen.getByTestId("open-ticket-file-input") as HTMLInputElement;
    // fireEvent.change bypasses the native accept filter to simulate a file that
    // slips past it, exercising the store-side pre-check (the real defense).
    fireEvent.change(input, {
      target: { files: [new File(["x"], "evil.exe", { type: "application/octet-stream" })] },
    });

    expect(screen.getByTestId("open-ticket-file-helper")).toHaveTextContent(/invalid file type/i);
    expect(screen.getByRole("button", { name: /submit ticket/i })).toBeDisabled();
  });

  it("A4-4: an oversized permitted type shows a 'too big' inline error", async () => {
    const user = userEvent.setup();
    renderWithProviders(<OpenTicketPage />);

    const input = screen.getByTestId("open-ticket-file-input") as HTMLInputElement;
    const big = new File(["a".repeat(2 * 1024 * 1024)], "big.pdf", { type: "application/pdf" });
    await user.upload(input, big);

    expect(screen.getByTestId("open-ticket-file-helper")).toHaveTextContent(/too big/i);
  });

  it("A4-2: a valid submission with a file shows the confirmation with an AttachmentChip", async () => {
    server.use(
      http.post("/api/tickets", () => HttpResponse.json({ ticketNumber: 222333 }, { status: 201 })),
    );

    const user = userEvent.setup();
    renderWithProviders(<OpenTicketPage />);

    await user.type(screen.getByLabelText(/name/i), "Jane Doe");
    await user.type(screen.getByLabelText(/email/i), "jane@example.com");
    await user.type(screen.getByLabelText(/subject/i), "Help");
    await user.type(screen.getByLabelText(/message/i), "It is broken");

    const input = screen.getByTestId("open-ticket-file-input") as HTMLInputElement;
    await user.upload(input, new File(["a".repeat(500)], "invoice.pdf", { type: "application/pdf" }));

    await user.click(screen.getByRole("button", { name: /submit ticket/i }));

    await waitFor(() => expect(screen.getByText("222333")).toBeInTheDocument());
    // The confirmation chip label is the local File.name (§11).
    const chip = screen.getByTestId("attachment-chip");
    expect(chip).toHaveTextContent("invoice.pdf");
  });
});
