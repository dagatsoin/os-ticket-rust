// SLA-plan management screen (TS-M4-D2) — FS-032.9/.10/.12. An SLA list (name,
// grace period, status, sortable Date Added) with mass activate/disable/delete,
// and a create/edit dialog (name, grace period, four boolean flags, notes). The
// default SLA is delete-protected: its checkbox is disabled and a Delete mass
// action over the default is blocked with a message (FS-032.12).
//
// @implements FS-032.9: SLA list + mass actions.
// @implements FS-032.10: create / edit an SLA plan.
// @implements FS-032.12: default SLA delete-protection surfaced in the UI.
// @implements KL-032.10: "Date Added" is sortable (modernised).
import { useEffect, useState, type FormEvent } from "react";
import { observer } from "mobx-react-lite";
import {
  Alert,
  Box,
  Button,
  Checkbox,
  Chip,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  FormControlLabel,
  Paper,
  Stack,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableRow,
  TableSortLabel,
  TextField,
  Tooltip,
  Typography,
} from "@mui/material";
import { useStores } from "../../stores/StoreContext";
import { DEFAULT_SLA_DELETE_MESSAGE, type SlaMassAction, type SlaSortKey } from "../../stores/SlaAdminStore";

const COLUMNS: Array<{ key: SlaSortKey; label: string }> = [
  { key: "name", label: "Name" },
  { key: "grace", label: "Grace Period (hrs)" },
  { key: "status", label: "Status" },
  { key: "created", label: "Date Added" },
];

