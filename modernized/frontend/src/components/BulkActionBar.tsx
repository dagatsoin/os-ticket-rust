/**
 * Bulk action bar component for ticket queue.
 * Shows action buttons (Close/Reopen/Delete) based on queue status and permissions.
 *
 * @implements TS-M3-G2: bulk action bar UI
 */
import { useState } from "react";
import { observer } from "mobx-react-lite";
import {
  Alert,
  Box,
  Button,
  Checkbox,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  Snackbar,
  Stack,
  Typography,
} from "@mui/material";
import { Delete, Lock, LockOpen } from "@mui/icons-material";
import { useStores } from "../stores/StoreContext";
import type { QueueStatus } from "../stores/StaffTicketStore";

export interface BulkActionBarProps {
  /** Current queue status. */
  queueStatus: QueueStatus;
  /** Whether staff can close tickets. */
  canCloseTickets: boolean;
  /** Whether staff can delete tickets. */
  canDeleteTickets: boolean;
  /** Called after a successful bulk action to refresh the queue. */
  onActionComplete: () => void;
}

/**
 * Bulk action bar with Close/Reopen/Delete buttons.
 * @implements TS-M3-G2 AC-1: Checkboxes only for canManageTickets staff
 * @implements TS-M3-G2 AC-2: Close on open queue, Reopen on closed
 * @implements TS-M3-G2 AC-4: Bulk close flow with confirmation
 * @implements TS-M3-G2 AC-5: Bulk delete with permanent warning
 * @implements TS-M3-G2 AC-6: Disabled when no tickets selected
 */
