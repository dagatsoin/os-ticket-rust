/**
 * Ticket edit form component.
 * Allows staff to edit ticket properties with validation.
 *
 * @implements TS-M3-I4: edit form UI
 */
import { useState, useEffect } from "react";
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
  InputLabel,
  MenuItem,
  Select,
  Snackbar,
  Stack,
  TextField,
  Tooltip,
} from "@mui/material";
import { useStores } from "../stores/StoreContext";
import type { TicketDetail } from "../stores/StaffTicketStore";

/** Department option. */
export interface DeptOption {
  id: number;
  name: string;
}

/** Help topic option. */
export interface TopicOption {
  id: number;
  name: string;
}

/** Priority option. */
export interface PriorityOption {
  id: number;
  name: string;
}

/** SLA option. */
export interface SlaOption {
  id: number;
  name: string;
}

export interface TicketEditFormProps {
  /** Whether dialog is open. */
  open: boolean;
  /** Called to close the dialog. */
  onClose: () => void;
  /** Current ticket detail. */
  ticket: TicketDetail;
  /** Called after successful update. */
  onSuccess: () => void;
  /** Available departments. */
  departments: DeptOption[];
  /** Available topics. */
  topics: TopicOption[];
  /** Available priorities. */
  priorities: PriorityOption[];
  /** Available SLAs. */
  slas: SlaOption[];
}

const SOURCE_OPTIONS = [
  { value: "Phone", label: "Phone" },
  { value: "Email", label: "Email" },
  { value: "Web", label: "Web" },
  { value: "API", label: "API" },
  { value: "Other", label: "Other" },
];

/**
 * Ticket edit form with validation.
 * @implements TS-M3-I4 AC-1: Edit button shown for canEditTickets
 * @implements TS-M3-I4 AC-2: Form loads current values
 * @implements TS-M3-I4 AC-3: Reason required validation
 * @implements TS-M3-I4 AC-4: Due date must be in future
 * @implements TS-M3-I4 AC-5: Due date disabled on closed ticket
 * @implements TS-M3-I4 AC-6: Success redirect
 * @implements TS-M3-I4 AC-7: Source dropdown options
 * @implements TS-M3-I4 AC-8: Dropdowns from API
 */
