// Reusable permission-flag grid (TS-M4-B4) — renders the canonical eleven
// BS-031-020 flags as Yes/No radio pairs with hint text. Pure presentational
// component (no store awareness) so it can be REUSED by EPIC-M4-C. The caller
// owns the flag booleans and receives per-flag change callbacks.
//
// @implements FS-031.8: the eleven flags render as Yes/No pairs with hints.
// @implements BS-031-020: canonical eleven-flag permission set.
import {
  FormControl,
  FormControlLabel,
  FormLabel,
  Radio,
  RadioGroup,
  Stack,
  Typography,
} from "@mui/material";
import {
  GROUP_FLAG_KEYS,
  GROUP_FLAG_META,
  type GroupFlagKey,
} from "../stores/GroupAdminStore";

export interface PermissionGridProps {
  /** Current flag booleans, keyed by storage key. */
  flags: Record<GroupFlagKey, boolean>;
  /** Called when a flag's Yes/No value changes. */
  onChange: (key: GroupFlagKey, value: boolean) => void;
  disabled?: boolean;
}

/**
 * The eleven permission flags as labelled Yes/No radio pairs.
 * @implements FS-031.8 / BS-031-020.
 */
export function PermissionGrid({ flags, onChange, disabled }: PermissionGridProps) {
  return (
    <Stack spacing={1.5} data-testid="permission-grid">
      {GROUP_FLAG_KEYS.map((key) => {
        const meta = GROUP_FLAG_META[key];
        return (
          <FormControl
            key={key}
            component="fieldset"
            disabled={disabled}
            sx={{ display: "flex" }}
            data-testid={`perm-${key}`}
          >
            <Stack
              direction={{ xs: "column", sm: "row" }}
              spacing={1}
              alignItems={{ sm: "center" }}
              justifyContent="space-between"
            >
              <FormLabel sx={{ color: "text.primary" }}>
                {meta.label}
                <Typography variant="caption" color="text.secondary" display="block">
                  {meta.hint}
                </Typography>
              </FormLabel>
              <RadioGroup
                row
                aria-label={meta.label}
                value={flags[key] ? "yes" : "no"}
                onChange={(e) => onChange(key, e.target.value === "yes")}
              >
                <FormControlLabel
                  value="yes"
                  control={<Radio size="small" data-testid={`perm-${key}-yes`} />}
                  label="Yes"
                />
                <FormControlLabel
                  value="no"
                  control={<Radio size="small" data-testid={`perm-${key}-no`} />}
                  label="No"
                />
              </RadioGroup>
            </Stack>
          </FormControl>
        );
      })}
    </Stack>
  );
}
