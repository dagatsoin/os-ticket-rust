// Staff management screen (TS-M4-B2) — FS-031.2/.3/.4. A filterable/sortable/
// paginated roster inside the admin shell, with a create/edit dialog and an
// enable/lock/delete mass-action bar. Wired to StaffAdminStore.
//
// @implements FS-031.2: list with filter/search + sortable headers + pagination.
// @implements FS-031.3: create / edit staff (inline 422 field errors).
// @implements FS-031.4: mass enable / lock / delete with confirm.
// @implements BS-031-001: duplicate-username → inline error under Username.
// @implements BS-031-014: last-admin → inline error on the Administrator field.
// @implements BS-031-015: self-action block surfaced (mass lock/delete).
import { useEffect, useState, type FormEvent } from "react";
import { observer } from "mobx-react-lite";
import {
  Alert,
  Box,
  Button,
  Checkbox,
  Chip,
  CircularProgress,
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
  TableSortLabel,
  TextField,
  Typography,
} from "@mui/material";
import { useStores } from "../../stores/StoreContext";
import { Pagination } from "../../components/Pagination";
import type { StaffRow, StaffSortKey } from "../../stores/StaffAdminStore";

const COLUMNS: Array<{ key: StaffSortKey; label: string }> = [
  { key: "name", label: "Name" },
  { key: "username", label: "Username" },
  { key: "status", label: "Status" },
  { key: "group", label: "Group" },
  { key: "dept", label: "Department" },
];

type MassAction = "enable" | "lock" | "delete";

