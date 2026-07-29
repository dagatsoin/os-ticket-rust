// Client portal (TS-M1-D2): a login form (ticket number + email) and a READ-ONLY
// thread view of the bound ticket. No reply box in M1. The client realm is
// token-less and isolated from the staff realm (distinct cookie + store).
//
// @implements BS-010: a client reads only its own (session-bound) ticket.
// @implements FS-010 (security): the thread renders M+R only, never internal N.
import { useEffect, useState } from "react";
import { observer } from "mobx-react-lite";
import { Box, Stack, Typography } from "@mui/material";
import { Navigate, useNavigate } from "react-router-dom";
import { useStores } from "../stores/StoreContext";
import { CredentialForm, type CredentialField } from "../components/CredentialForm";
import { ThreadView, type ThreadEntry } from "../components/ThreadView";
import { AttachmentList } from "../components/AttachmentList";
import type { ClientThreadEntry } from "../stores/ClientPortalStore";

const CLIENT_LOGIN_FIELDS: CredentialField[] = [
  { name: "ticketNumber", label: "Ticket Number", required: true },
  { name: "email", label: "Email Address", type: "email", required: true, autoComplete: "email" },
];

/**
 * Map a client thread entry to the ThreadView shape. Defensively renders ONLY
 * `M`/`R` — an internal `N` note must never reach a client even if one somehow
 * appeared in the payload (the route already excludes it; this is belt-and-braces).
 */
function toThreadEntries(entries: ClientThreadEntry[]): ThreadEntry[] {
  return entries
    .filter((e) => e.threadType === "M" || e.threadType === "R")
    .map((e) => ({
      id: e.id,
      author: e.poster,
      // BACKEND GAP: GET /api/client/ticket returns thread entries as
      // { id, thread_type, poster, body } only — no per-entry `created`
      // timestamp (see backend api/src/client.rs `ClientThreadEntry`). We do NOT
      // fabricate one; the timestamp stays empty until the backend adds a
      // per-entry created field to the client thread response.
      timestamp: "",
      kind: e.threadType === "R" ? "response" : "message",
      body: (
        <>
          <Typography variant="body2" sx={{ whiteSpace: "pre-wrap" }}>
            {e.body}
          </Typography>
          {/* Client downloads use the session-bound route (no ticketId, §8). */}
          <AttachmentList realm="client" attachments={e.attachments} />
        </>
      ),
    }));
}

/** Client sign-in (ticket number + email; single generic error on failure). */
export const ClientLoginPage = observer(function ClientLoginPage() {
  const { clientPortal } = useStores();
  const navigate = useNavigate();

  if (clientPortal.isAuthenticated) {
    return <Navigate to="/tickets" replace />;
  }

  return (
    <Stack spacing={3} sx={{ maxWidth: 420 }}>
      <Typography variant="h5">View Your Ticket</Typography>
      <Typography color="text.secondary">
        Enter your ticket number and the email you used to open it.
      </Typography>
      <CredentialForm
        fields={CLIENT_LOGIN_FIELDS}
        submitLabel="View Ticket"
        onSubmit={async (values) => {
          await clientPortal.login({
            ticketNumber: values.ticketNumber,
            email: values.email,
          });
          navigate("/tickets");
        }}
      />
    </Stack>
  );
});

/** Read-only ticket thread (M+R only; NO reply box in M1). */
export const ClientTicketsPage = observer(function ClientTicketsPage() {
  const { clientPortal } = useStores();
  // Attempt a one-shot session restore before deciding to bounce to login.
  const [restoreTried, setRestoreTried] = useState(clientPortal.ticket !== null);

  useEffect(() => {
    if (clientPortal.ticket) return;
    void clientPortal.loadTicket().finally(() => setRestoreTried(true));
  }, [clientPortal]);

  const ticket = clientPortal.ticket;

  if (!ticket) {
    if (!restoreTried || clientPortal.loading) {
      return <Typography color="text.secondary">Loading…</Typography>;
    }
    return <Navigate to="/tickets/login" replace />;
  }

  return (
    <Stack spacing={3}>
      <Box>
        <Typography variant="h5">{ticket.subject}</Typography>
        <Typography color="text.secondary">
          Ticket #{ticket.number} · Status: {ticket.status}
        </Typography>
      </Box>
      {/* Read-only: no replySlot is passed, so ThreadView renders no composer. */}
      <ThreadView entries={toThreadEntries(ticket.entries)} />
    </Stack>
  );
});