export const TicketEditForm = observer(function TicketEditForm({
  open,
  onClose,
  ticket,
  onSuccess,
  departments,
  topics,
  priorities,
  slas,
}: TicketEditFormProps) {
  const { staffTickets } = useStores();

  // Form state
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [phone, setPhone] = useState("");
  const [phoneExt, setPhoneExt] = useState("");
  const [deptId, setDeptId] = useState<number | "">("");
  const [topicId, setTopicId] = useState<number | "">("");
  const [priorityId, setPriorityId] = useState<number | "">("");
  const [slaId, setSlaId] = useState<number | "">("");
  const [source, setSource] = useState("");
  const [duedate, setDuedate] = useState("");
  const [reason, setReason] = useState("");

  // Validation errors
  const [emailError, setEmailError] = useState("");
  const [reasonError, setReasonError] = useState("");
  const [duedateError, setDuedateError] = useState("");

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

  const isClosed = ticket.status === "closed";

  // Load current values when dialog opens
  useEffect(() => {
    if (open) {
      setName(ticket.name || "");
      setEmail(ticket.email || "");
      setPhone("");
      setPhoneExt("");
      setDeptId(ticket.deptId ?? "");
      setTopicId("");
      setPriorityId("");
      setSlaId(ticket.slaId ?? "");
      setSource("");
      setDuedate(ticket.duedate ? ticket.duedate.split("T")[0] : "");
      setReason("");
      // Clear errors
      setEmailError("");
      setReasonError("");
      setDuedateError("");
    }
  }, [open, ticket]);

  const handleCloseToast = () => setToast({ ...toast, open: false });

  const validateEmail = (value: string): boolean => {
    if (value && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value)) {
      setEmailError("Invalid email format");
      return false;
    }
    setEmailError("");
    return true;
  };

  const validateReason = (): boolean => {
    if (!reason.trim()) {
      setReasonError("Reason for the update required");
      return false;
    }
    setReasonError("");
    return true;
  };

  const validateDuedate = (value: string): boolean => {
    if (value && !isClosed) {
      const selected = new Date(value);
      const today = new Date();
      today.setHours(0, 0, 0, 0);
      if (selected < today) {
        setDuedateError("Due date must be in the future");
        return false;
      }
    }
    setDuedateError("");
    return true;
  };

  const handleSubmit = async () => {
    // Validate all fields
    const isEmailValid = validateEmail(email);
    const isReasonValid = validateReason();
    const isDuedateValid = validateDuedate(duedate);

    if (!isEmailValid || !isReasonValid || !isDuedateValid) {
      return;
    }

    const success = await staffTickets.updateTicket(ticket.id, {
      name: name || undefined,
      email: email || undefined,
      phone: phone || undefined,
      phone_ext: phoneExt || undefined,
      dept_id: deptId || undefined,
      topic_id: topicId || undefined,
      priority_id: priorityId || undefined,
      sla_id: slaId || undefined,
      source: source || undefined,
      duedate: duedate || undefined,
      reason: reason.trim(),
    });

    if (success) {
      setToast({
        open: true,
        message: "Ticket updated successfully",
        severity: "success",
      });
      onClose();
      onSuccess();
    } else if (staffTickets.updateError) {
      setToast({
        open: true,
        message: staffTickets.updateError,
        severity: "error",
      });
    }
  };

  return (
    <>
      <Dialog
        open={open}
        onClose={onClose}
        maxWidth="md"
        fullWidth
        data-testid="edit-form-dialog"
      >
        <DialogTitle>Edit Ticket #{ticket.number}</DialogTitle>
        <DialogContent>
          <Stack spacing={2} sx={{ mt: 1 }}>
            {staffTickets.updateError && (
              <Alert severity="error">{staffTickets.updateError}</Alert>
            )}

            {/* Contact Information */}
            <Stack direction="row" spacing={2}>
              <TextField
                label="Name"
                fullWidth
                value={name}
                onChange={(e) => setName(e.target.value)}
                inputProps={{ "data-testid": "edit-name" }}
              />
              <TextField
                label="Email"
                fullWidth
                value={email}
                onChange={(e) => {
                  setEmail(e.target.value);
                  validateEmail(e.target.value);
                }}
                error={Boolean(emailError)}
                helperText={emailError}
                inputProps={{ "data-testid": "edit-email" }}
              />
            </Stack>

            <Stack direction="row" spacing={2}>
              <TextField
                label="Phone"
                fullWidth
                value={phone}
                onChange={(e) => setPhone(e.target.value)}
                inputProps={{ "data-testid": "edit-phone" }}
              />
              <TextField
                label="Phone Ext"
                sx={{ width: 150 }}
                value={phoneExt}
                onChange={(e) => setPhoneExt(e.target.value)}
                inputProps={{ "data-testid": "edit-phone-ext" }}
              />
            </Stack>

            {/* Ticket Properties */}
            <Stack direction="row" spacing={2}>
              <FormControl fullWidth>
                <InputLabel id="edit-dept-label">Department</InputLabel>
                <Select
                  labelId="edit-dept-label"
                  label="Department"
                  value={deptId}
                  onChange={(e) => setDeptId(e.target.value as number | "")}
                  data-testid="edit-department"
                >
                  <MenuItem value="">
                    <em>No change</em>
                  </MenuItem>
                  {departments.map((d) => (
                    <MenuItem key={d.id} value={d.id}>
                      {d.name}
                    </MenuItem>
                  ))}
                </Select>
              </FormControl>

              <FormControl fullWidth>
                <InputLabel id="edit-topic-label">Help Topic</InputLabel>
                <Select
                  labelId="edit-topic-label"
                  label="Help Topic"
                  value={topicId}
                  onChange={(e) => setTopicId(e.target.value as number | "")}
                  data-testid="edit-topic"
                >
                  <MenuItem value="">
                    <em>No change</em>
                  </MenuItem>
                  {topics.map((t) => (
                    <MenuItem key={t.id} value={t.id}>
                      {t.name}
                    </MenuItem>
                  ))}
                </Select>
              </FormControl>
            </Stack>

            <Stack direction="row" spacing={2}>
              <FormControl fullWidth>
                <InputLabel id="edit-priority-label">Priority</InputLabel>
                <Select
                  labelId="edit-priority-label"
                  label="Priority"
                  value={priorityId}
                  onChange={(e) => setPriorityId(e.target.value as number | "")}
                  data-testid="edit-priority"
                >
                  <MenuItem value="">
                    <em>No change</em>
                  </MenuItem>
                  {priorities.map((p) => (
                    <MenuItem key={p.id} value={p.id}>
                      {p.name}
                    </MenuItem>
                  ))}
                </Select>
              </FormControl>

              <FormControl fullWidth>
                <InputLabel id="edit-sla-label">SLA Plan</InputLabel>
                <Select
                  labelId="edit-sla-label"
                  label="SLA Plan"
                  value={slaId}
                  onChange={(e) => setSlaId(e.target.value as number | "")}
                  data-testid="edit-sla"
                >
                  <MenuItem value="">
                    <em>No change</em>
                  </MenuItem>
                  {slas.map((s) => (
                    <MenuItem key={s.id} value={s.id}>
                      {s.name}
                    </MenuItem>
                  ))}
                </Select>
              </FormControl>
            </Stack>

            <Stack direction="row" spacing={2}>
              <FormControl fullWidth>
                <InputLabel id="edit-source-label">Source</InputLabel>
                <Select
                  labelId="edit-source-label"
                  label="Source"
                  value={source}
                  onChange={(e) => setSource(e.target.value)}
                  data-testid="edit-source"
                >
                  <MenuItem value="">
                    <em>No change</em>
                  </MenuItem>
                  {SOURCE_OPTIONS.map((s) => (
                    <MenuItem key={s.value} value={s.value}>
                      {s.label}
                    </MenuItem>
                  ))}
                </Select>
              </FormControl>

              <Tooltip title={isClosed ? "Cannot set due date on closed ticket" : ""}>
                <Box sx={{ width: "100%" }}>
                  <TextField
                    label="Due Date"
                    type="date"
                    fullWidth
                    value={duedate}
                    onChange={(e) => {
                      setDuedate(e.target.value);
                      validateDuedate(e.target.value);
                    }}
                    disabled={isClosed}
                    error={Boolean(duedateError)}
                    helperText={duedateError || (isClosed ? "Cannot set due date on closed ticket" : "")}
                    InputLabelProps={{ shrink: true }}
                    inputProps={{ "data-testid": "edit-duedate" }}
                  />
                </Box>
              </Tooltip>
            </Stack>

            {/* Reason (required) */}
            <TextField
              label="Reason for Update"
              multiline
              rows={3}
              fullWidth
              required
              value={reason}
              onChange={(e) => {
                setReason(e.target.value);
                if (e.target.value.trim()) setReasonError("");
              }}
              error={Boolean(reasonError)}
              helperText={reasonError || "Required - explain why you are making this change"}
              inputProps={{ "data-testid": "edit-reason" }}
            />
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={onClose}>Cancel</Button>
          <Button
            variant="contained"
            onClick={handleSubmit}
            disabled={staffTickets.updateLoading}
            data-testid="edit-save"
          >
            {staffTickets.updateLoading ? "Saving..." : "Save"}
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