export const StaffListPage = observer(function StaffListPage() {
  const { staffAdmin, staffAuth } = useStores();
  const [filter, setFilter] = useState("");
  const [confirm, setConfirm] = useState<MassAction | null>(null);

  useEffect(() => {
    void staffAdmin.loadList();
    void staffAdmin.loadOptions();
  }, [staffAdmin]);

  const ownId = (staffAuth.user as { id?: number } | null)?.id;

  const handleSearch = (e: FormEvent) => {
    e.preventDefault();
    void staffAdmin.search(filter);
  };

  const openEdit = (row: StaffRow) => {
    const own = row.id === ownId;
    staffAdmin.openEdit(row, own ? { ownIsAdmin: staffAuth.isAdmin } : undefined);
  };

  const runMass = async () => {
    if (!confirm) return;
    const action = confirm;
    setConfirm(null);
    await staffAdmin.massAction(action);
  };

  return (
    <Box>
      <Stack direction="row" justifyContent="space-between" alignItems="center" sx={{ mb: 2 }}>
        <Typography variant="h5">Staff Members</Typography>
        <Button variant="contained" onClick={() => staffAdmin.openCreate()} data-testid="add-staff">
          Add Staff
        </Button>
      </Stack>

      {/* Filter / search */}
      <Box component="form" onSubmit={handleSearch} sx={{ mb: 2 }}>
        <Stack direction="row" spacing={1}>
          <TextField
            size="small"
            placeholder="Filter staff…"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            inputProps={{ "data-testid": "staff-filter-input" }}
            sx={{ minWidth: 260 }}
          />
          <Button type="submit" variant="outlined" data-testid="staff-filter-submit">
            Filter
          </Button>
        </Stack>
      </Box>

      {/* Mass-action bar */}
      <Paper
        variant="outlined"
        sx={{ display: "flex", alignItems: "center", gap: 2, p: 1, mb: 1 }}
        data-testid="staff-mass-bar"
      >
        <Checkbox
          checked={staffAdmin.allSelected}
          indeterminate={staffAdmin.someSelected}
          onChange={() => staffAdmin.toggleSelectAll()}
          inputProps={{ "aria-label": "Select all staff" }}
        />
        <Typography variant="body2" color="text.secondary">
          {staffAdmin.selectedIds.length > 0 ? `${staffAdmin.selectedIds.length} selected` : "Select staff"}
        </Typography>
        <Box sx={{ flexGrow: 1 }} />
        <Button
          size="small"
          disabled={staffAdmin.selectedIds.length === 0 || staffAdmin.massLoading}
          onClick={() => setConfirm("enable")}
          data-testid="mass-enable"
        >
          Enable
        </Button>
        <Button
          size="small"
          disabled={staffAdmin.selectedIds.length === 0 || staffAdmin.massLoading}
          onClick={() => setConfirm("lock")}
          data-testid="mass-lock"
        >
          Lock
        </Button>
        <Button
          size="small"
          color="error"
          disabled={staffAdmin.selectedIds.length === 0 || staffAdmin.massLoading}
          onClick={() => setConfirm("delete")}
          data-testid="mass-delete"
        >
          Delete
        </Button>
      </Paper>

      {staffAdmin.listError ? <Alert severity="error" sx={{ mb: 2 }}>{staffAdmin.listError}</Alert> : null}

      <Paper variant="outlined">
        <Table size="small">
          <TableHead>
            <TableRow>
              <TableCell padding="checkbox" />
              {COLUMNS.map((c) => (
                <TableCell key={c.key} sortDirection={staffAdmin.params.sort === c.key ? (staffAdmin.params.order === "ASC" ? "asc" : "desc") : false}>
                  <TableSortLabel
                    active={staffAdmin.params.sort === c.key}
                    direction={staffAdmin.params.sort === c.key && staffAdmin.params.order === "DESC" ? "desc" : "asc"}
                    onClick={() => void staffAdmin.setSort(c.key)}
                    data-testid={`staff-sort-${c.key}`}
                  >
                    {c.label}
                  </TableSortLabel>
                </TableCell>
              ))}
            </TableRow>
          </TableHead>
          <TableBody>
            {staffAdmin.staff.length === 0 ? (
              <TableRow>
                <TableCell colSpan={COLUMNS.length + 1}>
                  <Typography color="text.secondary">
                    {staffAdmin.loadingList ? "Loading…" : "No staff found."}
                  </Typography>
                </TableCell>
              </TableRow>
            ) : (
              staffAdmin.staff.map((row) => (
                <TableRow key={row.id} hover data-testid="staff-row">
                  <TableCell padding="checkbox">
                    <Checkbox
                      checked={staffAdmin.selected.has(row.id)}
                      onChange={() => staffAdmin.toggleSelect(row.id)}
                      inputProps={{ "aria-label": `Select ${row.username}` }}
                      data-testid={`staff-check-${row.id}`}
                    />
                  </TableCell>
                  <TableCell>
                    <Button variant="text" size="small" onClick={() => openEdit(row)} data-testid={`staff-edit-${row.id}`} sx={{ textTransform: "none" }}>
                      {row.name}
                    </Button>
                  </TableCell>
                  <TableCell>{row.username}</TableCell>
                  <TableCell>
                    <Chip
                      size="small"
                      label={row.isactive ? "Active" : "Locked"}
                      color={row.isactive ? "success" : "default"}
                      variant="outlined"
                    />
                    {row.onvacation ? <Chip size="small" label="Vacation" sx={{ ml: 0.5 }} variant="outlined" /> : null}
                  </TableCell>
                  <TableCell>{row.group_name}</TableCell>
                  <TableCell>{row.dept_name}</TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </Paper>

      {staffAdmin.pagination ? (
        <Pagination pagination={staffAdmin.pagination} onPageChange={(p) => void staffAdmin.setPage(p)} />
      ) : null}

      {/* Mass-action confirm */}
      <Dialog open={confirm !== null} onClose={() => setConfirm(null)}>
        <DialogTitle>Confirm {confirm}</DialogTitle>
        <DialogContent>
          <DialogContentText>
            {confirm === "delete"
              ? `Permanently delete ${staffAdmin.selectedIds.length} staff member(s)? This cannot be undone.`
              : `${confirm === "lock" ? "Lock" : "Enable"} ${staffAdmin.selectedIds.length} staff member(s)?`}
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setConfirm(null)}>Cancel</Button>
          <Button
            variant="contained"
            color={confirm === "delete" ? "error" : "primary"}
            onClick={() => void runMass()}
            data-testid="mass-confirm"
          >
            Confirm
          </Button>
        </DialogActions>
      </Dialog>

      <StaffFormDialog />
    </Box>
  );
});

/** Create / edit staff dialog (TS-M4-B2). */
const StaffFormDialog = observer(function StaffFormDialog() {
  const { staffAdmin } = useStores();
  const [addTeamId, setAddTeamId] = useState("");
  const editingId = staffAdmin.editingId;

  // Derive the member's current teams whenever the edit dialog targets a staff id
  // (BS-030-14 — there is no GET single-staff endpoint).
  useEffect(() => {
    setAddTeamId("");
    if (editingId != null) void staffAdmin.loadStaffTeams(editingId);
  }, [staffAdmin, editingId]);

  if (!staffAdmin.formOpen) return null;
  const f = staffAdmin.form;
  const err = (k: string) => staffAdmin.fieldError(k);

  const onSubmit = async (e: FormEvent) => {
    e.preventDefault();
    await staffAdmin.save();
  };

  const onAddTeam = async () => {
    if (editingId == null || !addTeamId) return;
    const ok = await staffAdmin.addToTeam(editingId, Number(addTeamId));
    if (ok) setAddTeamId("");
  };

  return (
    <Dialog open onClose={() => staffAdmin.closeForm()} maxWidth="md" fullWidth>
      <DialogTitle>{staffAdmin.isEditing ? "Edit Staff" : "Add Staff"}</DialogTitle>
      <Box component="form" onSubmit={onSubmit} noValidate>
        <DialogContent dividers>
          {!staffAdmin.optionsLoaded ? (
            <Box sx={{ display: "flex", justifyContent: "center", py: 2 }}>
              <CircularProgress size={24} />
            </Box>
          ) : null}
          <Stack spacing={2}>
            <Stack direction={{ xs: "column", sm: "row" }} spacing={2}>
              <TextField
                label="First name"
                value={f.firstname}
                onChange={(e) => staffAdmin.setField("firstname", e.target.value)}
                error={Boolean(err("firstname"))}
                helperText={err("firstname") ?? " "}
                fullWidth
                inputProps={{ "data-testid": "staff-firstname" }}
              />
              <TextField
                label="Last name"
                value={f.lastname}
                onChange={(e) => staffAdmin.setField("lastname", e.target.value)}
                error={Boolean(err("lastname"))}
                helperText={err("lastname") ?? " "}
                fullWidth
                inputProps={{ "data-testid": "staff-lastname" }}
              />
            </Stack>
            <Stack direction={{ xs: "column", sm: "row" }} spacing={2}>
              <TextField
                label="Username"
                value={f.username}
                onChange={(e) => staffAdmin.setField("username", e.target.value)}
                error={Boolean(err("username"))}
                helperText={err("username") ?? " "}
                fullWidth
                inputProps={{ "data-testid": "staff-username" }}
              />
              <TextField
                label="Email"
                type="email"
                value={f.email}
                onChange={(e) => staffAdmin.setField("email", e.target.value)}
                error={Boolean(err("email"))}
                helperText={err("email") ?? " "}
                fullWidth
                inputProps={{ "data-testid": "staff-email" }}
              />
            </Stack>
            <Stack direction={{ xs: "column", sm: "row" }} spacing={2}>
              <TextField
                label="Phone"
                value={f.phone}
                onChange={(e) => staffAdmin.setField("phone", e.target.value)}
                error={Boolean(err("phone"))}
                helperText={err("phone") ?? " "}
                fullWidth
              />
              <TextField
                label="Ext"
                value={f.phoneExt}
                onChange={(e) => staffAdmin.setField("phoneExt", e.target.value)}
                sx={{ maxWidth: 120 }}
              />
              <TextField
                label="Mobile"
                value={f.mobile}
                onChange={(e) => staffAdmin.setField("mobile", e.target.value)}
                error={Boolean(err("mobile"))}
                helperText={err("mobile") ?? " "}
                fullWidth
              />
            </Stack>
            <Stack direction={{ xs: "column", sm: "row" }} spacing={2}>
              <FormControl fullWidth error={Boolean(err("groupId"))}>
                <InputLabel id="staff-group-label">Group</InputLabel>
                <Select
                  labelId="staff-group-label"
                  label="Group"
                  value={f.groupId}
                  onChange={(e) => staffAdmin.setField("groupId", String(e.target.value))}
                  data-testid="staff-group"
                >
                  <MenuItem value=""><em>Select…</em></MenuItem>
                  {staffAdmin.groupOptions.map((g) => (
                    <MenuItem key={g.id} value={String(g.id)}>{g.name}</MenuItem>
                  ))}
                </Select>
                <FormHelperText>{err("groupId") ?? " "}</FormHelperText>
              </FormControl>
              <FormControl fullWidth error={Boolean(err("deptId"))}>
                <InputLabel id="staff-dept-label">Primary Department</InputLabel>
                <Select
                  labelId="staff-dept-label"
                  label="Primary Department"
                  value={f.deptId}
                  onChange={(e) => staffAdmin.setField("deptId", String(e.target.value))}
                  data-testid="staff-dept"
                >
                  <MenuItem value=""><em>Select…</em></MenuItem>
                  {staffAdmin.deptOptions.map((d) => (
                    <MenuItem key={d.id} value={String(d.id)}>{d.name}</MenuItem>
                  ))}
                </Select>
                <FormHelperText>{err("deptId") ?? " "}</FormHelperText>
              </FormControl>
              <FormControl fullWidth>
                <InputLabel id="staff-tz-label">Time Zone</InputLabel>
                <Select
                  labelId="staff-tz-label"
                  label="Time Zone"
                  value={f.timezoneId}
                  onChange={(e) => staffAdmin.setField("timezoneId", String(e.target.value))}
                  data-testid="staff-timezone"
                >
                  <MenuItem value=""><em>Default</em></MenuItem>
                  {staffAdmin.timezoneOptions.map((t) => (
                    <MenuItem key={t.id} value={String(t.id)}>{t.label}</MenuItem>
                  ))}
                </Select>
              </FormControl>
            </Stack>

            {/* Flags */}
            <Box>
              <Stack direction="row" flexWrap="wrap">
                <FormControlLabel
                  control={<Checkbox checked={f.isadmin} onChange={(e) => staffAdmin.setField("isadmin", e.target.checked)} data-testid="staff-isadmin" />}
                  label="Administrator"
                />
                <FormControlLabel
                  control={<Checkbox checked={f.isactive} onChange={(e) => staffAdmin.setField("isactive", e.target.checked)} data-testid="staff-isactive" />}
                  label="Active"
                />
                <FormControlLabel
                  control={<Checkbox checked={f.isvisible} onChange={(e) => staffAdmin.setField("isvisible", e.target.checked)} data-testid="staff-isvisible" />}
                  label="Directory visible"
                />
                <FormControlLabel
                  control={<Checkbox checked={f.onvacation} onChange={(e) => staffAdmin.setField("onvacation", e.target.checked)} data-testid="staff-onvacation" />}
                  label="On vacation"
                />
              </Stack>
              {err("isadmin") ? (
                <FormHelperText error data-testid="staff-isadmin-error">{err("isadmin")}</FormHelperText>
              ) : null}
            </Box>

            <TextField
              label="Signature"
              multiline
              rows={2}
              value={f.signature}
              onChange={(e) => staffAdmin.setField("signature", e.target.value)}
              fullWidth
            />

            {/* Password */}
            <Typography variant="subtitle2" color="text.secondary">
              {staffAdmin.isEditing ? "Reset password (leave blank to keep current)" : "Temporary password"}
            </Typography>
            <Stack direction={{ xs: "column", sm: "row" }} spacing={2}>
              <TextField
                label={staffAdmin.isEditing ? "New password" : "Temp password"}
                type="password"
                value={f.password}
                onChange={(e) => staffAdmin.setField("password", e.target.value)}
                error={Boolean(err("password"))}
                helperText={err("password") ?? " "}
                fullWidth
                inputProps={{ "data-testid": "staff-password" }}
              />
              <TextField
                label="Confirm password"
                type="password"
                value={f.passwd2}
                onChange={(e) => staffAdmin.setField("passwd2", e.target.value)}
                error={Boolean(err("passwd2"))}
                helperText={err("passwd2") ?? " "}
                fullWidth
                inputProps={{ "data-testid": "staff-passwd2" }}
              />
            </Stack>

            {/* Teams (BS-030-14) — only when editing an existing staff member. */}
            {staffAdmin.isEditing ? (
              <>
                <Divider textAlign="left">
                  <Typography variant="overline">Teams</Typography>
                </Divider>
                <Box data-testid="staff-teams">
                  {staffAdmin.staffTeams.length === 0 ? (
                    <Typography variant="body2" color="text.secondary" data-testid="staff-teams-empty">
                      {staffAdmin.teamsLoading ? "Loading teams…" : "Not a member of any team yet."}
                    </Typography>
                  ) : (
                    <Stack direction="row" spacing={1} flexWrap="wrap" useFlexGap>
                      {staffAdmin.staffTeams.map((t) => (
                        <Chip
                          key={t.id}
                          label={t.name}
                          onDelete={() =>
                            editingId != null && void staffAdmin.removeFromTeam(editingId, t.id)
                          }
                          disabled={staffAdmin.teamActionPending}
                          data-testid={`staff-team-chip-${t.id}`}
                        />
                      ))}
                    </Stack>
                  )}
                </Box>
                <Stack direction={{ xs: "column", sm: "row" }} spacing={2} alignItems={{ sm: "center" }}>
                  <FormControl fullWidth size="small" disabled={staffAdmin.availableTeams.length === 0}>
                    <InputLabel id="staff-add-team-label">Add to team</InputLabel>
                    <Select
                      labelId="staff-add-team-label"
                      label="Add to team"
                      value={addTeamId}
                      onChange={(e) => setAddTeamId(String(e.target.value))}
                      inputProps={{ "data-testid": "staff-add-team-select" }}
                    >
                      <MenuItem value="">
                        <em>Select a team…</em>
                      </MenuItem>
                      {staffAdmin.availableTeams.map((t) => (
                        <MenuItem key={t.id} value={String(t.id)}>
                          {t.name}
                        </MenuItem>
                      ))}
                    </Select>
                  </FormControl>
                  <Button
                    variant="outlined"
                    onClick={() => void onAddTeam()}
                    disabled={!addTeamId || staffAdmin.teamActionPending}
                    data-testid="staff-add-team-btn"
                  >
                    Add
                  </Button>
                </Stack>
              </>
            ) : null}
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => staffAdmin.closeForm()}>Cancel</Button>
          <Button type="submit" variant="contained" disabled={staffAdmin.saving} data-testid="staff-save">
            {staffAdmin.saving ? "Saving…" : "Save"}
          </Button>
        </DialogActions>
      </Box>
    </Dialog>
  );
});
