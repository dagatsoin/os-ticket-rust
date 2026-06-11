// Shared, consumer-agnostic attachment chip — RENDER ONLY, no fetch (TS-M2-A4,
// OWNED here; ROADMAP M2 Decisions §14). Consumers inject behaviour:
//   - TS-M2-B2 passes `onClick` (fetch→blob→objectURL download).
//   - TS-M2-D3 passes `readOnlyMarker` ("from canned response") for carried chips.
//   - TS-M2-A4 renders it bare on the /open confirmation page (local File.name).
//
// @implements FS-022.10: attachment presented as a labelled, downloadable chip.
import { Chip, Tooltip } from "@mui/material";
import AttachFileIcon from "@mui/icons-material/AttachFile";
import type { ReactElement } from "react";

export interface AttachmentChipProps {
  /** Filename shown on the chip. */
  label: string;
  /** Optional leading icon; defaults to a paperclip. */
  icon?: ReactElement;
  /** When set, the chip is clickable (consumer wires download behaviour). */
  onClick?: () => void;
  /**
   * When set, the chip is decorated as read-only (e.g. "from canned response")
   * and surfaces the marker as a tooltip; takes precedence over `onClick`.
   */
  readOnlyMarker?: string;
}

export function AttachmentChip({ label, icon, onClick, readOnlyMarker }: AttachmentChipProps) {
  const readOnly = readOnlyMarker !== undefined;
  const chip = (
    <Chip
      icon={icon ?? <AttachFileIcon />}
      label={label}
      size="small"
      variant="outlined"
      color={readOnly ? "default" : "primary"}
      onClick={readOnly ? undefined : onClick}
      clickable={!readOnly && Boolean(onClick)}
      data-testid="attachment-chip"
      data-readonly={readOnly ? "true" : undefined}
      sx={{ maxWidth: "100%" }}
    />
  );

  return readOnly ? (
    <Tooltip title={readOnlyMarker}>
      <span aria-label={`${label} (${readOnlyMarker})`}>{chip}</span>
    </Tooltip>
  ) : (
    chip
  );
}
