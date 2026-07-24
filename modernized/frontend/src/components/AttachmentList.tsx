// Clickable thread-entry attachment chips (TS-M2-B2). Renders one AttachmentChip
// per attachment (§7) and wires each click to the credentialed
// fetch→blob→objectURL download (§9). A 403/404 surfaces a VISIBLE inline error
// near the chips instead of navigating away (US-M2-3 AC-3 / EC-022.9).
//
// Rendered in BOTH the staff detail thread (staff realm, ticket-scoped route)
// and the client portal thread (client realm, session-bound route).
//
// @implements FS-022.10: clickable attachment chips → authorized download.
// @implements EC-022.9: unauthorized / unknown id → visible inline error.
import { useState } from "react";
import { Alert, Stack } from "@mui/material";
import { AttachmentChip } from "./AttachmentChip";
import {
  downloadAttachment,
  type Attachment,
  type DownloadDeps,
} from "../utils/downloadAttachment";
import type { Realm } from "../api/types";

export interface AttachmentListProps {
  realm: Realm;
  /** Required for staff downloads; ignored for the session-bound client route. */
  ticketId?: number;
  attachments: Attachment[] | undefined;
  /** Test seam for the browser download hooks. */
  downloadDeps?: Partial<DownloadDeps>;
}

export function AttachmentList({ realm, ticketId, attachments, downloadDeps }: AttachmentListProps) {
  const [error, setError] = useState<string | null>(null);
  const [busyId, setBusyId] = useState<number | null>(null);

  if (!attachments || attachments.length === 0) return null;

  async function handleDownload(att: Attachment) {
    setError(null);
    setBusyId(att.id);
    const result = await downloadAttachment(realm, att, ticketId, downloadDeps);
    setBusyId(null);
    if (!result.ok) setError(result.error);
  }

  return (
    <Stack spacing={1} sx={{ mt: 1 }} data-testid="attachment-list">
      <Stack direction="row" spacing={1} flexWrap="wrap" useFlexGap>
        {attachments.map((att) => (
          <AttachmentChip
            key={att.id}
            label={busyId === att.id ? `${att.name}…` : att.name}
            onClick={() => void handleDownload(att)}
          />
        ))}
      </Stack>
      {error ? (
        <Alert severity="error" data-testid="attachment-download-error" onClose={() => setError(null)}>
          {error}
        </Alert>
      ) : null}
    </Stack>
  );
}
