import { describe, expect, it, vi } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithProviders } from "../test/renderWithProviders";
import { server, http, HttpResponse } from "../test/mockServer";
import { AttachmentList } from "./AttachmentList";
import type { Attachment } from "../utils/downloadAttachment";

const ATTS: Attachment[] = [
  { id: 42, name: "policy.txt", size: 10, mime: "text/plain" },
];

/** Stub the browser download hooks so we can assert without a real save. */
function browserStub() {
  return {
    triggerSave: vi.fn(),
    createObjectURL: vi.fn(() => "blob:mock"),
    revokeObjectURL: vi.fn(),
  };
}

describe("AttachmentList (TS-M2-B2)", () => {
  it("renders nothing when there are no attachments (regression: plain threads)", () => {
    const { container } = renderWithProviders(
      <AttachmentList realm="client" attachments={[]} />,
    );
    expect(container).toBeEmptyDOMElement();
  });

  it("renders a clickable chip per attachment", () => {
    renderWithProviders(<AttachmentList realm="client" attachments={ATTS} />);
    expect(screen.getByText("policy.txt")).toBeInTheDocument();
  });

  it("downloads via the session-bound client route on click", async () => {
    let hitUrl: string | null = null;
    server.use(
      http.get("/api/client/ticket/attachments/42", ({ request }) => {
        hitUrl = new URL(request.url).pathname;
        return new HttpResponse("policy-bytes", { status: 200 });
      }),
    );
    const browser = browserStub();
    renderWithProviders(
      <AttachmentList realm="client" attachments={ATTS} downloadDeps={browser} />,
    );

    await userEvent.click(screen.getByTestId("attachment-chip"));

    await waitFor(() => expect(browser.triggerSave).toHaveBeenCalledWith("blob:mock", "policy.txt"));
    expect(hitUrl).toBe("/api/client/ticket/attachments/42");
  });

  it("uses the staff ticket-scoped route when realm=staff", async () => {
    let hitUrl: string | null = null;
    server.use(
      http.get("/api/staff/tickets/7/attachments/42", ({ request }) => {
        hitUrl = new URL(request.url).pathname;
        return new HttpResponse("bytes", { status: 200 });
      }),
    );
    const browser = browserStub();
    renderWithProviders(
      <AttachmentList realm="staff" ticketId={7} attachments={ATTS} downloadDeps={browser} />,
    );

    await userEvent.click(screen.getByTestId("attachment-chip"));
    await waitFor(() => expect(browser.triggerSave).toHaveBeenCalled());
    expect(hitUrl).toBe("/api/staff/tickets/7/attachments/42");
  });

  it("shows a visible inline error on a 404 and does NOT download (AC-3 / EC-022.9)", async () => {
    server.use(
      http.get("/api/client/ticket/attachments/42", () =>
        HttpResponse.json({ error: { message: "Not found" } }, { status: 404 }),
      ),
    );
    const browser = browserStub();
    renderWithProviders(
      <AttachmentList realm="client" attachments={ATTS} downloadDeps={browser} />,
    );

    await userEvent.click(screen.getByTestId("attachment-chip"));

    expect(await screen.findByTestId("attachment-download-error")).toBeInTheDocument();
    expect(browser.triggerSave).not.toHaveBeenCalled();
  });
});