export const SlaListPage = observer(function SlaListPage() {
  const { slaAdmin } = useStores();
  const [confirm, setConfirm] = useState<SlaMassAction | null>(null);

  useEffect(() => {
    void slaAdmin.loadList();
  }, [slaAdmin]);

  const runMass = async () => {
    if (!confirm) return;
    const action = confirm;
    setConfirm(null);
    await slaAdmin.massAction(action);
  };

  const deleteBlocked = slaAdmin.selectionHasDefault;

  return (
    <Box>
      <Stack direction="row" justifyContent="space-between" alignItems="center" sx={{ mb: 2 }}>
        <Typography variant="h5">SLA Plans</Typography>
        <Button variant="contained" onClick={() => slaAdmin.openCreate()} data-testid="add-sla">
          Add SLA Plan
        </Button>
      </Stack>

      <Paper
        variant="outlined"
        sx={{ display: "flex", alignItems: "center", gap: 2, p: 1, mb: 1 }}
        data-testid="sla-mass-bar"
      >
        <Checkbox
          checked={slaAdmin.allSelected}
          indeterminate={slaAdmin.someSelected}
          onChange={() => slaAdmin.toggleSelectAll()}
          inputProps={{ "aria-label": "Select all SLA plans" }}
        />
        <Typography variant="body2" color="text.secondary">
          {slaAdmin.selectedIds.length > 0 ? `${slaAdmin.selectedIds.length} selected` : "Select SLA plans"}
        </Typography>
        <Box sx={{ flexGrow: 1 }} />
        <Button size="small" disabled={slaAdmin.selectedIds.length === 0 || slaAdmin.massLoading} onClick={() => setConfirm("activate")} data-testid="sla-mass-activate">
          Activate
        </Button>
        <Button size="small" disabled={slaAdmin.selectedIds.length === 0 || slaAdmin.massLoading} onClick={() => setConfirm("disable")} data-testid="sla-mass-disable">
          Disable
        </Button>
        <Tooltip title={deleteBlocked ? DEFAULT_SLA_DELETE_MESSAGE : ""}>
          <span>
            <Button
              size="small"
              color="error"
              disabled={slaAdmin.selectedIds.length === 0 || slaAdmin.massLoading || deleteBlocked}
              onClick={() => setConfirm("delete")}
              data-testid="sla-mass-delete"
            >
              Delete
            </Button>
          </span>
        </Tooltip>
      </Paper>

      {slaAdmin.listError ? <Alert severity="error" sx={{ mb: 2 }}>{slaAdmin.listError}</Alert> : null}
      {slaAdmin.massError ? (
        <Alert severity="error" sx={{ mb: 2 }} data-testid="sla-mass-error">
          {slaAdmin.massError}
        </Alert>
      ) : null}

      <Paper variant="outlined">
        <Table size="small">
          <TableHead>
            <TableRow>
              <TableCell padding="checkbox" />
              {COLUMNS.map((c) => (
                <TableCell key={c.key} sortDirection={slaAdmin.sort === c.key ? (slaAdmin.order === "ASC" ? "asc" : "desc") : false}>
                  <TableSortLabel
                    active={slaAdmin.sort === c.key}
                    direction={slaAdmin.sort === c.key && slaAdmin.order === "DESC" ? "desc" : "asc"}
                    onClick={() => slaAdmin.setSort(c.key)}
                    data-testid={`sla-sort-${c.key}`}
                  >
                    {c.label}
                  </TableSortLabel>
                </TableCell>
              ))}
            </TableRow>
          </TableHead>
          <TableBody>
            {slaAdmin.sortedRows.length === 0 ? (
              <TableRow>
                <TableCell colSpan={COLUMNS.length + 1}>
                  <Typography color="text.secondary">
                    {slaAdmin.loadingList ? "Loading…" : "No SLA plans found."}
                  </Typography>
                </TableCell>
              </TableRow>
            ) : (
              slaAdmin.sortedRows.map((row) => (
                <TableRow key={row.id} hover data-testid="sla-row">
                  <TableCell padding="checkbox">
                    <Tooltip title={row.is_default ? DEFAULT_SLA_DELETE_MESSAGE : ""}>
                      <span>
                        <Checkbox
                          checked={slaAdmin.selected.has(row.id)}
                          disabled={row.is_default}
                          onChange={() => slaAdmin.toggleSelect(row.id)}
                          inputProps={{ "aria-label": `Select ${row.name}` }}
                          data-testid={`sla-check-${row.id}`}
                        />
                      </span>
                    </Tooltip>
                  </TableCell>
                  <TableCell>
                    <Button variant="text" size="small" onClick={() => slaAdmin.openEdit(row)} data-testid={`sla-edit-${row.id}`} sx={{ textTransform: "none" }}>
                      {row.name}
                    </Button>
                    {row.is_default ? <Chip size="small" label="Default" sx={{ ml: 1 }} data-testid={`sla-default-${row.id}`} /> : null}
                  </TableCell>
                  <TableCell data-testid={`sla-grace-${row.id}`}>{row.grace_period}</TableCell>
                  <TableCell>
                    <Chip size="small" label={row.isactive ? "Active" : "Disabled"} color={row.isactive ? "success" : "default"} variant="outlined" />
                  </TableCell>
                  <TableCell data-testid={`sla-created-${row.id}`}>
                    {row.created ? new Date(row.created).toLocaleDateString() : "—"}
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </Paper>

      <Dialog open={confirm !== null} onClose={() => setConfirm(null)}>
        <DialogTitle>Confirm {confirm}</DialogTitle>
        <DialogContent>
          <DialogContentText>
            {confirm === "delete"
              ? `Delete ${slaAdmin.selectedIds.length} SLA plan(s)?`
              : `${confirm === "disable" ? "Disable" : "Activate"} ${slaAdmin.selectedIds.length} SLA plan(s)?`}
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setConfirm(null)}>Cancel</Button>
          <Button variant="contained" color={confirm === "delete" ? "error" : "primary"} onClick={() => void runMass()} data-testid="sla-mass-confirm">
            Confirm
          </Button>
        </DialogActions>
      </Dialog>

      <SlaFormDialog />
    </Box>
  );
});

/** Create / edit SLA-plan dialog (TS-M4-D2). */
const SlaFormDialog = observer(function SlaFormDialog() {
  const { slaAdmin } = useStores();
  if (!slaAdmin.formOpen) return null;

  const onSubmit = async (e: FormEvent) => {
    e.preventDefault();
    await slaAdmin.save();
  };

  const checkbox = (
    label: string,
    checked: boolean,
    onChange: (v: boolean) => void,
    testid: string,
  ) => (
    <FormControlLabel
      control={<Checkbox checked={checked} onChange={(e) => onChange(e.target.checked)} data-testid={testid} />}
      label={label}
    />
  );

  return (
    <Dialog open onClose={() => slaAdmin.closeForm()} maxWidth="sm" fullWidth>
      <DialogTitle>{slaAdmin.isEditing ? "Edit SLA Plan" : "Add SLA Plan"}</DialogTitle>
      <Box component="form" onSubmit={onSubmit} noValidate>
        <DialogContent dividers>
          <Stack spacing={2}>
            <TextField
              label="Name"
              value={slaAdmin.name}
              onChange={(e) => slaAdmin.setName(e.target.value)}
              error={Boolean(slaAdmin.fieldError("name"))}
              helperText={slaAdmin.fieldError("name") ?? " "}
              fullWidth
              inputProps={{ "data-testid": "sla-name" }}
            />
            <TextField
              label="Grace Period (hours)"
              value={slaAdmin.gracePeriod}
              onChange={(e) => slaAdmin.setGracePeriod(e.target.value)}
              error={Boolean(slaAdmin.fieldError("grace_period"))}
              helperText={slaAdmin.fieldError("grace_period") ?? " "}
              fullWidth
              inputProps={{ "data-testid": "sla-grace", inputMode: "numeric" }}
            />
            <Stack>
              {checkbox("Active", slaAdmin.isactive, (v) => slaAdmin.setIsactive(v), "sla-active")}
              {checkbox("Enable priority escalation", slaAdmin.enablePriorityEscalation, (v) => slaAdmin.setEnablePriorityEscalation(v), "sla-escalation")}
              {checkbox("Transient (not saved on tickets)", slaAdmin.transient, (v) => slaAdmin.setTransient(v), "sla-transient")}
              {checkbox("Disable overdue alerts", slaAdmin.disableOverdueAlerts, (v) => slaAdmin.setDisableOverdueAlerts(v), "sla-no-alerts")}
            </Stack>
            <TextField
              label="Admin Notes"
              multiline
              rows={2}
              value={slaAdmin.notes}
              onChange={(e) => slaAdmin.setNotes(e.target.value)}
              fullWidth
              inputProps={{ "data-testid": "sla-notes" }}
            />
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => slaAdmin.closeForm()}>Cancel</Button>
          <Button type="submit" variant="contained" disabled={slaAdmin.saving} data-testid="sla-save">
            {slaAdmin.saving ? "Saving…" : "Save"}
          </Button>
        </DialogActions>
      </Box>
    </Dialog>
  );
});
