// Answered / Unanswered status badge (ROADMAP M2 §3). Shown on the staff queue
// rows and the ticket detail header. A client message → Unanswered; ANY staff
// reply → Answered (the backend computes `isanswered`).
//
// @implements BS-022.15: ticket answered/unanswered state surfaced to staff.
import { Chip } from "@mui/material";

export function AnsweredBadge({ answered }: { answered: boolean | undefined }) {
  return (
    <Chip
      size="small"
      variant="outlined"
      color={answered ? "success" : "warning"}
      label={answered ? "Answered" : "Unanswered"}
      data-testid="answered-badge"
    />
  );
}
