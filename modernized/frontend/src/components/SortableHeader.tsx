// Sortable column header component (TS-M3-B4): renders a clickable table header
// with sort indicator that toggles between ASC/DESC.
//
// @implements FS-020.7: sortable column headers
// @implements BS-020.8: sort column indicator
import { TableCell, TableSortLabel } from "@mui/material";
import type { SortKey, SortOrder } from "../stores/StaffTicketStore";

export interface SortableHeaderProps {
  /** The sort key for this column. */
  sortKey: SortKey;
  /** The label text to display. */
  label: string;
  /** The currently active sort key (null if none). */
  activeSort: SortKey | null;
  /** The current sort order. */
  activeOrder: SortOrder | null;
  /** Called when the header is clicked. */
  onSort: (sortKey: SortKey) => void;
  /** Optional: align content. */
  align?: "left" | "center" | "right";
}

/**
 * A clickable table header cell with sort indicator.
 * - When clicked on a non-active column: activates that column with DESC order.
 * - When clicked on the active column: toggles between DESC and ASC.
 *
 * @implements FS-020.7: sortable column headers (TS-M3-B4)
 */
export function SortableHeader({
  sortKey,
  label,
  activeSort,
  activeOrder,
  onSort,
  align = "left",
}: SortableHeaderProps) {
  const isActive = activeSort === sortKey;
  // MUI uses "asc" / "desc" (lowercase).
  const direction = activeOrder === "ASC" ? "asc" : "desc";

  return (
    <TableCell align={align} sortDirection={isActive ? direction : false}>
      <TableSortLabel
        active={isActive}
        direction={isActive ? direction : "desc"}
        onClick={() => onSort(sortKey)}
        data-testid={`sort-header-${sortKey}`}
      >
        {label}
      </TableSortLabel>
    </TableCell>
  );
}
