// Overdue badge component (TS-M3-D5): shows a warning indicator for overdue tickets.
//
// @implements FS-020.3: overdue visual indicator in listing
// @implements FS-021.13: due date display
import { Chip, Tooltip } from "@mui/material";
import { WarningAmber } from "@mui/icons-material";

export interface OverdueBadgeProps {
  /** Whether the ticket is overdue. */
  isoverdue: boolean;
  /** The due date/time (ISO string). */
  duedate?: string;
  /** If true, show a compact icon-only badge. */
  compact?: boolean;
}

/**
 * Format a due date for display.
 */
function formatDueDate(duedate: string): string {
  try {
    const date = new Date(duedate);
    return date.toLocaleString(undefined, {
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return duedate;
  }
}

/**
 * Overdue badge shown on tickets that are past their due date.
 * @implements FS-020.3: overdue visual indicator (TS-M3-D5)
 */
export function OverdueBadge({ isoverdue, duedate, compact }: OverdueBadgeProps) {
  if (!isoverdue) {
    // If not overdue but has a due date, show it in neutral color.
    if (duedate) {
      return (
        <Tooltip title={`Due: ${formatDueDate(duedate)}`}>
          <Chip
            size="small"
            variant="outlined"
            label={formatDueDate(duedate)}
            data-testid="due-date-chip"
          />
        </Tooltip>
      );
    }
    return null;
  }

  // Overdue: show warning badge.
  const tooltipText = duedate
    ? `Overdue since ${formatDueDate(duedate)}`
    : "Overdue";

  if (compact) {
    return (
      <Tooltip title={tooltipText}>
        <WarningAmber
          color="warning"
          fontSize="small"
          data-testid="overdue-icon"
          sx={{ verticalAlign: "middle" }}
        />
      </Tooltip>
    );
  }

  return (
    <Tooltip title={tooltipText}>
      <Chip
        size="small"
        color="warning"
        icon={<WarningAmber />}
        label="Overdue"
        data-testid="overdue-badge"
      />
    </Tooltip>
  );
}
