/**
 * Workflow action toolbar for ticket detail view.
 * Displays Claim, Assign, Transfer, Close, and Reopen buttons based on
 * ticket state and staff permissions.
 *
 * @implements TS-M3-C4: workflow action buttons + dialogs.
 */
import { useState } from "react";
import { observer } from "mobx-react-lite";
import {
  Alert,
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  FormControl,
  FormHelperText,
  InputLabel,
  MenuItem,
  Select,
  Snackbar,
  Stack,
  TextField,
} from "@mui/material";
import { useNavigate } from "react-router-dom";
import { useStores } from "../stores/StoreContext";
import type { TicketDetail, DepartmentOption } from "../stores/StaffTicketStore";

/** Staff permissions from the auth profile. */
export interface StaffPermissions {
  canAssignTickets?: boolean;
  canTransferTickets?: boolean;
  canCloseTickets?: boolean;
  canCreateTickets?: boolean;
}

interface WorkflowToolbarProps {
  ticket: TicketDetail;
  permissions: StaffPermissions;
  /** Current staff member's ID (for determining if assigned to self). */
  currentStaffId?: number;
  /** Departments the staff can transfer to (excludes current). */
  departments?: DepartmentOption[];
  /** Assignees (staff and teams) for the assign dialog. */
  assignees?: Array<{ id: string; name: string; type: "staff" | "team" }>;
}

/**
 * Workflow toolbar component with action buttons and dialogs.
 * @implements TS-M3-C4 AC-1 through AC-17
 */
