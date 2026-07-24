// Canned-response management screen (TS-M4-H2) — FS-022. Reachable via the
// `can_manage_premade` capability gate (NOT the admin gate). A list (title,
// department, enabled, attachment count) with mass enable/disable/delete, and a
// create/edit dialog (title, optional dept scope from the premade-gated
// dept-options endpoint, %{token} body, enabled flag, notes, attachments via the
// M2 AttachmentChip + validateAttachment as multipart, keep_file_ids on edit).
//
// @implements FS-022: canned-response admin CRUD surface.
// @implements BS-031-020: gated by can_manage_premade (delegated, non-admin).
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
  FormControl,
  FormControlLabel,
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
import { useStores } from "../../stores/StoreContext";
import { AttachmentChip } from "../../components/AttachmentChip";
import { ALLOWED_EXTENSIONS_LABEL, MAX_FILE_SIZE_LABEL } from "../../utils/validateAttachment";
import type { CannedMassAction } from "../../stores/CannedAdminStore";

export const CannedListPage = observer(function CannedListPage() {
  const { canned } = useStores();
  const [confirm, setConfirm] = useState<CannedMassAction | null>(null);

  useEffect(() => {
    void canned.loadList();
  }, [canned]);

  const runMass = async () => {
    if (!confirm) return;
    const action = confirm;
    setConfirm(null);
    await canned.massAction(action);
  };

  return (
    <Box>
      <Stack direction="row" justifyContent="space-between" alignItems="center" sx={{ mb: 2 }}>
        <Typography variant="h5">Canned Responses</Typography>
        <Button variant="contained" onClick={() => canned.openCreate()} data-testid="add-canned">
          Add Canned Response
        </Button>
      </Stack>

      <Paper variant="outlined" sx={{ display: "flex", alignItems: "center", gap: 2, p: 1, mb: 1 }} data-testid="canned-mass-bar">
        <Checkbox
          checked={canned.allSelected}
          indeterminate={canned.someSelected}
          onChange={() => canned.toggleSelectAll()}
          inputProps={{ "aria-label": "Select all canned responses" }}
        />
        <Typography variant="body2" color="text.secondary">
          {canned.selectedIds.length > 0 ? `${canned.selectedIds.length} selected` : "Select responses"}
        </Typography>
        <Box sx={{ flexGrow: 1 }} />
        <Button size="small" disabled={canned.selectedIds.length === 0 || canned.massLoading} onClick={() => setConfirm("enable")} data-testid="canned-mass-enable">
          Enable
        </Button>
        <Button size="small" disabled={canned.selectedIds.length === 0 || canned.massLoading} onClick={() => setConfirm("disable")} data-testid="canned-mass-disable">
          Disable
        </Button>
        <Button size="small" color="error" disabled={canned.selectedIds.length === 0 || canned.massLoading} onClick={() => setConfirm("delete")} data-testid="canned-mass-delete">
          Delete
        </Button>
      </Paper>

      {canned.listError ? <Alert severity="error" sx={{ mb: 2 }}>{canned.listError}</Alert> : null}

      <Paper variant="outlined">
        <Table size="small">
          <TableHead>
            <TableRow>
              <TableCell padding="checkbox" />
              <TableCell>Title</TableCell>
              <TableCell>Department</TableCell>
              <TableCell>Status</TableCell>
              <TableCell>Attachments</TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {canned.rows.length === 0 ? (
              <TableRow>
                <TableCell colSpan={5}>
                  <Typography color="text.secondary">
                    {canned.loadingList ? "Loading…" : "No canned responses found."}
                  </Typography>
                </TableCell>
              </TableRow>
            ) : (
              canned.rows.map((row) => (
                <TableRow key={row.id} hover data-testid="canned-row">
                  <TableCell padding="checkbox">
                    <Checkbox
                      checked={canned.selected.has(row.id)}
                      onChange={() => canned.toggleSelect(row.id)}
                      inputProps={{ "aria-label": `Select ${row.title}` }}
                      data-testid={`canned-check-${row.id}`}
                    />
                  </TableCell>
                  <TableCell>
                    <Button variant="text" size="small" onClick={() => void canned.openEdit(row.id)} data-testid={`canned-edit-${row.id}`} sx={{ textTransform: "none" }}>
                      {row.title}
                    </Button>
                  </TableCell>
                  <TableCell data-testid={`canned-dept-${row.id}`}>{row.dept_name ?? "— All —"}</TableCell>
                  <TableCell>
                    <Chip size="small" label={row.isenabled ? "Enabled" : "Disabled"} color={row.isenabled ? "success" : "default"} variant="outlined" />
                  </TableCell>
                  <TableCell data-testid={`canned-attcount-${row.id}`}>{row.attachment_count}</TableCell>
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
              ? `Delete ${canned.selectedIds.length} canned response(s)?`
              : `${confirm === "disable" ? "Disable" : "Enable"} ${canned.selectedIds.length} canned response(s)?`}
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setConfirm(null)}>Cancel</Button>
          <Button variant="contained" color={confirm === "delete" ? "error" : "primary"} onClick={() => void runMass()} data-testid="canned-mass-confirm">
            Confirm
          </Button>
        </DialogActions>
      </Dialog>

      <CannedFormDialog />
    </Box>
  );
});

/** Create / edit canned-response dialog (TS-M4-H2). */
const CannedFormDialog = observer(function CannedFormDialog() {
  const { canned } = useStores();
  if (!canned.formOpen) return null;

  const onSubmit = async (e: FormEvent) => {
    e.preventDefault();
    await canned.save();
  };

  return (
    <Dialog open onClose={() => canned.closeForm()} maxWidth="md" fullWidth>
      <DialogTitle>{canned.isEditing ? "Edit Canned Response" : "Add Canned Response"}</DialogTitle>
      <Box component="form" onSubmit={onSubmit} noValidate>
        <DialogContent dividers>
          <Stack spacing={2}>
            <Stack direction={{ xs: "column", sm: "row" }} spacing={2} alignItems={{ sm: "flex-start" }}>
              <TextField
                label="Title"
                value={canned.title}
                onChange={(e) => canned.setTitle(e.target.value)}
                error={Boolean(canned.fieldError("title"))}
                helperText={canned.fieldError("title") ?? " "}
                fullWidth
                inputProps={{ "data-testid": "canned-title" }}
              />
              <FormControl sx={{ minWidth: 180 }}>
                <InputLabel id="canned-dept-label">Department</InputLabel>
                <Select
                  labelId="canned-dept-label"
                  label="Department"
                  value={canned.deptId == null ? "" : String(canned.deptId)}
                  onChange={(e) => canned.setDeptId(e.target.value === "" ? null : Number(e.target.value))}
                  data-testid="canned-dept-select"
                >
                  <MenuItem value="">— All departments —</MenuItem>
                  {canned.deptOptions.map((d) => (
                    <MenuItem key={d.id} value={String(d.id)} data-testid={`canned-dept-option-${d.id}`}>
                      {d.name}
                    </MenuItem>
                  ))}
                </Select>
              </FormControl>
            </Stack>

            <TextField
              label="Response Body"
              multiline
              rows={8}
              value={canned.response}
              onChange={(e) => canned.setResponse(e.target.value)}
              error={Boolean(canned.fieldError("response"))}
              helperText={canned.fieldError("response") ?? "You can embed variables like %{ticket.name}."}
              fullWidth
              inputProps={{ "data-testid": "canned-body" }}
            />

            <FormControlLabel
              control={<Checkbox checked={canned.isenabled} onChange={(e) => canned.setIsenabled(e.target.checked)} data-testid="canned-enabled" />}
              label="Enabled"
            />

            {/* Attachments (multipart) */}
            <Box>
              <Typography variant="overline" color="text.secondary">
                Attachments
              </Typography>
              <Stack direction="row" spacing={1} flexWrap="wrap" useFlexGap sx={{ my: 1 }}>
                {canned.keptAttachments.map((att) => (
                  <AttachmentChip key={`keep-${att.id}`} label={att.name} onClick={() => canned.removeExisting(att.id)} />
                ))}
                {canned.stagedFiles.map((f, i) => (
                  <AttachmentChip key={`new-${i}`} label={f.name} onClick={() => canned.removeStagedFile(i)} />
                ))}
                {canned.keptAttachments.length === 0 && canned.stagedFiles.length === 0 ? (
                  <Typography variant="body2" color="text.secondary">
                    No attachments.
                  </Typography>
                ) : null}
              </Stack>
              <Button variant="outlined" component="label" size="small" data-testid="canned-file-button">
                Add attachment
                <input
                  type="file"
                  hidden
                  data-testid="canned-file-input"
                  onChange={(e) => {
                    const f = e.target.files?.[0];
                    if (f) canned.addFile(f);
                    e.target.value = "";
                  }}
                />
              </Button>
              <FormHelperText error={Boolean(canned.fileError)} data-testid="canned-file-helper">
                {canned.fileError ?? `Allowed types: ${ALLOWED_EXTENSIONS_LABEL}. Max size: ${MAX_FILE_SIZE_LABEL}. Click a chip to remove.`}
              </FormHelperText>
            </Box>

            <TextField
              label="Admin Notes"
              multiline
              rows={2}
              value={canned.notes}
              onChange={(e) => canned.setNotes(e.target.value)}
              fullWidth
            />
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => canned.closeForm()}>Cancel</Button>
          <Button type="submit" variant="contained" disabled={canned.saving || Boolean(canned.fileError)} data-testid="canned-save">
            {canned.saving ? "Saving…" : "Save"}
          </Button>
        </DialogActions>
      </Box>
    </Dialog>
  );
});
