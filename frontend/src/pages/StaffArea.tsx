// Staff area (TS-M1-C4): login, the Open-tickets queue, and the single-ticket
// detail with a reply-only box. The staff auth store holds the /api/staff/me
// PROFILE (never a token); a 401 from any staff route is turned into a redirect
// to /staff/login by the apiClient (see TS-M1-A5).
//
// @implements BS-002: staff login + authenticated profile.
// @implements BS-020: open-tickets queue (number/subject/email/created).
// @implements BS-021: ticket detail (full thread) + reply-then-refresh.
import { useEffect, useState } from "react";
import { observer } from "mobx-react-lite";
import {
  Alert,
  Box,
  Button,
  Paper,
  Stack,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableRow,
  TextField,
  Typography,
} from "@mui/material";
import {
  Link as RouterLink,
  Navigate,
  useNavigate,
  useParams,
} from "react-router-dom";
import { useStores } from "../stores/StoreContext";
import { CredentialForm, type CredentialField } from "../components/CredentialForm";
import { ThreadView, type ThreadEntry } from "../components/ThreadView";
import type { StaffThreadEntry } from "../stores/StaffTicketStore";

const STAFF_LOGIN_FIELDS: CredentialField[] = [
  { name: "username", label: "Username", required: true, autoComplete: "username" },
  { name: "password", label: "Password", type: "password", required: true, autoComplete: "current-password" },
];

/** Map a staff thread entry to the realm-agnostic ThreadView shape. */
function toThreadEntry(e: StaffThreadEntry): ThreadEntry {
  const kind = e.threadType === "R" ? "response" : e.threadType === "N" ? "note" : "message";
  const author = e.threadType === "N" ? `${e.poster} (internal note)` : e.poster;
  return { id: e.id, author, timestamp: "", body: e.body, kind };
}

/** `/staff` index — route authenticated agents to the queue, else to login. */
export const StaffDashboardPage = observer(function StaffDashboardPage() {
  const { staffAuth } = useStores();
  return <Navigate to={staffAuth.isAuthenticated ? "/staff/tickets" : "/staff/login"} replace />;
});

/** Staff sign-in (CredentialForm; generic error on failure). */
export const StaffLoginPage = observer(function StaffLoginPage() {
  const { staffAuth } = useStores();
  const navigate = useNavigate();

  if (staffAuth.isAuthenticated) {
    return <Navigate to="/staff/tickets" replace />;
  }

  return (
    <Stack spacing={3} sx={{ maxWidth: 420 }}>
      <Typography variant="h5">Staff Sign In</Typography>
      <CredentialForm
        fields={STAFF_LOGIN_FIELDS}
        submitLabel="Sign In"
        onSubmit={async (values) => {
          await staffAuth.login(values);
          navigate("/staff/tickets");
        }}
      />
    </Stack>
  );
});

/** Open-tickets queue (newest first; click a row to open the detail). */
export const StaffQueuePage = observer(function StaffQueuePage() {
  const { staffTickets, staffAuth } = useStores();
  const navigate = useNavigate();

  useEffect(() => {
    void staffTickets.loadQueue();
  }, [staffTickets]);

  return (
    <Stack spacing={3}>
      <Stack direction="row" justifyContent="space-between" alignItems="center">
        <Typography variant="h5">Open Tickets</Typography>
        <Button
          variant="text"
          onClick={async () => {
            await staffAuth.logout();
            navigate("/staff/login");
          }}
        >
          Log Out
        </Button>
      </Stack>

      {staffTickets.queueError ? (
        <Alert severity="error">{staffTickets.queueError}</Alert>
      ) : null}

      <Paper variant="outlined">
        <Table size="small">
          <TableHead>
            <TableRow>
              <TableCell>Number</TableCell>
              <TableCell>Subject</TableCell>
              <TableCell>Email</TableCell>
              <TableCell>Created</TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {staffTickets.queue.length === 0 ? (
              <TableRow>
                <TableCell colSpan={4}>
                  <Typography color="text.secondary">
                    {staffTickets.loadingQueue ? "Loading…" : "No open tickets."}
                  </Typography>
                </TableCell>
              </TableRow>
            ) : (
              staffTickets.queue.map((t) => (
                <TableRow
                  key={t.id}
                  hover
                  sx={{ cursor: "pointer" }}
                  onClick={() => navigate(`/staff/tickets/${t.id}`)}
                  data-testid="queue-row"
                >
                  <TableCell>{t.number}</TableCell>
                  <TableCell>{t.subject}</TableCell>
                  <TableCell>{t.email}</TableCell>
                  <TableCell>{t.created}</TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </Paper>
    </Stack>
  );
});

/** Single-ticket detail: full thread + reply-only box (refreshes from response). */
export const StaffTicketDetailPage = observer(function StaffTicketDetailPage() {
  const { staffTickets } = useStores();
  const { id } = useParams();
  const ticketId = Number(id);
  const [replyText, setReplyText] = useState("");

  useEffect(() => {
    if (Number.isFinite(ticketId)) {
      void staffTickets.loadDetail(ticketId);
    }
    return () => staffTickets.clearDetail();
  }, [staffTickets, ticketId]);

  const detail = staffTickets.detail;

  async function handleReply() {
    const body = replyText.trim();
    if (!body) return;
    try {
      await staffTickets.reply(ticketId, body);
      setReplyText("");
    } catch {
      // Error surfaced via staffTickets.replyError.
    }
  }

  const replySlot = (
    <Paper variant="outlined" sx={{ p: 2 }}>
      <Stack spacing={2}>
        <Typography variant="subtitle2">Reply to customer</Typography>
        {staffTickets.replyError ? (
          <Alert severity="error">{staffTickets.replyError}</Alert>
        ) : null}
        <TextField
          label="Reply"
          multiline
          rows={4}
          fullWidth
          value={replyText}
          onChange={(e) => setReplyText(e.target.value)}
        />
        <Box>
          <Button
            variant="contained"
            onClick={() => void handleReply()}
            disabled={staffTickets.replying || replyText.trim() === ""}
          >
            {staffTickets.replying ? "Sending…" : "Send Reply"}
          </Button>
        </Box>
      </Stack>
    </Paper>
  );

  return (
    <Stack spacing={3}>
      <Button component={RouterLink} to="/staff/tickets" variant="text" sx={{ alignSelf: "flex-start" }}>
        ← Back to queue
      </Button>

      {staffTickets.detailError ? (
        <Alert severity="error">{staffTickets.detailError}</Alert>
      ) : null}

      {detail ? (
        <>
          <Box>
            <Typography variant="h5">{detail.subject}</Typography>
            <Typography color="text.secondary">
              Ticket #{detail.number} · {detail.name} · {detail.email}
            </Typography>
          </Box>
          <ThreadView
            entries={detail.entries.map(toThreadEntry)}
            replySlot={replySlot}
          />
        </>
      ) : (
        <Typography color="text.secondary">
          {staffTickets.loadingDetail ? "Loading…" : "Ticket not found."}
        </Typography>
      )}
    </Stack>
  );
});
