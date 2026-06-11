// Staff area (TS-M1-C4): login, the Open-tickets queue, and the single-ticket
// detail with a reply-only box. The staff auth store holds the /api/staff/me
// PROFILE (never a token); a 401 from any staff route is turned into a redirect
// to /staff/login by the apiClient (see TS-M1-A5).
//
// @implements BS-002: staff login + authenticated profile.
// @implements BS-020: open-tickets queue (number/subject/email/created).
// @implements BS-021: ticket detail (full thread) + reply-then-refresh.
import { useEffect } from "react";
import { observer } from "mobx-react-lite";
import {
  Alert,
  Box,
  Button,
  Divider,
  FormControl,
  FormHelperText,
  InputLabel,
  MenuItem,
  Paper,
  Select,
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
import { AttachmentList } from "../components/AttachmentList";
import { AttachmentChip } from "../components/AttachmentChip";
import { AnsweredBadge } from "../components/AnsweredBadge";
import {
  ALLOWED_EXTENSIONS,
  ALLOWED_EXTENSIONS_LABEL,
  MAX_FILE_SIZE_LABEL,
} from "../utils/validateAttachment";
import type { StaffThreadEntry } from "../stores/StaffTicketStore";

/** `accept` for the reply own-file input (seeded allow-list). */
const REPLY_FILE_ACCEPT = ALLOWED_EXTENSIONS.join(",");

const STAFF_LOGIN_FIELDS: CredentialField[] = [
  { name: "username", label: "Username", required: true, autoComplete: "username" },
  { name: "password", label: "Password", type: "password", required: true, autoComplete: "current-password" },
];

/**
 * Map a staff thread entry to the realm-agnostic ThreadView shape, composing the
 * body text with clickable attachment chips (B2). Staff downloads are
 * ticket-scoped, so the AttachmentList needs the ticket id.
 */
function toThreadEntry(e: StaffThreadEntry, ticketId: number): ThreadEntry {
  const kind = e.threadType === "R" ? "response" : e.threadType === "N" ? "note" : "message";
  const author = e.threadType === "N" ? `${e.poster} (internal note)` : e.poster;
  return {
    id: e.id,
    author,
    timestamp: "",
    kind,
    body: (
      <>
        <Typography variant="body2" sx={{ whiteSpace: "pre-wrap" }}>
          {e.body}
        </Typography>
        <AttachmentList realm="staff" ticketId={ticketId} attachments={e.attachments} />
      </>
    ),
  };
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
              <TableCell>Status</TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {staffTickets.queue.length === 0 ? (
              <TableRow>
                <TableCell colSpan={5}>
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
                  <TableCell>
                    <AnsweredBadge answered={t.isanswered} />
                  </TableCell>
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

  useEffect(() => {
    if (Number.isFinite(ticketId)) {
      void staffTickets.loadDetail(ticketId);
      void staffTickets.loadCanned(ticketId);
    }
    return () => staffTickets.clearDetail();
  }, [staffTickets, ticketId]);

  const detail = staffTickets.detail;

  async function handleReply() {
    if (!staffTickets.canReply) return;
    try {
      await staffTickets.reply(ticketId);
    } catch {
      // Error surfaced via staffTickets.replyError.
    }
  }

  function handleCannedChange(value: string) {
    if (value === "") {
      staffTickets.clearCanned();
      return;
    }
    void staffTickets.selectCanned(ticketId, Number(value));
  }

  const replySlot = (
    <Paper variant="outlined" sx={{ p: 2 }}>
      <Stack spacing={2}>
        <Typography variant="subtitle2">Reply to customer</Typography>
        {staffTickets.replyError ? (
          <Alert severity="error">{staffTickets.replyError}</Alert>
        ) : null}

        {/* Canned-response dropdown: selecting one fills the textarea (D3 AC-1/2). */}
        <FormControl fullWidth size="small" disabled={staffTickets.cannedList.length === 0}>
          <InputLabel id="canned-response-label">Canned response</InputLabel>
          <Select
            labelId="canned-response-label"
            label="Canned response"
            value={staffTickets.selectedCannedId === null ? "" : String(staffTickets.selectedCannedId)}
            onChange={(e) => handleCannedChange(e.target.value)}
            data-testid="canned-select"
          >
            <MenuItem value="">
              <em>None</em>
            </MenuItem>
            {staffTickets.cannedList.map((c) => (
              <MenuItem key={c.id} value={String(c.id)}>
                {c.title}
              </MenuItem>
            ))}
          </Select>
        </FormControl>

        <TextField
          label="Reply"
          multiline
          rows={4}
          fullWidth
          value={staffTickets.replyBody}
          onChange={(e) => staffTickets.setReplyBody(e.target.value)}
          inputProps={{ "data-testid": "reply-textarea" }}
        />

        {/* Read-only chips carried by the selected canned response (D3 AC-3). */}
        {staffTickets.cannedAttachments.length > 0 ? (
          <Stack direction="row" spacing={1} flexWrap="wrap" useFlexGap>
            {staffTickets.cannedAttachments.map((att) => (
              <AttachmentChip
                key={att.id}
                label={att.name}
                readOnlyMarker="from canned response"
              />
            ))}
          </Stack>
        ) : null}

        {/* Own-file input (reuses validateAttachment via the store) (D3 AC-4). */}
        <Box>
          <Button variant="outlined" component="label" size="small" data-testid="reply-file-button">
            {staffTickets.replyFile ? "Change file" : "Attach a file (optional)"}
            <input
              type="file"
              hidden
              accept={REPLY_FILE_ACCEPT}
              data-testid="reply-file-input"
              onChange={(e) => {
                const f = e.target.files?.[0];
                if (f) staffTickets.setReplyFile(f);
                e.target.value = "";
              }}
            />
          </Button>
          {staffTickets.replyFile ? (
            <Box sx={{ mt: 1 }}>
              <AttachmentChip
                label={staffTickets.replyFile.name}
                onClick={() => staffTickets.clearReplyFile()}
              />
            </Box>
          ) : null}
          <FormHelperText error={Boolean(staffTickets.replyFileError)} data-testid="reply-file-helper">
            {staffTickets.replyFileError ??
              `Allowed types: ${ALLOWED_EXTENSIONS_LABEL}. Max size: ${MAX_FILE_SIZE_LABEL}.`}
          </FormHelperText>
        </Box>

        <Divider />
        <Box>
          <Button
            variant="contained"
            onClick={() => void handleReply()}
            disabled={!staffTickets.canReply}
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
            <Stack direction="row" spacing={1} alignItems="center">
              <Typography variant="h5">{detail.subject}</Typography>
              <AnsweredBadge answered={detail.isanswered} />
            </Stack>
            <Typography color="text.secondary">
              Ticket #{detail.number} · {detail.name} · {detail.email}
            </Typography>
          </Box>
          <ThreadView
            entries={detail.entries.map((e) => toThreadEntry(e, ticketId))}
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
