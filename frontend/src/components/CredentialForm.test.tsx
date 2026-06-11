import { describe, expect, it, vi } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithProviders } from "../test/renderWithProviders";
import { CredentialForm, type CredentialField } from "./CredentialForm";
import { ApiError } from "../api/types";

const fields: CredentialField[] = [
  { name: "email", label: "Email", type: "email", required: true },
  { name: "password", label: "Password", type: "password", required: true },
];

describe("CredentialForm", () => {
  it("collects values and calls onSubmit with them", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn().mockResolvedValue(undefined);
    renderWithProviders(<CredentialForm fields={fields} onSubmit={onSubmit} />);

    await user.type(screen.getByLabelText(/email/i), "c@x.io");
    await user.type(screen.getByLabelText(/password/i), "secret");
    await user.click(screen.getByRole("button"));

    await waitFor(() =>
      expect(onSubmit).toHaveBeenCalledWith({ email: "c@x.io", password: "secret" }),
    );
  });

  it("maps 422 field errors from the error envelope onto inputs and shows the top-level message", async () => {
    const user = userEvent.setup();
    const onSubmit = vi
      .fn()
      .mockRejectedValue(
        new ApiError(422, "Validation failed", { email: "Invalid email" }),
      );
    renderWithProviders(<CredentialForm fields={fields} onSubmit={onSubmit} />);

    await user.type(screen.getByLabelText(/email/i), "bad");
    await user.click(screen.getByRole("button"));

    expect(await screen.findByTestId("credential-form-error")).toHaveTextContent(
      "Validation failed",
    );
    expect(await screen.findByText("Invalid email")).toBeInTheDocument();
  });
});