export const WorkflowToolbar = observer(function WorkflowToolbar({
  ticket,
  permissions,
  currentStaffId: _currentStaffId,
  departments = [],
  assignees = [],
}: WorkflowToolbarProps) {
  const { staffTickets } = useStores();
  const navigate = useNavigate();

  // Dialog open states
  const [assignOpen, setAssignOpen] = useState(false);
  const [transferOpen, setTransferOpen] = useState(false);
  const [closeOpen, setCloseOpen] = useState(false);
  const [reopenOpen, setReopenOpen] = useState(false);

  // Dialog form states
  const [assignee, setAssignee] = useState("");
  const [assignComments, setAssignComments] = useState("");
  const [transferDept, setTransferDept] = useState<number | "">("");
  const [transferComments, setTransferComments] = useState("");
  const [closeComments, setCloseComments] = useState("");
  const [reopenComments, setReopenComments] = useState("");

  // Toast state
  const [toast, setToast] = useState<{ open: boolean; message: string; severity: "success" | "error" }>({
    open: false,
    message: "",
    severity: "success",
  });

  const isOpen = ticket.status === "open";
  const isClosed = ticket.status === "closed";
  const isUnassigned = !ticket.staffId && !ticket.teamId;

  // Visibility rules per AC
  const showClaim =
    isOpen && isUnassigned && permissions.canAssignTickets;
  const showAssign = permissions.canAssignTickets; // Can assign open or closed (reopens)
  const showTransfer = permissions.canTransferTickets;
  const showClose = isOpen && permissions.canCloseTickets;
  const showReopen =
    isClosed && (permissions.canCloseTickets || permissions.canCreateTickets);

  const handleCloseToast = () => setToast({ ...toast, open: false });

  const showSuccess = (message: string) => {
    setToast({ open: true, message, severity: "success" });
  };

  const showError = (message: string) => {
    setToast({ open: true, message, severity: "error" });
  };

  // --- Claim ---
  const handleClaim = async () => {
    const success = await staffTickets.claimTicket(ticket.id);
    if (success) {
      showSuccess("Ticket is now assigned to you!");
    } else if (staffTickets.workflowError) {
      showError(staffTickets.workflowError);
    }
  };

  // --- Assign ---
  const openAssign = () => {
    setAssignee("");
    setAssignComments("");
    setAssignOpen(true);
  };

  const handleAssign = async () => {
    if (assignComments.trim().length < 5) {
      showError("Comment too short (min 5 chars)");
      return;
    }
    const success = await staffTickets.assignTicket(
      ticket.id,
      assignee,
      assignComments.trim(),
    );
    if (success) {
      setAssignOpen(false);
      showSuccess("Ticket assigned successfully");
      // Navigate back to queue (non-claim assignment)
      navigate("/staff/tickets");
    } else if (staffTickets.workflowError) {
      showError(staffTickets.workflowError);
    }
  };

  // --- Transfer ---
  const openTransfer = () => {
    setTransferDept("");
    setTransferComments("");
    setTransferOpen(true);
  };

  const handleTransfer = async () => {
    if (transferDept === "") {
      showError("Please select a department");
      return;
    }
    if (transferComments.trim().length < 5) {
      showError("Transfer comments too short!");
      return;
    }
    const result = await staffTickets.transferTicket(
      ticket.id,
      transferDept as number,
      transferComments.trim(),
    );
    if (result.success) {
      setTransferOpen(false);
      const deptName = departments.find((d) => d.id === transferDept)?.name ?? "new department";
      showSuccess(`Ticket transferred successfully to ${deptName}`);
      if (result.accessLost) {
        // Redirect when access is lost
        navigate("/staff/tickets");
      }
    } else if (staffTickets.workflowError) {
      showError(staffTickets.workflowError);
    }
  };

  // --- Close ---
  const openClose = () => {
    setCloseComments("");
    setCloseOpen(true);
  };

  const handleClose = async () => {
    const success = await staffTickets.closeTicket(
      ticket.id,
      closeComments.trim() || undefined,
    );
    if (success) {
      setCloseOpen(false);
      showSuccess(staffTickets.workflowMessage ?? "Ticket closed");
      // Navigate back to queue
      navigate("/staff/tickets");
    } else if (staffTickets.workflowError) {
      showError(staffTickets.workflowError);
    }
  };

  // --- Reopen ---
  const openReopen = () => {
    setReopenComments("");
    setReopenOpen(true);
  };

  const handleReopen = async () => {
    const success = await staffTickets.reopenTicket(
      ticket.id,
      reopenComments.trim() || undefined,
    );
    if (success) {
      setReopenOpen(false);
      showSuccess("Ticket REOPENED");
      // Stay on detail view, ticket will refresh
    } else if (staffTickets.workflowError) {
      showError(staffTickets.workflowError);
    }
  };

  // Filter out current department from transfer options
  const availableDepts = departments.filter((d) => d.id !== ticket.deptId);

  return (
    <>
      <Stack direction="row" spacing={1} sx={{ mb: 2 }} data-testid="workflow-toolbar">
        {showClaim && (
          <Button
            variant="contained"
            color="primary"
            onClick={handleClaim}
            disabled={staffTickets.workflowLoading}
            data-testid="claim-button"
          >
            Claim
          </Button>
        )}
        {showAssign && (
          <Button
            variant="outlined"
            onClick={openAssign}
            disabled={staffTickets.workflowLoading}
            data-testid="assign-button"
          >
            Assign
          </Button>
        )}
        {showTransfer && (
          <Button
            variant="outlined"
            onClick={openTransfer}
            disabled={staffTickets.workflowLoading}
            data-testid="transfer-button"
          >
            Transfer
          </Button>
        )}
        {showClose && (
          <Button
            variant="outlined"
            color="warning"
            onClick={openClose}
            disabled={staffTickets.workflowLoading}
            data-testid="close-button"
          >
            Close
          </Button>
        )}
        {showReopen && (
          <Button
            variant="outlined"
            color="success"
            onClick={openReopen}
            disabled={staffTickets.workflowLoading}
            data-testid="reopen-button"
          >
            Reopen
          </Button>
        )}

        {/* Show Assigned To field */}
        {ticket.assignedToName && (
          <Box sx={{ ml: 2, display: "flex", alignItems: "center" }}>
            <strong>Assigned To:</strong>&nbsp;{ticket.assignedToName}
          </Box>
        )}
        {isUnassigned && (
          <Box sx={{ ml: 2, display: "flex", alignItems: "center", color: "text.secondary" }}>
            <strong>Assigned To:</strong>&nbsp;Unassigned
          </Box>
        )}
      </Stack>

      {/* Assign Dialog */}
      <Dialog
        open={assignOpen}
        onClose={() => setAssignOpen(false)}
        maxWidth="sm"
        fullWidth
        disableRestoreFocus
        data-testid="assign-dialog"
      >
        <DialogTitle>Assign Ticket</DialogTitle>
        <DialogContent>
          <Stack spacing={2} sx={{ mt: 1 }}>
            <FormControl fullWidth required>
              <InputLabel id="assignee-label">Assignee</InputLabel>
              <Select
                labelId="assignee-label"
                label="Assignee"
                value={assignee}
                onChange={(e) => setAssignee(e.target.value)}
                data-testid="assignee-select"
              >
                {assignees.length === 0 && (
                  <MenuItem value="" disabled>
                    No assignees available
                  </MenuItem>
                )}
                {assignees.map((a) => (
                  <MenuItem key={a.id} value={a.id}>
                    {a.name} ({a.type === "staff" ? "Staff" : "Team"})
                  </MenuItem>
                ))}
              </Select>
            </FormControl>
            <TextField
              label="Comments"
              multiline
              rows={3}
              fullWidth
              required
              value={assignComments}
              onChange={(e) => setAssignComments(e.target.value)}
              error={assignComments.length > 0 && assignComments.length < 5}
              helperText={
                assignComments.length > 0 && assignComments.length < 5
                  ? "Comment too short (min 5 chars)"
                  : "Required, minimum 5 characters"
              }
              inputProps={{ "data-testid": "assign-comments" }}
            />
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setAssignOpen(false)}>Cancel</Button>
          <Button
            variant="contained"
            onClick={handleAssign}
            disabled={!assignee || assignComments.length < 5 || staffTickets.workflowLoading}
            data-testid="assign-submit"
          >
            Assign
          </Button>
        </DialogActions>
      </Dialog>

      {/* Transfer Dialog */}
      <Dialog
        open={transferOpen}
        onClose={() => setTransferOpen(false)}
        maxWidth="sm"
        fullWidth
        disableRestoreFocus
        data-testid="transfer-dialog"
      >
        <DialogTitle>Transfer Ticket</DialogTitle>
        <DialogContent>
          <Stack spacing={2} sx={{ mt: 1 }}>
            <FormControl fullWidth required>
              <InputLabel id="dept-label">Department</InputLabel>
              <Select
                labelId="dept-label"
                label="Department"
                value={transferDept}
                onChange={(e) => setTransferDept(e.target.value as number)}
                data-testid="dept-select"
              >
                {availableDepts.length === 0 && (
                  <MenuItem value="" disabled>
                    No other departments available
                  </MenuItem>
                )}
                {availableDepts.map((d) => (
                  <MenuItem key={d.id} value={d.id}>
                    {d.name}
                  </MenuItem>
                ))}
              </Select>
              {ticket.deptName && (
                <FormHelperText>Current: {ticket.deptName}</FormHelperText>
              )}
            </FormControl>
            <TextField
              label="Comments"
              multiline
              rows={3}
              fullWidth
              required
              value={transferComments}
              onChange={(e) => setTransferComments(e.target.value)}
              error={transferComments.length > 0 && transferComments.length < 5}
              helperText={
                transferComments.length > 0 && transferComments.length < 5
                  ? "Transfer comments too short!"
                  : "Required, minimum 5 characters"
              }
              inputProps={{ "data-testid": "transfer-comments" }}
            />
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setTransferOpen(false)}>Cancel</Button>
          <Button
            variant="contained"
            onClick={handleTransfer}
            disabled={
              transferDept === "" ||
              transferComments.length < 5 ||
              staffTickets.workflowLoading
            }
            data-testid="transfer-submit"
          >
            Transfer
          </Button>
        </DialogActions>
      </Dialog>

      {/* Close Dialog */}
      <Dialog
        open={closeOpen}
        onClose={() => setCloseOpen(false)}
        maxWidth="sm"
        fullWidth
        disableRestoreFocus
        data-testid="close-dialog"
      >
        <DialogTitle>Close Ticket</DialogTitle>
        <DialogContent>
          <Stack spacing={2} sx={{ mt: 1 }}>
            <TextField
              label="Comments (optional)"
              multiline
              rows={3}
              fullWidth
              value={closeComments}
              onChange={(e) => setCloseComments(e.target.value)}
              inputProps={{ "data-testid": "close-comments" }}
            />
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setCloseOpen(false)}>Cancel</Button>
          <Button
            variant="contained"
            color="warning"
            onClick={handleClose}
            disabled={staffTickets.workflowLoading}
            data-testid="close-submit"
          >
            Close Ticket
          </Button>
        </DialogActions>
      </Dialog>

      {/* Reopen Dialog */}
      <Dialog
        open={reopenOpen}
        onClose={() => setReopenOpen(false)}
        maxWidth="sm"
        fullWidth
        disableRestoreFocus
        data-testid="reopen-dialog"
      >
        <DialogTitle>Reopen Ticket</DialogTitle>
        <DialogContent>
          <Stack spacing={2} sx={{ mt: 1 }}>
            <TextField
              label="Comments (optional)"
              multiline
              rows={3}
              fullWidth
              value={reopenComments}
              onChange={(e) => setReopenComments(e.target.value)}
              inputProps={{ "data-testid": "reopen-comments" }}
            />
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setReopenOpen(false)}>Cancel</Button>
          <Button
            variant="contained"
            color="success"
            onClick={handleReopen}
            disabled={staffTickets.workflowLoading}
            data-testid="reopen-submit"
          >
            Reopen Ticket
          </Button>
        </DialogActions>
      </Dialog>

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
