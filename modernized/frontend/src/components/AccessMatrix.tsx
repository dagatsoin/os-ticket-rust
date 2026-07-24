// Reusable access-matrix (TS-M4-C2) — a checkbox list of {id,name} items with
// Select All / Select None. Pure presentational (no store awareness); the caller
// owns the checked set (full-replace semantics on save). Generalised from the
// original DeptAccessMatrix (TS-M4-B4) so EPIC-M4-C reuses it for the department
// form's group-access matrix (and any future roster-style checkbox grid).
//
// The `testIdPrefix` namespaces the emitted data-testids:
//   `${prefix}-access-matrix`, `${prefix}-select-all`, `${prefix}-select-none`,
//   `${prefix}-checkbox-${id}` — so multiple matrices can coexist in one form.
//
// @implements FS-031.8: checkbox matrix with select-all / select-none.
// @implements BS-031-021: the access set is a full-replace sync.
import {
  Box,
  Button,
  Checkbox,
  FormControlLabel,
  FormGroup,
  Stack,
  Typography,
} from "@mui/material";

/** A selectable {id,name} row. */
export interface AccessMatrixItem {
  id: number;
  name: string;
}

export interface AccessMatrixProps {
  /** All selectable items. */
  items: AccessMatrixItem[];
  /** Predicate: is this item currently checked? */
  isChecked: (id: number) => boolean;
  /** Toggle a single item. */
  onToggle: (id: number) => void;
  /** Check every item. */
  onSelectAll: () => void;
  /** Uncheck every item. */
  onSelectNone: () => void;
  /** data-testid namespace (default "access"). */
  testIdPrefix?: string;
  /** Message shown when there are no items (default "No items available."). */
  emptyLabel?: string;
  disabled?: boolean;
}

/**
 * A checkbox matrix over {id,name} items with bulk Select All / Select None.
 * @implements FS-031.8 / BS-031-021.
 */
export function AccessMatrix({
  items,
  isChecked,
  onToggle,
  onSelectAll,
  onSelectNone,
  testIdPrefix = "access",
  emptyLabel = "No items available.",
  disabled,
}: AccessMatrixProps) {
  return (
    <Box data-testid={`${testIdPrefix}-access-matrix`}>
      <Stack direction="row" spacing={1} sx={{ mb: 1 }}>
        <Button
          size="small"
          onClick={onSelectAll}
          disabled={disabled}
          data-testid={`${testIdPrefix}-select-all`}
        >
          Select All
        </Button>
        <Button
          size="small"
          onClick={onSelectNone}
          disabled={disabled}
          data-testid={`${testIdPrefix}-select-none`}
        >
          Select None
        </Button>
      </Stack>
      {items.length === 0 ? (
        <Typography variant="body2" color="text.secondary">
          {emptyLabel}
        </Typography>
      ) : (
        <FormGroup>
          {items.map((it) => (
            <FormControlLabel
              key={it.id}
              control={
                <Checkbox
                  size="small"
                  checked={isChecked(it.id)}
                  onChange={() => onToggle(it.id)}
                  disabled={disabled}
                  inputProps={{ "aria-label": it.name }}
                  data-testid={`${testIdPrefix}-checkbox-${it.id}`}
                />
              }
              label={it.name}
            />
          ))}
        </FormGroup>
      )}
    </Box>
  );
}
