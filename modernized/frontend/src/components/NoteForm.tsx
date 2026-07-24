/**
 * Internal note form component for ticket detail view.
 * Allows staff to post internal notes with optional title and state change.
 *
 * @implements TS-M3-E3: note form UI + thread display of notes.
 */
import { useState } from "react";
import { observer } from "mobx-react-lite";
import {
  Alert,
  Box,
  Button,
  FormControl,
  InputLabel,
  MenuItem,
  Paper,
  Select,
  Snackbar,
  Stack,
  TextField,
  Typography,
} from "@mui/material";
import { useNavigate } from "react-router-dom";
import { useStores } from "../stores/StoreContext";
import type { NoteState } from "../stores/StaffTicketStore";

/** State change options for the dropdown (TS-M3-E3 AC-3). */
const STATE_OPTIONS: Array<{ value: NoteState; label: string }> = [
  { value: "unchanged", label: "(unchanged)" },
  { value: "closed", label: "Close Ticket" },
  { value: "open", label: "Reopen Ticket" },
  { value: "answered", label: "Answered" },
  { value: "unanswered", label: "Unanswered" },
  { value: "overdue", label: "Overdue" },
  { value: "notdue", label: "Not Due" },
];

interface NoteFormProps {
  ticketId: number;
  ticketStatus: string;
}

/**
 * Note form component with title, body, and state change options.
 * @implements TS-M3-E3 AC-1 through AC-6
 */
export const NoteForm = observer(function NoteForm({ ticketId, ticketStatus: _ticketStatus }: NoteFormProps) {
  const { staffTickets } = useStores();
  const navigate = useNavigate();

  // Local validation state for "attempted submit with empty body" (TS-M3-E3 AC-2)
  const [showBodyRequired, setShowBodyRequired] = useState(false);

  // Toast state
  const [toast, setToast] = useState<{
    open: boolean;
    message: string;
    severity: "success" | "error";
  }>({
    open: false,
    message: "",
    severity: "success",
  });

  const handleCloseToast = () => setToast({ ...toast, open: false });

  const handleSubmit = async () => {
    // Show validation error if body is empty (TS-M3-E3 AC-2)
    if (staffTickets.noteBody.trim() === "") {
      setShowBodyRequired(true);
      return;
    }
    setShowBodyRequired(false);

    if (!staffTickets.canSendNote) return;

    // Capture state BEFORE postNote clears the composer (TS-M3-E3 AC-6 fix)
    const requestedState = staffTickets.noteState;
    const result = await staffTickets.postNote(ticketId);

    if (result.success) {
      // Check if state changed to closed - redirect to listing (TS-M3-E3 AC-6)
      if (result.stateChanged && requestedState === "closed") {
        setToast({
          open: true,
          message: "Note posted and ticket closed",
          severity: "success",
        });
        // Small delay to show toast before navigating
        setTimeout(() => navigate("/staff/tickets?status=closed"), 500);
      } else {
        setToast({
          open: true,
          message: "Note posted successfully",
          severity: "success",
        });
      }
    } else if (staffTickets.noteError) {
      setToast({
        open: true,
        message: staffTickets.noteError,
        severity: "error",
      });
    }
  };

  return (
    <>
      <Paper variant="outlined" sx={{ p: 2 }} data-testid="note-form">
        <Stack spacing={2}>
          <Typography variant="subtitle2">Post Internal Note</Typography>

          {staffTickets.noteError && (
            <Alert severity="error">{staffTickets.noteError}</Alert>
          )}

          {/* Optional title field (TS-M3-E3 AC-5) */}
          <TextField
            label="Title (optional)"
            fullWidth
            size="small"
            value={staffTickets.noteTitle}
            onChange={(e) => staffTickets.setNoteTitle(e.target.value)}
            inputProps={{ "data-testid": "note-title" }}
          />

          {/* Required body field (TS-M3-E3 AC-2) */}
          <TextField
            label="Note"
            multiline
            rows={4}
            fullWidth
            required
            value={staffTickets.noteBody}
            onChange={(e) => {
              staffTickets.setNoteBody(e.target.value);
              // Clear validation error when user starts typing
              if (e.target.value.trim() !== "") {
                setShowBodyRequired(false);
              }
            }}
            error={showBodyRequired}
            helperText={showBodyRequired ? "Note required" : "Internal notes are only visible to staff"}
            inputProps={{ "data-testid": "note-body" }}
          />

          {/* State change dropdown (TS-M3-E3 AC-3) */}
          <FormControl fullWidth size="small">
            <InputLabel id="note-state-label">Ticket Status</InputLabel>
            <Select
              labelId="note-state-label"
              label="Ticket Status"
              value={staffTickets.noteState}
              onChange={(e) => staffTickets.setNoteState(e.target.value as NoteState)}
              data-testid="note-state-select"
            >
              {STATE_OPTIONS.map((opt) => (
                <MenuItem key={opt.value} value={opt.value}>
                  {opt.label}
                </MenuItem>
              ))}
            </Select>
          </FormControl>

          <Box>
            <Button
              variant="contained"
              onClick={handleSubmit}
              disabled={!staffTickets.canSendNote}
              data-testid="note-submit"
            >
              {staffTickets.noteSending ? "Posting..." : "Post Note"}
            </Button>
          </Box>
        </Stack>
      </Paper>

      {/* Toast notifications */}
      <Snackbar
        open={toast.open}
        autoHideDuration={5000}
        onClose={handleCloseToast}
        anchorOrigin={{ vertical: "top", horizontal: "center" }}
      >
        <Alert
          onClose={handleCloseToast}
          severity={toast.severity}
          variant="filled"
          sx={{ width: "100%" }}
        >
          {toast.message}
        </Alert>
      </Snackbar>
    </>
  );
});
