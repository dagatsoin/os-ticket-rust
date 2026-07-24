/**
 * Delete confirmation dialog component.
 * Shows warning that deletion is permanent before allowing delete.
 *
 * @implements TS-M3-I5: delete confirmation dialog
 */
import { useState } from "react";
import { observer } from "mobx-react-lite";
import {
  Alert,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  Snackbar,
} from "@mui/material";
import { useNavigate } from "react-router-dom";
import { useStores } from "../stores/StoreContext";

export interface DeleteConfirmDialogProps {
  /** Whether dialog is open. */
  open: boolean;
  /** Called to close the dialog. */
  onClose: () => void;
  /** Ticket ID to delete. */
  ticketId: number;
  /** Ticket number for display. */
  ticketNumber: number;
}

/**
 * Delete confirmation dialog with permanent deletion warning.
 * @implements TS-M3-I5 AC-1: Delete button shown for canDeleteTickets
 * @implements TS-M3-I5 AC-2: Dialog with permanent warning
 * @implements TS-M3-I5 AC-3: Cancel closes without action
 * @implements TS-M3-I5 AC-4: Confirm deletes and redirects
 * @implements TS-M3-I5 AC-5: Error shows message
 * @implements TS-M3-I5 AC-6: Keyboard accessible
 */
export const DeleteConfirmDialog = observer(function DeleteConfirmDialog({
  open,
  onClose,
  ticketId,
  ticketNumber,
}: DeleteConfirmDialogProps) {
  const { staffTickets } = useStores();
  const navigate = useNavigate();

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

  const handleDelete = async () => {
    const success = await staffTickets.deleteTicket(ticketId);

    if (success) {
      setToast({
        open: true,
        message: `Ticket #${ticketNumber} deleted successfully`,
        severity: "success",
      });
      onClose();
      // Navigate to queue after short delay to show toast
      setTimeout(() => navigate("/staff/tickets"), 500);
    } else if (staffTickets.deleteError) {
      setToast({
        open: true,
        message: staffTickets.deleteError,
        severity: "error",
      });
      onClose();
    }
  };

  return (
    <>
      <Dialog
        open={open}
        onClose={onClose}
        aria-labelledby="delete-dialog-title"
        aria-describedby="delete-dialog-description"
        data-testid="delete-confirm-dialog"
      >
        <DialogTitle id="delete-dialog-title">Delete Ticket</DialogTitle>
        <DialogContent>
          <DialogContentText id="delete-dialog-description">
            Are you sure you want to delete ticket #{ticketNumber}?
            <br />
            <br />
            <strong>This action is irreversible.</strong> The ticket and all attachments
            will be <strong>permanently deleted</strong> and cannot be recovered.
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button
            onClick={onClose}
            autoFocus
            data-testid="delete-cancel"
          >
            Cancel
          </Button>
          <Button
            variant="contained"
            color="error"
            onClick={handleDelete}
            disabled={staffTickets.deleteLoading}
            data-testid="delete-confirm"
          >
            {staffTickets.deleteLoading ? "Deleting..." : "Delete"}
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
