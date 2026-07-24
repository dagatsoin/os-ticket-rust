// Department management screen (TS-M4-C2) — FS-030.3/.4. A department list with
// public flag / manager / user count + mass actions (delete / make public /
// make private), and a create/edit dialog: name, public/private, required Email +
// Template selects, optional SLA + Manager, auto-response toggles, dept signature,
// and the group-access matrix (reusing the generalised AccessMatrix). The default
// department row's selection checkbox is disabled (BS-030-05).
//
// @implements FS-030.3: department list + mass actions; default row protected.
// @implements FS-030.4: create/edit with Email + Template + SLA + Manager + groups.
// @implements BS-030-02: missing Email → inline "Email selection required".
// @implements BS-030-03: missing Template → inline "Template selection required".
// @implements BS-030-04: default department cannot be private (inline block).
// @implements BS-030-05: default row checkbox disabled; not mass-deletable.
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
  Divider,
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
import { AccessMatrix } from "../../components/AccessMatrix";
import type { DeptMassAction } from "../../stores/DeptAdminStore";

/** Parse an MUI Select string value into a number | null. */
function toNum(v: string): number | null {
  return v === "" ? null : Number(v);
}

export const DeptListPage = observer(function DeptListPage() {
  const { deptAdmin } = useStores();
  const [confirm, setConfirm] = useState<DeptMassAction | null>(null);

  useEffect(() => {
    void deptAdmin.loadList();
  }, [deptAdmin]);

  const runMass = async () => {
    if (!confirm) return;
    const action = confirm;
    setConfirm(null);
    await deptAdmin.massAction(action);
  };

  return (
    <Box>
      <Stack direction="row" justifyContent="space-between" alignItems="center" sx={{ mb: 2 }}>
        <Typography variant="h5">Departments</Typography>
        <Button variant="contained" onClick={() => deptAdmin.openCreate()} data-testid="add-department">
          Add Department
        </Button>
      </Stack>

      <Paper
        variant="outlined"
        sx={{ display: "flex", alignItems: "center", gap: 2, p: 1, mb: 1 }}
        data-testid="dept-mass-bar"
      >
        <Checkbox
          checked={deptAdmin.allSelected}
          indeterminate={deptAdmin.someSelected}
          onChange={() => deptAdmin.toggleSelectAll()}
          inputProps={{ "aria-label": "Select all departments" }}
        />
        <Typography variant="body2" color="text.secondary">
          {deptAdmin.selectedIds.length > 0 ? `${deptAdmin.selectedIds.length} selected` : "Select departments"}
        </Typography>
        <Box sx={{ flexGrow: 1 }} />
        <Button size="small" disabled={deptAdmin.selectedIds.length === 0 || deptAdmin.massLoading} onClick={() => setConfirm("makepublic")} data-testid="dept-mass-makepublic">
          Make Public
        </Button>
        <Button size="small" disabled={deptAdmin.selectedIds.length === 0 || deptAdmin.massLoading} onClick={() => setConfirm("makeprivate")} data-testid="dept-mass-makeprivate">
          Make Private
        </Button>
        <Button size="small" color="error" disabled={deptAdmin.selectedIds.length === 0 || deptAdmin.massLoading} onClick={() => setConfirm("delete")} data-testid="dept-mass-delete">
          Delete
        </Button>
      </Paper>

      {deptAdmin.listError ? <Alert severity="error" sx={{ mb: 2 }}>{deptAdmin.listError}</Alert> : null}

      <Paper variant="outlined">
        <Table size="small">
          <TableHead>
            <TableRow>
              <TableCell padding="checkbox" />
              <TableCell>Name</TableCell>
              <TableCell>Type</TableCell>
              <TableCell>Manager</TableCell>
              <TableCell>Users</TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {deptAdmin.rows.length === 0 ? (
              <TableRow>
                <TableCell colSpan={5}>
                  <Typography color="text.secondary">
                    {deptAdmin.loadingList ? "Loading…" : "No departments found."}
                  </Typography>
                </TableCell>
              </TableRow>
            ) : (
              deptAdmin.rows.map((row) => (
                <TableRow key={row.id} hover data-testid="dept-row">
                  <TableCell padding="checkbox">
                    <Checkbox
                      checked={deptAdmin.selected.has(row.id)}
                      disabled={!deptAdmin.isSelectable(row)}
                      onChange={() => deptAdmin.toggleSelect(row.id)}
                      inputProps={{ "aria-label": `Select ${row.name}` }}
                      data-testid={`dept-check-${row.id}`}
                    />
                  </TableCell>
                  <TableCell>
                    <Button variant="text" size="small" onClick={() => void deptAdmin.openEdit(row.id)} data-testid={`dept-edit-${row.id}`} sx={{ textTransform: "none" }}>
                      {row.name}
                      {row.is_default ? " (default)" : ""}
                    </Button>
                  </TableCell>
                  <TableCell>
                    <Chip size="small" label={row.ispublic ? "Public" : "Private"} color={row.ispublic ? "success" : "default"} variant="outlined" data-testid={`dept-type-${row.id}`} />
                  </TableCell>
                  <TableCell data-testid={`dept-manager-${row.id}`}>{row.manager_name ?? "—"}</TableCell>
                  <TableCell data-testid={`dept-users-${row.id}`}>{row.user_count}</TableCell>
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
              ? `Delete ${deptAdmin.selectedIds.length} department(s)? Departments with staff cannot be deleted.`
              : `Make ${deptAdmin.selectedIds.length} department(s) ${confirm === "makepublic" ? "public" : "private"}?`}
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setConfirm(null)}>Cancel</Button>
          <Button variant="contained" color={confirm === "delete" ? "error" : "primary"} onClick={() => void runMass()} data-testid="dept-mass-confirm">
            Confirm
          </Button>
        </DialogActions>
      </Dialog>

      <DepartmentFormDialog />
    </Box>
  );
});

/** Create / edit department dialog (TS-M4-C2). */
const DepartmentFormDialog = observer(function DepartmentFormDialog() {
  const { deptAdmin } = useStores();
  if (!deptAdmin.formOpen) return null;

  const onSubmit = async (e: FormEvent) => {
    e.preventDefault();
    await deptAdmin.save();
  };

  const emailErr = deptAdmin.fieldError("emailId");
  const tplErr = deptAdmin.fieldError("tplId");
  const publicErr = deptAdmin.fieldError("ispublic");

  return (
    <Dialog open onClose={() => deptAdmin.closeForm()} maxWidth="md" fullWidth>
      <DialogTitle>{deptAdmin.isEditing ? "Edit Department" : "Add Department"}</DialogTitle>
      <Box component="form" onSubmit={onSubmit} noValidate>
        <DialogContent dividers>
          <Stack spacing={2}>
            <TextField
              label="Name"
              value={deptAdmin.name}
              onChange={(e) => deptAdmin.setName(e.target.value)}
              error={Boolean(deptAdmin.fieldError("name"))}
              helperText={deptAdmin.fieldError("name") ?? " "}
              fullWidth
              inputProps={{ "data-testid": "dept-name" }}
            />

            <FormControl error={Boolean(publicErr)}>
              <FormControlLabel
                control={
                  <Checkbox
                    checked={deptAdmin.ispublic}
                    onChange={(e) => deptAdmin.setIspublic(e.target.checked)}
                    data-testid="dept-ispublic"
                  />
                }
                label="Public (visible to end users)"
              />
              {publicErr ? <FormHelperText data-testid="dept-ispublic-error">{publicErr}</FormHelperText> : null}
            </FormControl>

            <Stack direction={{ xs: "column", sm: "row" }} spacing={2}>
              <FormControl fullWidth required error={Boolean(emailErr)}>
                <InputLabel id="dept-email-label">Email</InputLabel>
                <Select
                  labelId="dept-email-label"
                  label="Email"
                  value={deptAdmin.emailId == null ? "" : String(deptAdmin.emailId)}
                  onChange={(e) => deptAdmin.setEmailId(toNum(e.target.value))}
                  inputProps={{ "data-testid": "dept-email" }}
                >
                  <MenuItem value="">
                    <em>— Select —</em>
                  </MenuItem>
                  {deptAdmin.options.email_accounts.map((o) => (
                    <MenuItem key={o.id} value={String(o.id)}>
                      {o.email}
                    </MenuItem>
                  ))}
                </Select>
                <FormHelperText>{emailErr ?? " "}</FormHelperText>
              </FormControl>

              <FormControl fullWidth required error={Boolean(tplErr)}>
                <InputLabel id="dept-tpl-label">Template</InputLabel>
                <Select
                  labelId="dept-tpl-label"
                  label="Template"
                  value={deptAdmin.tplId == null ? "" : String(deptAdmin.tplId)}
                  onChange={(e) => deptAdmin.setTplId(toNum(e.target.value))}
                  inputProps={{ "data-testid": "dept-tpl" }}
                >
                  <MenuItem value="">
                    <em>— Select —</em>
                  </MenuItem>
                  {deptAdmin.options.template_groups.map((o) => (
                    <MenuItem key={o.id} value={String(o.id)}>
                      {o.name}
                    </MenuItem>
                  ))}
                </Select>
                <FormHelperText>{tplErr ?? " "}</FormHelperText>
              </FormControl>
            </Stack>

            <Stack direction={{ xs: "column", sm: "row" }} spacing={2}>
              <FormControl fullWidth>
                <InputLabel id="dept-sla-label">SLA Plan</InputLabel>
                <Select
                  labelId="dept-sla-label"
                  label="SLA Plan"
                  value={deptAdmin.slaId == null ? "" : String(deptAdmin.slaId)}
                  onChange={(e) => deptAdmin.setSlaId(toNum(e.target.value))}
                  inputProps={{ "data-testid": "dept-sla" }}
                >
                  <MenuItem value="">
                    <em>— System Default —</em>
                  </MenuItem>
                  {deptAdmin.options.sla.map((o) => (
                    <MenuItem key={o.id} value={String(o.id)}>
                      {o.name}
                    </MenuItem>
                  ))}
                </Select>
              </FormControl>

              <FormControl fullWidth>
                <InputLabel id="dept-manager-label">Manager</InputLabel>
                <Select
                  labelId="dept-manager-label"
                  label="Manager"
                  value={deptAdmin.managerId == null ? "" : String(deptAdmin.managerId)}
                  onChange={(e) => deptAdmin.setManagerId(toNum(e.target.value))}
                  inputProps={{ "data-testid": "dept-manager" }}
                >
                  <MenuItem value="">
                    <em>— None —</em>
                  </MenuItem>
                  {deptAdmin.options.staff.map((o) => (
                    <MenuItem key={o.id} value={String(o.id)}>
                      {o.name}
                    </MenuItem>
                  ))}
                </Select>
              </FormControl>
            </Stack>

            <Divider textAlign="left">
              <Typography variant="overline">Auto-Response</Typography>
            </Divider>
            <Stack direction={{ xs: "column", sm: "row" }} spacing={2}>
              <FormControlLabel
                control={
                  <Checkbox
                    checked={!deptAdmin.ticketAutoResponse}
                    onChange={(e) => deptAdmin.setTicketAutoResponse(!e.target.checked)}
                    data-testid="dept-no-ticket-autoresp"
                  />
                }
                label="Disable new-ticket auto-response"
              />
              <FormControlLabel
                control={
                  <Checkbox
                    checked={!deptAdmin.messageAutoResponse}
                    onChange={(e) => deptAdmin.setMessageAutoResponse(!e.target.checked)}
                    data-testid="dept-no-message-autoresp"
                  />
                }
                label="Disable new-message auto-response"
              />
            </Stack>

            <TextField
              label="Department Signature"
              multiline
              rows={2}
              value={deptAdmin.deptSignature}
              onChange={(e) => deptAdmin.setDeptSignature(e.target.value)}
              fullWidth
              inputProps={{ "data-testid": "dept-signature" }}
            />

            <Divider textAlign="left">
              <Typography variant="overline">Group Access</Typography>
            </Divider>
            <AccessMatrix
              items={deptAdmin.options.groups}
              isChecked={(id) => deptAdmin.isGroupChecked(id)}
              onToggle={(id) => deptAdmin.toggleGroup(id)}
              onSelectAll={() => deptAdmin.selectAllGroups()}
              onSelectNone={() => deptAdmin.selectNoGroups()}
              testIdPrefix="group"
              emptyLabel="No groups available."
            />
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => deptAdmin.closeForm()}>Cancel</Button>
          <Button type="submit" variant="contained" disabled={deptAdmin.saving} data-testid="dept-save">
            {deptAdmin.saving ? "Saving…" : "Save"}
          </Button>
        </DialogActions>
      </Box>
    </Dialog>
  );
});
