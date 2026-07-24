// Permission-group management screen (TS-M4-B4) — FS-031.7/.8. A group list with
// member/dept counts + mass actions, and a create/edit dialog composed from the
// reusable PermissionGrid (11 flags) and DeptAccessMatrix. Wired to GroupAdminStore.
//
// @implements FS-031.7: group list (counts) + mass enable/disable/delete.
// @implements FS-031.8: create/edit with the eleven flags + dept matrix.
// @implements BS-031-019: too-short / duplicate name → inline error under Name.
// @implements BS-031-022: delete-with-members block surfaced.
// @implements BS-031-023: self-group disable/delete block surfaced.
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
  Typography,
} from "@mui/material";
import { useStores } from "../../stores/StoreContext";
import { PermissionGrid } from "../../components/PermissionGrid";
import { DeptAccessMatrix } from "../../components/DeptAccessMatrix";
import type { GroupSortKey } from "../../stores/GroupAdminStore";

const COLUMNS: Array<{ key: GroupSortKey; label: string }> = [
  { key: "name", label: "Name" },
  { key: "status", label: "Status" },
  { key: "users", label: "Members" },
  { key: "depts", label: "Dept Access" },
];

type MassAction = "enable" | "disable" | "delete";

export const GroupListPage = observer(function GroupListPage() {
  const { groupAdmin } = useStores();
  const [confirm, setConfirm] = useState<MassAction | null>(null);

  useEffect(() => {
    void groupAdmin.loadList();
  }, [groupAdmin]);

  const runMass = async () => {
    if (!confirm) return;
    const action = confirm;
    setConfirm(null);
    await groupAdmin.massAction(action);
  };

  return (
    <Box>
      <Stack direction="row" justifyContent="space-between" alignItems="center" sx={{ mb: 2 }}>
        <Typography variant="h5">Permission Groups</Typography>
        <Button variant="contained" onClick={() => groupAdmin.openCreate()} data-testid="add-group">
          Add Group
        </Button>
      </Stack>

      <Paper
        variant="outlined"
        sx={{ display: "flex", alignItems: "center", gap: 2, p: 1, mb: 1 }}
        data-testid="group-mass-bar"
      >
        <Checkbox
          checked={groupAdmin.allSelected}
          indeterminate={groupAdmin.someSelected}
          onChange={() => groupAdmin.toggleSelectAll()}
          inputProps={{ "aria-label": "Select all groups" }}
        />
        <Typography variant="body2" color="text.secondary">
          {groupAdmin.selectedIds.length > 0 ? `${groupAdmin.selectedIds.length} selected` : "Select groups"}
        </Typography>
        <Box sx={{ flexGrow: 1 }} />
        <Button size="small" disabled={groupAdmin.selectedIds.length === 0 || groupAdmin.massLoading} onClick={() => setConfirm("enable")} data-testid="group-mass-enable">
          Enable
        </Button>
        <Button size="small" disabled={groupAdmin.selectedIds.length === 0 || groupAdmin.massLoading} onClick={() => setConfirm("disable")} data-testid="group-mass-disable">
          Disable
        </Button>
        <Button size="small" color="error" disabled={groupAdmin.selectedIds.length === 0 || groupAdmin.massLoading} onClick={() => setConfirm("delete")} data-testid="group-mass-delete">
          Delete
        </Button>
      </Paper>

      {groupAdmin.listError ? <Alert severity="error" sx={{ mb: 2 }}>{groupAdmin.listError}</Alert> : null}

      <Paper variant="outlined">
        <Table size="small">
          <TableHead>
            <TableRow>
              <TableCell padding="checkbox" />
              {COLUMNS.map((c) => (
                <TableCell key={c.key} sortDirection={groupAdmin.sort === c.key ? (groupAdmin.order === "ASC" ? "asc" : "desc") : false}>
                  <TableSortLabel
                    active={groupAdmin.sort === c.key}
                    direction={groupAdmin.sort === c.key && groupAdmin.order === "DESC" ? "desc" : "asc"}
                    onClick={() => void groupAdmin.setSort(c.key)}
                    data-testid={`group-sort-${c.key}`}
                  >
                    {c.label}
                  </TableSortLabel>
                </TableCell>
              ))}
            </TableRow>
          </TableHead>
          <TableBody>
            {groupAdmin.groups.length === 0 ? (
              <TableRow>
                <TableCell colSpan={COLUMNS.length + 1}>
                  <Typography color="text.secondary">
                    {groupAdmin.loadingList ? "Loading…" : "No groups found."}
                  </Typography>
                </TableCell>
              </TableRow>
            ) : (
              groupAdmin.groups.map((row) => (
                <TableRow key={row.id} hover data-testid="group-row">
                  <TableCell padding="checkbox">
                    <Checkbox
                      checked={groupAdmin.selected.has(row.id)}
                      onChange={() => groupAdmin.toggleSelect(row.id)}
                      inputProps={{ "aria-label": `Select ${row.name}` }}
                      data-testid={`group-check-${row.id}`}
                    />
                  </TableCell>
                  <TableCell>
                    <Button variant="text" size="small" onClick={() => void groupAdmin.openEdit(row.id)} data-testid={`group-edit-${row.id}`} sx={{ textTransform: "none" }}>
                      {row.name}
                    </Button>
                  </TableCell>
                  <TableCell>
                    <Chip size="small" label={row.enabled ? "Active" : "Disabled"} color={row.enabled ? "success" : "default"} variant="outlined" />
                  </TableCell>
                  <TableCell data-testid={`group-members-${row.id}`}>{row.member_count}</TableCell>
                  <TableCell data-testid={`group-depts-${row.id}`}>{row.dept_count}</TableCell>
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
              ? `Delete ${groupAdmin.selectedIds.length} group(s)? Groups with members cannot be deleted.`
              : `${confirm === "disable" ? "Disable" : "Enable"} ${groupAdmin.selectedIds.length} group(s)?`}
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setConfirm(null)}>Cancel</Button>
          <Button variant="contained" color={confirm === "delete" ? "error" : "primary"} onClick={() => void runMass()} data-testid="group-mass-confirm">
            Confirm
          </Button>
        </DialogActions>
      </Dialog>

      <GroupFormDialog />
    </Box>
  );
});

/** Create / edit group dialog (TS-M4-B4). */
const GroupFormDialog = observer(function GroupFormDialog() {
  const { groupAdmin } = useStores();
  if (!groupAdmin.formOpen) return null;

  const onSubmit = async (e: FormEvent) => {
    e.preventDefault();
    await groupAdmin.save();
  };

  return (
    <Dialog open onClose={() => groupAdmin.closeForm()} maxWidth="md" fullWidth>
      <DialogTitle>{groupAdmin.isEditing ? "Edit Group" : "Add Group"}</DialogTitle>
      <Box component="form" onSubmit={onSubmit} noValidate>
        <DialogContent dividers>
          <Stack spacing={2}>
            <Stack direction={{ xs: "column", sm: "row" }} spacing={2} alignItems={{ sm: "center" }}>
              <TextField
                label="Group Name"
                value={groupAdmin.name}
                onChange={(e) => groupAdmin.setName(e.target.value)}
                error={Boolean(groupAdmin.fieldError("name"))}
                helperText={groupAdmin.fieldError("name") ?? " "}
                fullWidth
                inputProps={{ "data-testid": "group-name" }}
              />
              <FormControlLabel
                control={<Checkbox checked={groupAdmin.enabled} onChange={(e) => groupAdmin.setEnabled(e.target.checked)} data-testid="group-enabled" />}
                label="Active"
              />
            </Stack>

            <Divider textAlign="left">
              <Typography variant="overline">Permissions</Typography>
            </Divider>
            <PermissionGrid flags={groupAdmin.flags} onChange={(k, v) => groupAdmin.setFlag(k, v)} />

            <Divider textAlign="left">
              <Typography variant="overline">Department Access</Typography>
            </Divider>
            <DeptAccessMatrix
              departments={groupAdmin.deptOptions}
              isChecked={(id) => groupAdmin.isDeptChecked(id)}
              onToggle={(id) => groupAdmin.toggleDept(id)}
              onSelectAll={() => groupAdmin.selectAllDepts()}
              onSelectNone={() => groupAdmin.selectNoDepts()}
            />

            <TextField
              label="Admin Notes"
              multiline
              rows={2}
              value={groupAdmin.notes}
              onChange={(e) => groupAdmin.setNotes(e.target.value)}
              fullWidth
            />
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => groupAdmin.closeForm()}>Cancel</Button>
          <Button type="submit" variant="contained" disabled={groupAdmin.saving} data-testid="group-save">
            {groupAdmin.saving ? "Saving…" : "Save"}
          </Button>
        </DialogActions>
      </Box>
    </Dialog>
  );
});
