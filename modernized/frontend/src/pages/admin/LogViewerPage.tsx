// System-log viewer screen (TS-M4-G3) — FS-033.1/.2/.3/.4/.5/.6/.8. A type +
// date-span filter (native MUI TextField type="date" — no new date-picker dep),
// a sortable + paginated results table (type, title, date), a row-click detail
// dialog (content AJAX), row checkboxes + bulk delete, and a visible "Purge now"
// control invoking the grace-period sweep (BS-033.6).
//
// @implements FS-033.2: filter by type + date span.
// @implements FS-033.3: results table.
// @implements FS-033.4: sortable columns.
// @implements FS-033.5: pagination.
// @implements FS-033.6: bulk manual deletion.
// @implements FS-033.8: single-record detail (content AJAX).
// @implements BS-033.6: manual grace-period purge trigger ("Purge now").
import { useEffect, useState } from "react";
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
  FormControl,
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
import { LOG_TYPES, type LogSortKey } from "../../stores/LogViewerStore";

const COLUMNS: Array<{ key: LogSortKey; label: string }> = [
  { key: "log_type", label: "Type" },
  { key: "title", label: "Title" },
  { key: "created", label: "Date" },
];

export const LogViewerPage = observer(function LogViewerPage() {
  const { logs } = useStores();
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [confirmPurge, setConfirmPurge] = useState(false);

  useEffect(() => {
    void logs.loadList();
  }, [logs]);

  return (
    <Box>
      <Stack direction="row" justifyContent="space-between" alignItems="center" sx={{ mb: 2 }}>
        <Typography variant="h5">System Logs</Typography>
        <Button variant="outlined" color="warning" onClick={() => setConfirmPurge(true)} disabled={logs.purging} data-testid="logs-purge">
          Purge now
        </Button>
      </Stack>

      {/* Filters (FS-033.2) */}
      <Paper variant="outlined" sx={{ p: 2, mb: 2 }}>
        <Stack direction={{ xs: "column", sm: "row" }} spacing={2} alignItems={{ sm: "flex-end" }}>
          <FormControl size="small" sx={{ minWidth: 160 }}>
            <InputLabel id="log-type-label">Type</InputLabel>
            <Select
              labelId="log-type-label"
              label="Type"
              value={logs.filterType}
              onChange={(e) => logs.setFilterType(e.target.value)}
              data-testid="log-type-filter"
            >
              <MenuItem value="">All types</MenuItem>
              {LOG_TYPES.map((t) => (
                <MenuItem key={t} value={t} data-testid={`log-type-option-${t}`}>
                  {t}
                </MenuItem>
              ))}
            </Select>
          </FormControl>
          <TextField
            size="small"
            label="From"
            type="date"
            value={logs.filterFrom}
            onChange={(e) => logs.setFilterFrom(e.target.value)}
            InputLabelProps={{ shrink: true }}
            inputProps={{ "data-testid": "log-from" }}
          />
          <TextField
            size="small"
            label="To"
            type="date"
            value={logs.filterTo}
            onChange={(e) => logs.setFilterTo(e.target.value)}
            InputLabelProps={{ shrink: true }}
            inputProps={{ "data-testid": "log-to" }}
          />
          <Button variant="contained" onClick={() => void logs.applyFilters()} data-testid="log-apply">
            Apply
          </Button>
        </Stack>
      </Paper>

      {/* Bulk actions */}
      <Paper variant="outlined" sx={{ display: "flex", alignItems: "center", gap: 2, p: 1, mb: 1 }} data-testid="log-mass-bar">
        <Checkbox
          checked={logs.allSelected}
          indeterminate={logs.someSelected}
          onChange={() => logs.toggleSelectAll()}
          inputProps={{ "aria-label": "Select all log entries" }}
        />
        <Typography variant="body2" color="text.secondary">
          {logs.selectedIds.length > 0 ? `${logs.selectedIds.length} selected` : "Select entries"}
        </Typography>
        <Box sx={{ flexGrow: 1 }} />
        <Button size="small" color="error" disabled={logs.selectedIds.length === 0 || logs.deleting} onClick={() => setConfirmDelete(true)} data-testid="log-bulk-delete">
          Delete
        </Button>
      </Paper>

      {logs.listError ? <Alert severity="error" sx={{ mb: 2 }}>{logs.listError}</Alert> : null}

      <Paper variant="outlined">
        <Table size="small">
          <TableHead>
            <TableRow>
              <TableCell padding="checkbox" />
              {COLUMNS.map((c) => (
                <TableCell key={c.key} sortDirection={logs.sort === c.key ? (logs.order === "ASC" ? "asc" : "desc") : false}>
                  <TableSortLabel
                    active={logs.sort === c.key}
                    direction={logs.sort === c.key && logs.order === "DESC" ? "desc" : "asc"}
                    onClick={() => void logs.setSort(c.key)}
                    data-testid={`log-sort-${c.key}`}
                  >
                    {c.label}
                  </TableSortLabel>
                </TableCell>
              ))}
            </TableRow>
          </TableHead>
          <TableBody>
            {logs.rows.length === 0 ? (
              <TableRow>
                <TableCell colSpan={COLUMNS.length + 1}>
                  <Typography color="text.secondary">
                    {logs.loadingList ? "Loading…" : "No log entries found."}
                  </Typography>
                </TableCell>
              </TableRow>
            ) : (
              logs.rows.map((row) => (
                <TableRow key={row.id} hover data-testid="log-row">
                  <TableCell padding="checkbox">
                    <Checkbox
                      checked={logs.selected.has(row.id)}
                      onChange={() => logs.toggleSelect(row.id)}
                      inputProps={{ "aria-label": `Select log ${row.id}` }}
                      data-testid={`log-check-${row.id}`}
                      onClick={(e) => e.stopPropagation()}
                    />
                  </TableCell>
                  <TableCell data-testid={`log-type-${row.id}`}>{row.log_type}</TableCell>
                  <TableCell sx={{ cursor: "pointer" }} onClick={() => void logs.openDetail(row.id)} data-testid={`log-title-${row.id}`}>
                    {row.title}
                  </TableCell>
                  <TableCell sx={{ cursor: "pointer" }} onClick={() => void logs.openDetail(row.id)} data-testid={`log-date-${row.id}`}>
                    {row.created ? new Date(row.created).toLocaleString() : "—"}
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </Paper>

      <Pagination pagination={logs.pagination} onPageChange={(p) => void logs.setPage(p)} />

      {/* Detail dialog (content AJAX, FS-033.8) */}
      <Dialog open={logs.detailOpen} onClose={() => logs.closeDetail()} maxWidth="sm" fullWidth>
        <DialogTitle>{logs.detail?.title ?? "Log entry"}</DialogTitle>
        <DialogContent dividers>
          {logs.loadingDetail ? (
            <Typography color="text.secondary">Loading…</Typography>
          ) : logs.detail ? (
            <Stack spacing={1}>
              <Typography variant="body2" color="text.secondary">
                {logs.detail.log_type}
                {logs.detail.created ? ` · ${new Date(logs.detail.created).toLocaleString()}` : ""}
                {logs.detail.ip_address ? ` · ${logs.detail.ip_address}` : ""}
              </Typography>
              <Typography variant="body1" sx={{ whiteSpace: "pre-wrap" }} data-testid="log-detail-body">
                {logs.detail.log}
              </Typography>
            </Stack>
          ) : null}
        </DialogContent>
        <DialogActions>
          <Button onClick={() => logs.closeDetail()}>Close</Button>
        </DialogActions>
      </Dialog>

      {/* Bulk delete confirm */}
      <Dialog open={confirmDelete} onClose={() => setConfirmDelete(false)}>
        <DialogTitle>Delete log entries</DialogTitle>
        <DialogContent>
          <DialogContentText>Delete {logs.selectedIds.length} log entr{logs.selectedIds.length === 1 ? "y" : "ies"}?</DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setConfirmDelete(false)}>Cancel</Button>
          <Button
            variant="contained"
            color="error"
            data-testid="log-bulk-delete-confirm"
            onClick={async () => {
              setConfirmDelete(false);
              await logs.deleteSelected();
            }}
          >
            Delete
          </Button>
        </DialogActions>
      </Dialog>

      {/* Purge confirm */}
      <Dialog open={confirmPurge} onClose={() => setConfirmPurge(false)}>
        <DialogTitle>Purge old log entries</DialogTitle>
        <DialogContent>
          <DialogContentText>
            Remove log entries older than the configured grace period? Recent entries are retained.
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setConfirmPurge(false)}>Cancel</Button>
          <Button
            variant="contained"
            color="warning"
            data-testid="log-purge-confirm"
            onClick={async () => {
              setConfirmPurge(false);
              await logs.purge();
            }}
          >
            Purge now
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
});
