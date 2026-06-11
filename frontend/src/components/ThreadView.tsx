// Realm-agnostic presentational primitive: renders an ordered list of thread entries
// passed in as props. NO store/realm awareness. Consumed by:
//   - TS-M1-C4 (staff UI) with a reply slot
//   - TS-M1-D2 (client portal) read-only (no slot)
import { Box, Divider, Paper, Stack, Typography } from "@mui/material";
import type { ReactNode } from "react";

/** One entry in a ticket thread (message / response / internal note). */
export interface ThreadEntry {
  id: string | number;
  /** Display author name. */
  author: string;
  /** Pre-formatted timestamp string (formatting is the caller's concern). */
  timestamp: string;
  /** Entry body. Plain text by default; callers may pass a node for richer content. */
  body: ReactNode;
  /** Optional coarse kind used only for a subtle visual accent. */
  kind?: "message" | "response" | "note";
}

export interface ThreadViewProps {
  /** Ordered entries (caller supplies the order; this component does not sort). */
  entries: ThreadEntry[];
  /** Optional reply composer slot rendered after the thread (C4 supplies it; D2 omits it). */
  replySlot?: ReactNode;
  /** Message shown when there are no entries. */
  emptyLabel?: string;
}

const KIND_ACCENT: Record<NonNullable<ThreadEntry["kind"]>, string> = {
  message: "primary.main",
  response: "secondary.main",
  note: "warning.main",
};

export function ThreadView({ entries, replySlot, emptyLabel = "No messages yet." }: ThreadViewProps) {
  return (
    <Stack spacing={2} data-testid="thread-view">
      {entries.length === 0 ? (
        <Typography color="text.secondary">{emptyLabel}</Typography>
      ) : (
        entries.map((entry) => (
          <Paper
            key={entry.id}
            variant="outlined"
            sx={{
              p: 2,
              borderLeft: 4,
              borderLeftColor: KIND_ACCENT[entry.kind ?? "message"],
            }}
            data-testid="thread-entry"
          >
            <Stack
              direction="row"
              justifyContent="space-between"
              alignItems="baseline"
              spacing={1}
            >
              <Typography variant="subtitle2">{entry.author}</Typography>
              <Typography variant="caption" color="text.secondary">
                {entry.timestamp}
              </Typography>
            </Stack>
            <Divider sx={{ my: 1 }} />
            <Box sx={{ whiteSpace: "pre-wrap" }}>
              {typeof entry.body === "string" ? (
                <Typography variant="body2">{entry.body}</Typography>
              ) : (
                entry.body
              )}
            </Box>
          </Paper>
        ))
      )}
      {replySlot ? <Box data-testid="thread-reply-slot">{replySlot}</Box> : null}
    </Stack>
  );
}
