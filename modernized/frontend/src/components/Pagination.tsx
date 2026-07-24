// Pagination controls component (TS-M3-B4): renders "Showing X-Y of N" info,
// page number links, and prev/next buttons.
//
// @implements FS-020.6: pagination controls
import { Button, Stack, Typography } from "@mui/material";
import { NavigateBefore, NavigateNext } from "@mui/icons-material";
import type { PaginationMeta } from "../stores/StaffTicketStore";

export interface PaginationProps {
  /** Pagination metadata from the API response. */
  pagination: PaginationMeta;
  /** Called when a page is selected. */
  onPageChange: (page: number) => void;
}

/**
 * Pagination controls with "Showing X-Y of N" info and page navigation.
 * @implements FS-020.6: pagination controls (TS-M3-B4)
 */
export function Pagination({ pagination, onPageChange }: PaginationProps) {
  const { page, pageSize, totalCount, totalPages } = pagination;

  // Don't render if there are no items.
  if (totalCount === 0) {
    return null;
  }

  // Calculate the range being shown.
  const start = (page - 1) * pageSize + 1;
  const end = Math.min(page * pageSize, totalCount);

  // Generate page numbers to show (windowed).
  const pageNumbers = getPageNumbers(page, totalPages);

  const canGoPrev = page > 1;
  const canGoNext = page < totalPages;

  return (
    <Stack
      direction="row"
      spacing={2}
      alignItems="center"
      justifyContent="space-between"
      sx={{ mt: 2 }}
      data-testid="pagination"
    >
      <Typography variant="body2" color="text.secondary" data-testid="pagination-info">
        Showing {start}-{end} of {totalCount}
      </Typography>

      <Stack direction="row" spacing={0.5} alignItems="center">
        <Button
          size="small"
          disabled={!canGoPrev}
          onClick={() => onPageChange(page - 1)}
          aria-label="Previous page"
          data-testid="pagination-prev"
        >
          <NavigateBefore fontSize="small" />
          Prev
        </Button>

        {pageNumbers.map((p, idx) =>
          p === "..." ? (
            <Typography key={`ellipsis-${idx}`} variant="body2" sx={{ px: 1 }}>
              ...
            </Typography>
          ) : (
            <Button
              key={p}
              size="small"
              variant={p === page ? "contained" : "text"}
              onClick={() => onPageChange(p)}
              data-testid={`pagination-page-${p}`}
              sx={{ minWidth: 36 }}
            >
              {p}
            </Button>
          ),
        )}

        <Button
          size="small"
          disabled={!canGoNext}
          onClick={() => onPageChange(page + 1)}
          aria-label="Next page"
          data-testid="pagination-next"
        >
          Next
          <NavigateNext fontSize="small" />
        </Button>
      </Stack>
    </Stack>
  );
}

/**
 * Generate an array of page numbers to display, with ellipsis for gaps.
 * Shows at most 7 items: first, last, current, and neighbors.
 */
function getPageNumbers(current: number, total: number): (number | "...")[] {
  if (total <= 7) {
    // Show all pages if 7 or fewer.
    return Array.from({ length: total }, (_, i) => i + 1);
  }

  const pages: (number | "...")[] = [];

  // Always include first page.
  pages.push(1);

  // Calculate the range around current page.
  const rangeStart = Math.max(2, current - 1);
  const rangeEnd = Math.min(total - 1, current + 1);

  // Add ellipsis before range if needed.
  if (rangeStart > 2) {
    pages.push("...");
  }

  // Add pages in the range.
  for (let i = rangeStart; i <= rangeEnd; i++) {
    pages.push(i);
  }

  // Add ellipsis after range if needed.
  if (rangeEnd < total - 1) {
    pages.push("...");
  }

  // Always include last page.
  pages.push(total);

  return pages;
}
