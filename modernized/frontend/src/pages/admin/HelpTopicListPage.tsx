// Help-topic management screen (TS-M4-C6) — FS-030.13/.14. A PAGINATED topic list
// (child rows render "Parent / Child") + mass actions, and a create/edit dialog:
// topic text, active/public, parent select (top-level only), required dept +
// priority, optional SLA override, thank-you page, a single MUTUALLY-EXCLUSIVE
// auto-assign staff-OR-team control (encoded s<id>/t<id>), auto-response override,
// notes.
//
// @implements FS-030.13: paginated list ("Parent / Child") + mass actions.
// @implements FS-030.14: create/edit with dept + priority + SLA + routing.
// @implements BS-030-20: parent select lists top-level topics only.
// @implements BS-030-22: auto-assign is a single staff-OR-team choice.
import { useEffect, useState, type FormEvent, type ReactNode } from "react";
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
import { Pagination } from "../../components/Pagination";
import type { TopicMassAction, TopicRow } from "../../stores/TopicAdminStore";

/** Parse an MUI Select string value into a number | null. */
function toNum(v: string): number | null {
  return v === "" ? null : Number(v);
}

/** Render a row as "Parent / Child" when it has a parent, else the topic. */
function topicLabel(row: TopicRow): string {
  return row.parent_topic ? `${row.parent_topic} / ${row.topic}` : row.topic;
}

