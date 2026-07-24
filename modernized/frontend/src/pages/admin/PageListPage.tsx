// Site-pages management screen (TS-M4-F2) — FS-033.11/.12/.13/.14/.15/.16. A
// sortable, paginated page list (name, type, status, in-use badge) with mass
// enable/disable/delete, and a create/edit dialog (name, type, body, active,
// notes). In-use pages show a badge; delete AND disable over an in-use page are
// blocked with a message (BS-033.9). Duplicate names surface inline (BS-033.10).
//
// @implements FS-033.11: site-pages list.
// @implements FS-033.12: results table — sort + pagination.
// @implements FS-033.13: create / edit a page.
// @implements FS-033.15: enable / disable / delete bulk actions.
// @implements BS-033.9: in-use page — delete AND disable refused (badge + guard).
// @implements BS-033.10: duplicate name → inline error.
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
  Tooltip,
  Typography,
} from "@mui/material";
import { useStores } from "../../stores/StoreContext";
import { Pagination } from "../../components/Pagination";
import { PAGE_TYPES, type PageMassAction, type PageSortKey, type PageType } from "../../stores/PageAdminStore";

const COLUMNS: Array<{ key: PageSortKey; label: string }> = [
  { key: "name", label: "Name" },
  { key: "type", label: "Type" },
  { key: "status", label: "Status" },
];