export const BulkActionBar = observer(function BulkActionBar({
  queueStatus,
  canCloseTickets,
  canDeleteTickets,
  onActionComplete,
}: BulkActionBarProps) {
  const { staffTickets } = useStores();

  // Dialog states
  const [closeDialogOpen, setCloseDialogOpen] = useState(false);
  const [reopenDialogOpen, setReopenDialogOpen] = useState(false);
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);

  // Toast state
  const [toast, setToast] = useState<{
    open: boolean;
    message: string;
    severity: "success" | "warning" | "error";
  }>({
    open: false,
    message: "",
    severity: "success",
  });

  const selectedCount = staffTickets.selectedTicketIds.size;
  const hasSelection = selectedCount > 0;

  // Show Close on open/answered/overdue/assigned queues, Reopen on closed
  const showClose = queueStatus !== "closed" && canCloseTickets;
  const showReopen = queueStatus === "closed" && canCloseTickets;
  const showDelete = canDeleteTickets;

  const handleCloseToast = () => setToast({ ...toast, open: false });

  const handleSelectAllChange = () => {
    if (staffTickets.allSelected) {
      staffTickets.deselectAllTickets();
    } else {
      staffTickets.selectAllTickets();
    }
  };

  // Bulk Close
  const handleCloseConfirm = async () => {
    setCloseDialogOpen(false);
    const ids = Array.from(staffTickets.selectedTicketIds);
    const result = await staffTickets.bulkAction("close", ids);
    if (result.success) {
      if (result.affected === result.total) {
        setToast({
          open: true,
          message: `${result.affected} tickets closed`,
          severity: "success",
        });
      } else {
        setToast({
          open: true,
          message: `${result.affected} of ${result.total} tickets closed`,
          severity: "warning",
        });
      }
      onActionComplete();
    } else if (staffTickets.bulkError) {
      setToast({
        open: true,
        message: staffTickets.bulkError,
        severity: "error",
      });
    }
  };

  // Bulk Reopen
  const handleReopenConfirm = async () => {
    setReopenDialogOpen(false);
    const ids = Array.from(staffTickets.selectedTicketIds);
    const result = await staffTickets.bulkAction("reopen", ids);
    if (result.success) {
      if (result.affected === result.total) {
        setToast({
          open: true,
          message: `${result.affected} tickets reopened`,
          severity: "success",
        });
      } else {
        setToast({
          open: true,
          message: `${result.affected} of ${result.total} tickets reopened`,
          severity: "warning",
        });
      }
      onActionComplete();
    } else if (staffTickets.bulkError) {
      setToast({
        open: true,
        message: staffTickets.bulkError,
        severity: "error",
      });
    }
  };

  // Bulk Delete
  const handleDeleteConfirm = async () => {
    setDeleteDialogOpen(false);
    const ids = Array.from(staffTickets.selectedTicketIds);
    const result = await staffTickets.bulkAction("delete", ids);
    if (result.success) {
      setToast({
        open: true,
        message: `${result.affected} tickets deleted`,
        severity: "success",
      });
      onActionComplete();
    } else if (staffTickets.bulkError) {
      setToast({
        open: true,
        message: staffTickets.bulkError,
        severity: "error",
      });
    }
  };

  // Don't render if no permissions
  if (!canCloseTickets && !canDeleteTickets) {
    return null;
  }

  return (
    <>
      <Box
        sx={{
          display: "flex",
          alignItems: "center",
          gap: 2,
          p: 1,
          bgcolor: "grey.100",
          borderRadius: 1,
        }}
        data-testid="bulk-action-bar"
      >
        {/* Select All Checkbox */}
        <Checkbox
          checked={staffTickets.allSelected}
          indeterminate={staffTickets.someSelected}
          onChange={handleSelectAllChange}
          inputProps={{ "aria-label": "Select all tickets" }}
          data-testid="select-all-checkbox"
        />
        <Typography variant="body2" color="text.secondary">
          {hasSelection ? `${selectedCount} selected` : "Select tickets"}
        </Typography>

        <Box sx={{ flexGrow: 1 }} />

        <Stack direction="row" spacing={1}>
          {showClose && (
            <Button
              variant="outlined"
              size="small"
              disabled={!hasSelection || staffTickets.bulkLoading}
              onClick={() => setCloseDialogOpen(true)}
              startIcon={<Lock />}
              data-testid="bulk-close-button"
            >
              Close
            </Button>
          )}
          {showReopen && (
            <Button
              variant="outlined"
              size="small"
              color="success"
              disabled={!hasSelection || staffTickets.bulkLoading}
              onClick={() => setReopenDialogOpen(true)}
              startIcon={<LockOpen />}
              data-testid="bulk-reopen-button"
            >
              Reopen
            </Button>
          )}
          {showDelete && (
            <Button
              variant="outlined"
              size="small"
              color="error"
              disabled={!hasSelection || staffTickets.bulkLoading}
              onClick={() => setDeleteDialogOpen(true)}
              startIcon={<Delete />}
              data-testid="bulk-delete-button"
            >
              Delete
            </Button>
          )}
        </Stack>
      </Box>

      {/* Close Confirmation Dialog */}
      <Dialog open={closeDialogOpen} onClose={() => setCloseDialogOpen(false)}>
        <DialogTitle>Close Tickets</DialogTitle>
        <DialogContent>
          <DialogContentText>
            Are you sure you want to close {selectedCount} ticket(s)?
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setCloseDialogOpen(false)}>Cancel</Button>
          <Button
            variant="contained"
            onClick={handleCloseConfirm}
            data-testid="bulk-close-confirm"
          >
            Confirm
          </Button>
        </DialogActions>
      </Dialog>

      {/* Reopen Confirmation Dialog */}
      <Dialog open={reopenDialogOpen} onClose={() => setReopenDialogOpen(false)}>
        <DialogTitle>Reopen Tickets</DialogTitle>
        <DialogContent>
          <DialogContentText>
            Are you sure you want to reopen {selectedCount} ticket(s)?
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setReopenDialogOpen(false)}>Cancel</Button>
          <Button
            variant="contained"
            color="success"
            onClick={handleReopenConfirm}
            data-testid="bulk-reopen-confirm"
          >
            Confirm
          </Button>
        </DialogActions>
      </Dialog>

      {/* Delete Confirmation Dialog */}
      <Dialog open={deleteDialogOpen} onClose={() => setDeleteDialogOpen(false)}>
        <DialogTitle>Delete Tickets</DialogTitle>
        <DialogContent>
          <DialogContentText>
            Are you sure you want to delete {selectedCount} ticket(s)?
            <br />
            <strong>This action is permanent and cannot be recovered.</strong>
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setDeleteDialogOpen(false)}>Cancel</Button>
          <Button
            variant="contained"
            color="error"
            onClick={handleDeleteConfirm}
            data-testid="bulk-delete-confirm"
          >
            Delete
          </Button>
        </DialogActions>
      </Dialog>

      {/* Toast Notifications */}
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
