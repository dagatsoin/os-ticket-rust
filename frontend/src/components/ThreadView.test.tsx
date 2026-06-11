import { describe, expect, it } from "vitest";
import { screen } from "@testing-library/react";
import { renderWithProviders } from "../test/renderWithProviders";
import { ThreadView, type ThreadEntry } from "./ThreadView";

const entries: ThreadEntry[] = [
  { id: 1, author: "Client", timestamp: "2026-06-10 09:00", body: "My printer is broken", kind: "message" },
  { id: 2, author: "Agent Smith", timestamp: "2026-06-10 09:15", body: "Have you tried turning it off and on?", kind: "response" },
];

describe("ThreadView", () => {
  it("renders entries in the order provided", () => {
    renderWithProviders(<ThreadView entries={entries} />);
    const rendered = screen.getAllByTestId("thread-entry");
    expect(rendered).toHaveLength(2);
    expect(screen.getByText("My printer is broken")).toBeInTheDocument();
    expect(screen.getByText("Agent Smith")).toBeInTheDocument();
  });

  it("renders the reply slot when supplied (C4 case)", () => {
    renderWithProviders(
      <ThreadView entries={entries} replySlot={<div>reply box</div>} />,
    );
    expect(screen.getByTestId("thread-reply-slot")).toBeInTheDocument();
    expect(screen.getByText("reply box")).toBeInTheDocument();
  });

  it("omits the reply slot when not supplied (D2 read-only case)", () => {
    renderWithProviders(<ThreadView entries={entries} />);
    expect(screen.queryByTestId("thread-reply-slot")).not.toBeInTheDocument();
  });

  it("shows the empty label when there are no entries", () => {
    renderWithProviders(<ThreadView entries={[]} emptyLabel="Nothing here" />);
    expect(screen.getByText("Nothing here")).toBeInTheDocument();
  });
});