export const PageListPage = observer(function PageListPage() {
  const { pageAdmin } = useStores();
  const [confirm, setConfirm] = useState<PageMassAction | null>(null);

  useEffect(() => {
    void pageAdmin.loadList();
  }, [pageAdmin]);

  const runMass = async () => {
    if (!confirm) return;
    const action = confirm;
    setConfirm(null);
    await pageAdmin.massAction(action);
  };

  const guardBlocked = pageAdmin.selectionHasInUse;

  return (
    <Box>
      <Stack direction="row" justifyContent="space-between" alignItems="center" sx={{ mb: 2 }}>
        <Typography variant="h5">Site Pages</Typography>
        <Button variant="contained" onClick={() => pageAdmin.openCreate()} data-testid="add-page">
          Add Page
        </Button>
      </Stack>

      <Paper
        variant="outlined"
        sx={{ display: "flex", alignItems: "center", gap: 2, p: 1, mb: 1 }}
        data-testid="page-mass-bar"
      >
        <Checkbox
          checked={pageAdmin.allSelected}
          indeterminate={pageAdmin.someSelected}
          onChange={() => pageAdmin.toggleSelectAll()}
          inputProps={{ "aria-label": "Select all pages" }}
        />
        <Typography variant="body2" color="text.secondary">
          {pageAdmin.selectedIds.length > 0 ? `${pageAdmin.selectedIds.length} selected` : "Select pages"}
        </Typography>
        <Box sx={{ flexGrow: 1 }} />
        <Button size="small" disabled={pageAdmin.selectedIds.length === 0 || pageAdmin.massLoading} onClick={() => setConfirm("enable")} data-testid="page-mass-enable">
          Enable
        </Button>
        <Tooltip title={guardBlocked ? "A page in use cannot be disabled." : ""}>
          <span>
            <Button size="small" disabled={pageAdmin.selectedIds.length === 0 || pageAdmin.massLoading || guardBlocked} onClick={() => setConfirm("disable")} data-testid="page-mass-disable">
              Disable
            </Button>
          </span>
        </Tooltip>
        <Tooltip title={guardBlocked ? "A page in use cannot be deleted." : ""}>
          <span>
            <Button size="small" color="error" disabled={pageAdmin.selectedIds.length === 0 || pageAdmin.massLoading || guardBlocked} onClick={() => setConfirm("delete")} data-testid="page-mass-delete">
              Delete
            </Button>
          </span>
        </Tooltip>
      </Paper>

      {pageAdmin.listError ? <Alert severity="error" sx={{ mb: 2 }}>{pageAdmin.listError}</Alert> : null}
      {pageAdmin.massError ? (
        <Alert severity="error" sx={{ mb: 2 }} data-testid="page-mass-error">
          {pageAdmin.massError}
        </Alert>
      ) : null}

      <Paper variant="outlined">
        <Table size="small">
          <TableHead>
            <TableRow>
              <TableCell padding="checkbox" />
              {COLUMNS.map((c) => (
                <TableCell key={c.key} sortDirection={pageAdmin.sort === c.key ? (pageAdmin.order === "ASC" ? "asc" : "desc") : false}>
                  <TableSortLabel
                    active={pageAdmin.sort === c.key}
                    direction={pageAdmin.sort === c.key && pageAdmin.order === "DESC" ? "desc" : "asc"}
                    onClick={() => void pageAdmin.setSort(c.key)}
                    data-testid={`page-sort-${c.key}`}
                  >
                    {c.label}
                  </TableSortLabel>
                </TableCell>
              ))}
              <TableCell>In Use</TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {pageAdmin.rows.length === 0 ? (
              <TableRow>
                <TableCell colSpan={COLUMNS.length + 2}>
                  <Typography color="text.secondary">
                    {pageAdmin.loadingList ? "Loading…" : "No pages found."}
                  </Typography>
                </TableCell>
              </TableRow>
            ) : (
              pageAdmin.rows.map((row) => (
                <TableRow key={row.id} hover data-testid="page-row">
                  <TableCell padding="checkbox">
                    <Checkbox
                      checked={pageAdmin.selected.has(row.id)}
                      onChange={() => pageAdmin.toggleSelect(row.id)}
                      inputProps={{ "aria-label": `Select ${row.name}` }}
                      data-testid={`page-check-${row.id}`}
                    />
                  </TableCell>
                  <TableCell>
                    <Button variant="text" size="small" onClick={() => void pageAdmin.openEdit(row.id)} data-testid={`page-edit-${row.id}`} sx={{ textTransform: "none" }}>
                      {row.name}
                    </Button>
                  </TableCell>
                  <TableCell data-testid={`page-type-${row.id}`}>{row.type}</TableCell>
                  <TableCell>
                    <Chip size="small" label={row.isactive ? "Active" : "Disabled"} color={row.isactive ? "success" : "default"} variant="outlined" />
                  </TableCell>
                  <TableCell>
                    {row.in_use ? (
                      <Chip size="small" label="In use" color="warning" variant="outlined" data-testid={`page-inuse-${row.id}`} />
                    ) : (
                      <Typography variant="body2" color="text.secondary">
                        —
                      </Typography>
                    )}
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </Paper>

      <Pagination pagination={pageAdmin.pagination} onPageChange={(p) => void pageAdmin.setPage(p)} />

      <Dialog open={confirm !== null} onClose={() => setConfirm(null)}>
        <DialogTitle>Confirm {confirm}</DialogTitle>
        <DialogContent>
          <DialogContentText>
            {confirm === "delete"
              ? `Delete ${pageAdmin.selectedIds.length} page(s)?`
              : `${confirm === "disable" ? "Disable" : "Enable"} ${pageAdmin.selectedIds.length} page(s)?`}
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setConfirm(null)}>Cancel</Button>
          <Button variant="contained" color={confirm === "delete" ? "error" : "primary"} onClick={() => void runMass()} data-testid="page-mass-confirm">
            Confirm
          </Button>
        </DialogActions>
      </Dialog>

      <PageFormDialog />
    </Box>
  );
});

/** Create / edit site-page dialog (TS-M4-F2). */
const PageFormDialog = observer(function PageFormDialog() {
  const { pageAdmin } = useStores();
  if (!pageAdmin.formOpen) return null;

  const onSubmit = async (e: FormEvent) => {
    e.preventDefault();
    await pageAdmin.save();
  };

  return (
    <Dialog open onClose={() => pageAdmin.closeForm()} maxWidth="md" fullWidth>
      <DialogTitle>{pageAdmin.isEditing ? "Edit Page" : "Add Page"}</DialogTitle>
      <Box component="form" onSubmit={onSubmit} noValidate>
        <DialogContent dividers>
          <Stack spacing={2}>
            <Stack direction={{ xs: "column", sm: "row" }} spacing={2} alignItems={{ sm: "flex-start" }}>
              <TextField
                label="Name"
                value={pageAdmin.name}
                onChange={(e) => pageAdmin.setName(e.target.value)}
                error={Boolean(pageAdmin.fieldError("name"))}
                helperText={pageAdmin.fieldError("name") ?? " "}
                fullWidth
                inputProps={{ "data-testid": "page-name" }}
              />
              <FormControl sx={{ minWidth: 160 }}>
                <InputLabel id="page-type-label">Type</InputLabel>
                <Select
                  labelId="page-type-label"
                  label="Type"
                  value={pageAdmin.type}
                  onChange={(e) => pageAdmin.setType(e.target.value as PageType)}
                  data-testid="page-type-select"
                >
                  {PAGE_TYPES.map((t) => (
                    <MenuItem key={t} value={t} data-testid={`page-type-option-${t}`}>
                      {t}
                    </MenuItem>
                  ))}
                </Select>
              </FormControl>
            </Stack>
            <TextField
              label="Body"
              multiline
              rows={8}
              value={pageAdmin.body}
              onChange={(e) => pageAdmin.setBody(e.target.value)}
              error={Boolean(pageAdmin.fieldError("body"))}
              helperText={pageAdmin.fieldError("body") ?? " "}
              fullWidth
              inputProps={{ "data-testid": "page-body" }}
            />
            <FormControlLabel
              control={<Checkbox checked={pageAdmin.isactive} onChange={(e) => pageAdmin.setIsactive(e.target.checked)} data-testid="page-active" />}
              label="Active"
            />
            <TextField
              label="Admin Notes"
              multiline
              rows={2}
              value={pageAdmin.notes}
              onChange={(e) => pageAdmin.setNotes(e.target.value)}
              fullWidth
            />
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => pageAdmin.closeForm()}>Cancel</Button>
          <Button type="submit" variant="contained" disabled={pageAdmin.saving} data-testid="page-save">
            {pageAdmin.saving ? "Saving…" : "Save"}
          </Button>
        </DialogActions>
      </Box>
    </Dialog>
  );
});
