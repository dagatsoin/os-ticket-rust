import { describe, expect, it, vi } from "vitest";
import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithProviders } from "../test/renderWithProviders";
import { AttachmentChip } from "./AttachmentChip";

describe("AttachmentChip (TS-M2-A4 shared shell)", () => {
  it("renders its label", () => {
    renderWithProviders(<AttachmentChip label="invoice.pdf" />);
    expect(screen.getByText("invoice.pdf")).toBeInTheDocument();
  });

  it("invokes onClick when clicked (B2 download wiring)", async () => {
    const onClick = vi.fn();
    renderWithProviders(<AttachmentChip label="policy.txt" onClick={onClick} />);
    await userEvent.click(screen.getByTestId("attachment-chip"));
    expect(onClick).toHaveBeenCalledOnce();
  });

  it("is read-only and does NOT invoke onClick when a readOnlyMarker is set (D3 carried chip)", async () => {
    const onClick = vi.fn();
    renderWithProviders(
      <AttachmentChip label="policy.txt" onClick={onClick} readOnlyMarker="from canned response" />,
    );
    const chip = screen.getByTestId("attachment-chip");
    expect(chip).toHaveAttribute("data-readonly", "true");
    await userEvent.click(chip);
    expect(onClick).not.toHaveBeenCalled();
    // The marker is reachable for assistive tech / hover.
    expect(screen.getByLabelText("policy.txt (from canned response)")).toBeInTheDocument();
  });
});
