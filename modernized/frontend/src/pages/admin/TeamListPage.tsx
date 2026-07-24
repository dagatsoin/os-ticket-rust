// Team management screen (TS-M4-C4) — FS-030.8/.9. A team list (name, status,
// member count, lead) + mass actions (enable / disable / delete), and a
// create/edit dialog: name, enabled, assignment-alert override, a Lead select
// sourced from CURRENT members only, and a remove-only member roster. There is
// NO add-member control here — members are added from a staff profile
// (BS-030-14); marking the lead for removal resets the lead.
//
// @implements FS-030.8: team list + mass actions.
// @implements FS-030.9: create/edit with lead-from-members + remove roster.
// @implements BS-030-14: no add-member control; removing the lead resets it.
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
import type { TeamMassAction } from "../../stores/TeamAdminStore";

export const TeamListPage = observer(function TeamListPage() {
  const { teamAdmin } = useStores();
  const [confirm, setConfirm] = useState<TeamMassAction | null>(null);

  useEffect(() => {
    void teamAdmin.loadList();
  }, [teamAdmin]);

  const runMass = async () => {
    if (!confirm) return;
    const action = confirm;
    setConfirm(null);
    await teamAdmin.massAction(action);
  };

  return (
    <Box>
      <Stack direction="row" justifyContent="space-between" alignItems="center" sx={{ mb: 2 }}>
        <Typography variant="h5">Teams</Typography>
        <Button variant="contained" onClick={() => teamAdmin.openCreate()} data-testid="add-team">
          Add Team
        </Button>
      </Stack>

      <Paper
        variant="outlined"
        sx={{ display: "flex", alignItems: "center", gap: 2, p: 1, mb: 1 }}
        data-testid="team-mass-bar"
      >
        <Checkbox
          checked={teamAdmin.allSelected}
          indeterminate={teamAdmin.someSelected}
          onChange={() => teamAdmin.toggleSelectAll()}
          inputProps={{ "aria-label": "Select all teams" }}
        />
        <Typography variant="body2" color="text.secondary">
          {teamAdmin.selectedIds.length > 0 ? `${teamAdmin.selectedIds.length} selected` : "Select teams"}
        </Typography>
        <Box sx={{ flexGrow: 1 }} />
        <Button size="small" disabled={teamAdmin.selectedIds.length === 0 || teamAdmin.massLoading} onClick={() => setConfirm("enable")} data-testid="team-mass-enable">
          Enable
        </Button>
        <Button size="small" disabled={teamAdmin.selectedIds.length === 0 || teamAdmin.massLoading} onClick={() => setConfirm("disable")} data-testid="team-mass-disable">
          Disable
        </Button>
        <Button size="small" color="error" disabled={teamAdmin.selectedIds.length === 0 || teamAdmin.massLoading} onClick={() => setConfirm("delete")} data-testid="team-mass-delete">
          Delete
        </Button>
      </Paper>

      {teamAdmin.listError ? <Alert severity="error" sx={{ mb: 2 }}>{teamAdmin.listError}</Alert> : null}

      <Paper variant="outlined">
        <Table size="small">
          <TableHead>
            <TableRow>
              <TableCell padding="checkbox" />
              <TableCell>Name</TableCell>
              <TableCell>Status</TableCell>
              <TableCell>Members</TableCell>
              <TableCell>Lead</TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {teamAdmin.rows.length === 0 ? (
              <TableRow>
                <TableCell colSpan={5}>
                  <Typography color="text.secondary">
                    {teamAdmin.loadingList ? "Loading…" : "No teams found."}
                  </Typography>
                </TableCell>
              </TableRow>
            ) : (
              teamAdmin.rows.map((row) => (
                <TableRow key={row.id} hover data-testid="team-row">
                  <TableCell padding="checkbox">
                    <Checkbox
                      checked={teamAdmin.selected.has(row.id)}
                      onChange={() => teamAdmin.toggleSelect(row.id)}
                      inputProps={{ "aria-label": `Select ${row.name}` }}
                      data-testid={`team-check-${row.id}`}
                    />
                  </TableCell>
                  <TableCell>
                    <Button variant="text" size="small" onClick={() => void teamAdmin.openEdit(row.id)} data-testid={`team-edit-${row.id}`} sx={{ textTransform: "none" }}>
                      {row.name}
                    </Button>
                  </TableCell>
                  <TableCell>
                    <Chip size="small" label={row.isenabled ? "Active" : "Disabled"} color={row.isenabled ? "success" : "default"} variant="outlined" data-testid={`team-status-${row.id}`} />
                  </TableCell>
                  <TableCell data-testid={`team-members-${row.id}`}>{row.member_count}</TableCell>
                  <TableCell data-testid={`team-lead-${row.id}`}>{row.lead_name ?? "—"}</TableCell>
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
              ? `Delete ${teamAdmin.selectedIds.length} team(s)?`
              : `${confirm === "disable" ? "Disable" : "Enable"} ${teamAdmin.selectedIds.length} team(s)?`}
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setConfirm(null)}>Cancel</Button>
          <Button variant="contained" color={confirm === "delete" ? "error" : "primary"} onClick={() => void runMass()} data-testid="team-mass-confirm">
            Confirm
          </Button>
        </DialogActions>
      </Dialog>

      <TeamFormDialog />
    </Box>
  );
});

/** Create / edit team dialog (TS-M4-C4). */
const TeamFormDialog = observer(function TeamFormDialog() {
  const { teamAdmin } = useStores();
  if (!teamAdmin.formOpen) return null;

  const onSubmit = async (e: FormEvent) => {
    e.preventDefault();
    await teamAdmin.save();
  };

  return (
    <Dialog open onClose={() => teamAdmin.closeForm()} maxWidth="sm" fullWidth>
      <DialogTitle>{teamAdmin.isEditing ? "Edit Team" : "Add Team"}</DialogTitle>
      <Box component="form" onSubmit={onSubmit} noValidate>
        <DialogContent dividers>
          <Stack spacing={2}>
            <Stack direction={{ xs: "column", sm: "row" }} spacing={2} alignItems={{ sm: "center" }}>
              <TextField
                label="Team Name"
                value={teamAdmin.name}
                onChange={(e) => teamAdmin.setName(e.target.value)}
                error={Boolean(teamAdmin.fieldError("name"))}
                helperText={teamAdmin.fieldError("name") ?? " "}
                fullWidth
                inputProps={{ "data-testid": "team-name" }}
              />
              <FormControlLabel
                control={<Checkbox checked={teamAdmin.isenabled} onChange={(e) => teamAdmin.setIsenabled(e.target.checked)} data-testid="team-enabled" />}
                label="Active"
              />
            </Stack>

            <FormControl fullWidth>
              <InputLabel id="team-lead-label">Team Lead</InputLabel>
              <Select
                labelId="team-lead-label"
                label="Team Lead"
                value={teamAdmin.leadId == null ? "" : String(teamAdmin.leadId)}
                onChange={(e) => teamAdmin.setLeadId(e.target.value === "" ? null : Number(e.target.value))}
                inputProps={{ "data-testid": "team-lead" }}
              >
                <MenuItem value="">
                  <em>— None —</em>
                </MenuItem>
                {teamAdmin.leadCandidates.map((m) => (
                  <MenuItem key={m.staff_id} value={String(m.staff_id)}>
                    {m.name}
                  </MenuItem>
                ))}
              </Select>
            </FormControl>

            <FormControlLabel
              control={<Checkbox checked={teamAdmin.noalerts} onChange={(e) => teamAdmin.setNoalerts(e.target.checked)} data-testid="team-noalerts" />}
              label="Disable assignment alerts to this team"
            />

            <Divider textAlign="left">
              <Typography variant="overline">Members</Typography>
            </Divider>
            <Box data-testid="team-roster">
              {teamAdmin.members.length === 0 ? (
                <Typography variant="body2" color="text.secondary">
                  No members yet. Add members from a staff member's profile.
                </Typography>
              ) : (
                teamAdmin.members.map((m) => (
                  <FormControlLabel
                    key={m.staff_id}
                    control={
                      <Checkbox
                        size="small"
                        checked={teamAdmin.isMarkedForRemoval(m.staff_id)}
                        onChange={() => teamAdmin.toggleRemoveMember(m.staff_id)}
                        data-testid={`team-remove-${m.staff_id}`}
                      />
                    }
                    label={
                      <span>
                        Remove {m.name}
                        {teamAdmin.leadId === m.staff_id ? (
                          <Chip size="small" label="Lead" color="primary" variant="outlined" sx={{ ml: 1 }} data-testid={`team-lead-badge-${m.staff_id}`} />
                        ) : null}
                      </span>
                    }
                  />
                ))
              )}
            </Box>

            <TextField
              label="Notes"
              multiline
              rows={2}
              value={teamAdmin.notes}
              onChange={(e) => teamAdmin.setNotes(e.target.value)}
              fullWidth
              inputProps={{ "data-testid": "team-notes" }}
            />
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => teamAdmin.closeForm()}>Cancel</Button>
          <Button type="submit" variant="contained" disabled={teamAdmin.saving} data-testid="team-save">
            {teamAdmin.saving ? "Saving…" : "Save"}
          </Button>
        </DialogActions>
      </Box>
    </Dialog>
  );
});