export const HelpTopicListPage = observer(function HelpTopicListPage() {
  const { topicAdmin } = useStores();
  const [confirm, setConfirm] = useState<TopicMassAction | null>(null);

  useEffect(() => {
    void topicAdmin.loadList(1);
  }, [topicAdmin]);

  const runMass = async () => {
    if (!confirm) return;
    const action = confirm;
    setConfirm(null);
    await topicAdmin.massAction(action);
  };

  return (
    <Box>
      <Stack direction="row" justifyContent="space-between" alignItems="center" sx={{ mb: 2 }}>
        <Typography variant="h5">Help Topics</Typography>
        <Button variant="contained" onClick={() => topicAdmin.openCreate()} data-testid="add-help-topic">
          Add Help Topic
        </Button>
      </Stack>

      <Paper
        variant="outlined"
        sx={{ display: "flex", alignItems: "center", gap: 2, p: 1, mb: 1 }}
        data-testid="topic-mass-bar"
      >
        <Checkbox
          checked={topicAdmin.allSelected}
          indeterminate={topicAdmin.someSelected}
          onChange={() => topicAdmin.toggleSelectAll()}
          inputProps={{ "aria-label": "Select all topics" }}
        />
        <Typography variant="body2" color="text.secondary">
          {topicAdmin.selectedIds.length > 0 ? `${topicAdmin.selectedIds.length} selected` : "Select topics"}
        </Typography>
        <Box sx={{ flexGrow: 1 }} />
        <Button size="small" disabled={topicAdmin.selectedIds.length === 0 || topicAdmin.massLoading} onClick={() => setConfirm("enable")} data-testid="topic-mass-enable">
          Enable
        </Button>
        <Button size="small" disabled={topicAdmin.selectedIds.length === 0 || topicAdmin.massLoading} onClick={() => setConfirm("disable")} data-testid="topic-mass-disable">
          Disable
        </Button>
        <Button size="small" color="error" disabled={topicAdmin.selectedIds.length === 0 || topicAdmin.massLoading} onClick={() => setConfirm("delete")} data-testid="topic-mass-delete">
          Delete
        </Button>
      </Paper>

      {topicAdmin.listError ? <Alert severity="error" sx={{ mb: 2 }}>{topicAdmin.listError}</Alert> : null}

      <Paper variant="outlined">
        <Table size="small">
          <TableHead>
            <TableRow>
              <TableCell padding="checkbox" />
              <TableCell>Topic</TableCell>
              <TableCell>Status</TableCell>
              <TableCell>Priority</TableCell>
              <TableCell>Department</TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {topicAdmin.rows.length === 0 ? (
              <TableRow>
                <TableCell colSpan={5}>
                  <Typography color="text.secondary">
                    {topicAdmin.loadingList ? "Loading…" : "No help topics found."}
                  </Typography>
                </TableCell>
              </TableRow>
            ) : (
              topicAdmin.rows.map((row) => (
                <TableRow key={row.topic_id} hover data-testid="topic-row">
                  <TableCell padding="checkbox">
                    <Checkbox
                      checked={topicAdmin.selected.has(row.topic_id)}
                      onChange={() => topicAdmin.toggleSelect(row.topic_id)}
                      inputProps={{ "aria-label": `Select ${row.topic}` }}
                      data-testid={`topic-check-${row.topic_id}`}
                    />
                  </TableCell>
                  <TableCell>
                    <Button variant="text" size="small" onClick={() => void topicAdmin.openEdit(row.topic_id)} data-testid={`topic-edit-${row.topic_id}`} sx={{ textTransform: "none" }}>
                      {topicLabel(row)}
                    </Button>
                  </TableCell>
                  <TableCell>
                    <Chip size="small" label={row.isactive ? "Active" : "Disabled"} color={row.isactive ? "success" : "default"} variant="outlined" data-testid={`topic-status-${row.topic_id}`} />
                  </TableCell>
                  <TableCell data-testid={`topic-priority-${row.topic_id}`}>{row.priority ?? "—"}</TableCell>
                  <TableCell data-testid={`topic-dept-${row.topic_id}`}>{row.dept_name ?? "—"}</TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </Paper>

      {topicAdmin.pagination ? (
        <Pagination pagination={topicAdmin.pagination} onPageChange={(p) => void topicAdmin.setPage(p)} />
      ) : null}

      <Dialog open={confirm !== null} onClose={() => setConfirm(null)}>
        <DialogTitle>Confirm {confirm}</DialogTitle>
        <DialogContent>
          <DialogContentText>
            {confirm === "delete"
              ? `Delete ${topicAdmin.selectedIds.length} topic(s)?`
              : `${confirm === "disable" ? "Disable" : "Enable"} ${topicAdmin.selectedIds.length} topic(s)?`}
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setConfirm(null)}>Cancel</Button>
          <Button variant="contained" color={confirm === "delete" ? "error" : "primary"} onClick={() => void runMass()} data-testid="topic-mass-confirm">
            Confirm
          </Button>
        </DialogActions>
      </Dialog>

      <HelpTopicFormDialog />
    </Box>
  );
});

/** Create / edit help-topic dialog (TS-M4-C6). */
const HelpTopicFormDialog = observer(function HelpTopicFormDialog() {
  const { topicAdmin } = useStores();
  if (!topicAdmin.formOpen) return null;

  const onSubmit = async (e: FormEvent) => {
    e.preventDefault();
    await topicAdmin.save();
  };

  const deptErr = topicAdmin.fieldError("deptId");
  const prioErr = topicAdmin.fieldError("priorityId");

  // The single auto-assign select value: "" | s<id> | t<id> from the store.
  const assignValue = topicAdmin.assignTo;
  const onAssignChange = (v: string) => {
    if (v === "") topicAdmin.clearAssign();
    else if (v[0] === "s") topicAdmin.setAssignStaff(Number(v.slice(1)));
    else topicAdmin.setAssignTeam(Number(v.slice(1)));
  };

  return (
    <Dialog open onClose={() => topicAdmin.closeForm()} maxWidth="md" fullWidth>
      <DialogTitle>{topicAdmin.isEditing ? "Edit Help Topic" : "Add Help Topic"}</DialogTitle>
      <Box component="form" onSubmit={onSubmit} noValidate>
        <DialogContent dividers>
          <Stack spacing={2}>
            <TextField
              label="Topic"
              value={topicAdmin.topic}
              onChange={(e) => topicAdmin.setTopic(e.target.value)}
              error={Boolean(topicAdmin.fieldError("topic"))}
              helperText={topicAdmin.fieldError("topic") ?? " "}
              fullWidth
              inputProps={{ "data-testid": "topic-text" }}
            />

            <Stack direction="row" spacing={2}>
              <FormControlLabel
                control={<Checkbox checked={topicAdmin.isactive} onChange={(e) => topicAdmin.setIsactive(e.target.checked)} data-testid="topic-active" />}
                label="Active"
              />
              <FormControlLabel
                control={<Checkbox checked={topicAdmin.ispublic} onChange={(e) => topicAdmin.setIspublic(e.target.checked)} data-testid="topic-public" />}
                label="Public"
              />
            </Stack>

            <FormControl fullWidth>
              <InputLabel id="topic-parent-label">Parent Topic</InputLabel>
              <Select
                labelId="topic-parent-label"
                label="Parent Topic"
                value={topicAdmin.parentId == null ? "" : String(topicAdmin.parentId)}
                onChange={(e) => topicAdmin.setParentId(toNum(e.target.value))}
                inputProps={{ "data-testid": "topic-parent" }}
              >
                <MenuItem value="">
                  <em>— Top Level —</em>
                </MenuItem>
                {topicAdmin.parentCandidates.map((o) => (
                  <MenuItem key={o.id} value={String(o.id)}>
                    {o.name}
                  </MenuItem>
                ))}
              </Select>
            </FormControl>

            <Stack direction={{ xs: "column", sm: "row" }} spacing={2}>
              <FormControl fullWidth required error={Boolean(deptErr)}>
                <InputLabel id="topic-dept-label">Department</InputLabel>
                <Select
                  labelId="topic-dept-label"
                  label="Department"
                  value={topicAdmin.deptId == null ? "" : String(topicAdmin.deptId)}
                  onChange={(e) => topicAdmin.setDeptId(toNum(e.target.value))}
                  inputProps={{ "data-testid": "topic-dept" }}
                >
                  <MenuItem value="">
                    <em>— Select —</em>
                  </MenuItem>
                  {topicAdmin.options.departments.map((o) => (
                    <MenuItem key={o.id} value={String(o.id)}>
                      {o.name}
                    </MenuItem>
                  ))}
                </Select>
                <FormHelperText>{deptErr ?? " "}</FormHelperText>
              </FormControl>

              <FormControl fullWidth required error={Boolean(prioErr)}>
                <InputLabel id="topic-priority-label">Priority</InputLabel>
                <Select
                  labelId="topic-priority-label"
                  label="Priority"
                  value={topicAdmin.priorityId == null ? "" : String(topicAdmin.priorityId)}
                  onChange={(e) => topicAdmin.setPriorityId(toNum(e.target.value))}
                  inputProps={{ "data-testid": "topic-priority" }}
                >
                  <MenuItem value="">
                    <em>— Select —</em>
                  </MenuItem>
                  {topicAdmin.options.priorities.map((o) => (
                    <MenuItem key={o.id} value={String(o.id)}>
                      {o.name}
                    </MenuItem>
                  ))}
                </Select>
                <FormHelperText>{prioErr ?? " "}</FormHelperText>
              </FormControl>
            </Stack>

            <Stack direction={{ xs: "column", sm: "row" }} spacing={2}>
              <FormControl fullWidth>
                <InputLabel id="topic-sla-label">SLA Override</InputLabel>
                <Select
                  labelId="topic-sla-label"
                  label="SLA Override"
                  value={topicAdmin.slaId == null ? "" : String(topicAdmin.slaId)}
                  onChange={(e) => topicAdmin.setSlaId(toNum(e.target.value))}
                  inputProps={{ "data-testid": "topic-sla" }}
                >
                  <MenuItem value="">
                    <em>— Department's Default —</em>
                  </MenuItem>
                  {topicAdmin.options.sla.map((o) => (
                    <MenuItem key={o.id} value={String(o.id)}>
                      {o.name}
                    </MenuItem>
                  ))}
                </Select>
              </FormControl>

              <FormControl fullWidth>
                <InputLabel id="topic-page-label">Thank-You Page</InputLabel>
                <Select
                  labelId="topic-page-label"
                  label="Thank-You Page"
                  value={topicAdmin.pageId == null ? "" : String(topicAdmin.pageId)}
                  onChange={(e) => topicAdmin.setPageId(toNum(e.target.value))}
                  inputProps={{ "data-testid": "topic-page" }}
                >
                  <MenuItem value="">
                    <em>— System Default —</em>
                  </MenuItem>
                  {topicAdmin.options.pages.map((o) => (
                    <MenuItem key={o.id} value={String(o.id)}>
                      {o.name}
                    </MenuItem>
                  ))}
                </Select>
              </FormControl>
            </Stack>

            <Divider textAlign="left">
              <Typography variant="overline">Auto-Assign</Typography>
            </Divider>
            <FormControl fullWidth>
              <InputLabel id="topic-assign-label">Auto-Assign To</InputLabel>
              <Select
                labelId="topic-assign-label"
                label="Auto-Assign To"
                value={assignValue}
                onChange={(e) => onAssignChange(e.target.value)}
                inputProps={{ "data-testid": "topic-assign" }}
              >
                <MenuItem value="">
                  <em>— Unassigned —</em>
                </MenuItem>
                {topicAdmin.options.staff.length > 0
                  ? [
                      <ListSubheaderLike key="staff-h">Staff</ListSubheaderLike>,
                      ...topicAdmin.options.staff.map((o) => (
                        <MenuItem key={`s${o.id}`} value={`s${o.id}`} data-testid={`topic-assign-s${o.id}`}>
                          {o.name}
                        </MenuItem>
                      )),
                    ]
                  : null}
                {topicAdmin.options.teams.length > 0
                  ? [
                      <ListSubheaderLike key="team-h">Teams</ListSubheaderLike>,
                      ...topicAdmin.options.teams.map((o) => (
                        <MenuItem key={`t${o.id}`} value={`t${o.id}`} data-testid={`topic-assign-t${o.id}`}>
                          {o.name}
                        </MenuItem>
                      )),
                    ]
                  : null}
              </Select>
            </FormControl>

            <FormControlLabel
              control={<Checkbox checked={topicAdmin.noautoresp} onChange={(e) => topicAdmin.setNoautoresp(e.target.checked)} data-testid="topic-noautoresp" />}
              label="Disable new-ticket auto-response for this topic"
            />

            <TextField
              label="Notes"
              multiline
              rows={2}
              value={topicAdmin.notes}
              onChange={(e) => topicAdmin.setNotes(e.target.value)}
              fullWidth
              inputProps={{ "data-testid": "topic-notes" }}
            />
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => topicAdmin.closeForm()}>Cancel</Button>
          <Button type="submit" variant="contained" disabled={topicAdmin.saving} data-testid="topic-save">
            {topicAdmin.saving ? "Saving…" : "Save"}
          </Button>
        </DialogActions>
      </Box>
    </Dialog>
  );
});

/** A non-selectable group heading rendered inside the auto-assign Select. */
function ListSubheaderLike({ children }: { children: ReactNode }) {
  return (
    <MenuItem disabled sx={{ opacity: 0.7, fontWeight: 600 }}>
      {children}
    </MenuItem>
  );
}
