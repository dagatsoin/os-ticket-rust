// FAQ-category management screen (TS-M4-E2) — FS-032.13/.14. A FAQ-category list
// (name, public/internal, description) + mass actions + single delete, and a
// create/edit dialog: name, public/internal toggle, description, notes. Reachable
// via the `can_manage_faq` capability gate (a delegated non-admin FAQ manager),
// so it must not assume admin-only chrome.
//
// @implements FS-032.13: FAQ-category list + edit/delete.
// @implements FS-032.14: create a FAQ category.
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
  TextField,
  Typography,
} from "@mui/material";
import { useStores } from "../../stores/StoreContext";
import type { FaqMassAction } from "../../stores/FaqCategoryStore";

export const FaqCategoryListPage = observer(function FaqCategoryListPage() {
  const { faqCategories } = useStores();
  const [confirm, setConfirm] = useState<FaqMassAction | null>(null);
  const [deleteId, setDeleteId] = useState<number | null>(null);

  useEffect(() => {
    void faqCategories.loadList();
  }, [faqCategories]);

  const runMass = async () => {
    if (!confirm) return;
    const action = confirm;
    setConfirm(null);
    await faqCategories.massAction(action);
  };

  const runDelete = async () => {
    if (deleteId == null) return;
    const id = deleteId;
    setDeleteId(null);
    await faqCategories.deleteOne(id);
  };

  return (
    <Box>
      <Stack direction="row" justifyContent="space-between" alignItems="center" sx={{ mb: 2 }}>
        <Typography variant="h5">FAQ Categories</Typography>
        <Button variant="contained" onClick={() => faqCategories.openCreate()} data-testid="add-faq-category">
          Add Category
        </Button>
      </Stack>

      <Paper
        variant="outlined"
        sx={{ display: "flex", alignItems: "center", gap: 2, p: 1, mb: 1 }}
        data-testid="faq-mass-bar"
      >
        <Checkbox
          checked={faqCategories.allSelected}
          indeterminate={faqCategories.someSelected}
          onChange={() => faqCategories.toggleSelectAll()}
          inputProps={{ "aria-label": "Select all categories" }}
        />
        <Typography variant="body2" color="text.secondary">
          {faqCategories.selectedIds.length > 0 ? `${faqCategories.selectedIds.length} selected` : "Select categories"}
        </Typography>
        <Box sx={{ flexGrow: 1 }} />
        <Button size="small" disabled={faqCategories.selectedIds.length === 0 || faqCategories.massLoading} onClick={() => setConfirm("makepublic")} data-testid="faq-mass-makepublic">
          Make Public
        </Button>
        <Button size="small" disabled={faqCategories.selectedIds.length === 0 || faqCategories.massLoading} onClick={() => setConfirm("makeprivate")} data-testid="faq-mass-makeprivate">
          Make Internal
        </Button>
        <Button size="small" color="error" disabled={faqCategories.selectedIds.length === 0 || faqCategories.massLoading} onClick={() => setConfirm("delete")} data-testid="faq-mass-delete">
          Delete
        </Button>
      </Paper>

      {faqCategories.listError ? <Alert severity="error" sx={{ mb: 2 }}>{faqCategories.listError}</Alert> : null}

      <Paper variant="outlined">
        <Table size="small">
          <TableHead>
            <TableRow>
              <TableCell padding="checkbox" />
              <TableCell>Name</TableCell>
              <TableCell>Type</TableCell>
              <TableCell>Description</TableCell>
              <TableCell align="right">Actions</TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {faqCategories.rows.length === 0 ? (
              <TableRow>
                <TableCell colSpan={5}>
                  <Typography color="text.secondary">
                    {faqCategories.loadingList ? "Loading…" : "No categories found."}
                  </Typography>
                </TableCell>
              </TableRow>
            ) : (
              faqCategories.rows.map((row) => (
                <TableRow key={row.id} hover data-testid="faq-row">
                  <TableCell padding="checkbox">
                    <Checkbox
                      checked={faqCategories.selected.has(row.id)}
                      onChange={() => faqCategories.toggleSelect(row.id)}
                      inputProps={{ "aria-label": `Select ${row.name}` }}
                      data-testid={`faq-check-${row.id}`}
                    />
                  </TableCell>
                  <TableCell>
                    <Button variant="text" size="small" onClick={() => void faqCategories.openEdit(row.id)} data-testid={`faq-edit-${row.id}`} sx={{ textTransform: "none" }}>
                      {row.name}
                    </Button>
                  </TableCell>
                  <TableCell>
                    <Chip size="small" label={row.ispublic ? "Public" : "Internal"} color={row.ispublic ? "success" : "default"} variant="outlined" data-testid={`faq-type-${row.id}`} />
                  </TableCell>
                  <TableCell data-testid={`faq-desc-${row.id}`}>{row.description}</TableCell>
                  <TableCell align="right">
                    <Button size="small" color="error" onClick={() => setDeleteId(row.id)} data-testid={`faq-delete-${row.id}`}>
                      Delete
                    </Button>
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
              ? `Delete ${faqCategories.selectedIds.length} category(ies)?`
              : `Make ${faqCategories.selectedIds.length} category(ies) ${confirm === "makepublic" ? "public" : "internal"}?`}
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setConfirm(null)}>Cancel</Button>
          <Button variant="contained" color={confirm === "delete" ? "error" : "primary"} onClick={() => void runMass()} data-testid="faq-mass-confirm">
            Confirm
          </Button>
        </DialogActions>
      </Dialog>

      <Dialog open={deleteId !== null} onClose={() => setDeleteId(null)}>
        <DialogTitle>Delete Category</DialogTitle>
        <DialogContent>
          <DialogContentText>Delete this FAQ category? This cannot be undone.</DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setDeleteId(null)}>Cancel</Button>
          <Button variant="contained" color="error" onClick={() => void runDelete()} data-testid="faq-delete-confirm">
            Delete
          </Button>
        </DialogActions>
      </Dialog>

      <FaqCategoryFormDialog />
    </Box>
  );
});

/** Create / edit FAQ-category dialog (TS-M4-E2). */
const FaqCategoryFormDialog = observer(function FaqCategoryFormDialog() {
  const { faqCategories } = useStores();
  if (!faqCategories.formOpen) return null;

  const onSubmit = async (e: FormEvent) => {
    e.preventDefault();
    await faqCategories.save();
  };

  return (
    <Dialog open onClose={() => faqCategories.closeForm()} maxWidth="sm" fullWidth>
      <DialogTitle>{faqCategories.isEditing ? "Edit Category" : "Add Category"}</DialogTitle>
      <Box component="form" onSubmit={onSubmit} noValidate>
        <DialogContent dividers>
          <Stack spacing={2}>
            <TextField
              label="Name"
              value={faqCategories.name}
              onChange={(e) => faqCategories.setName(e.target.value)}
              error={Boolean(faqCategories.fieldError("name"))}
              helperText={faqCategories.fieldError("name") ?? " "}
              fullWidth
              inputProps={{ "data-testid": "faq-name" }}
            />
            <FormControlLabel
              control={<Checkbox checked={faqCategories.ispublic} onChange={(e) => faqCategories.setIspublic(e.target.checked)} data-testid="faq-ispublic" />}
              label="Public (visible to end users)"
            />
            <TextField
              label="Description"
              multiline
              rows={3}
              value={faqCategories.description}
              onChange={(e) => faqCategories.setDescription(e.target.value)}
              error={Boolean(faqCategories.fieldError("description"))}
              helperText={faqCategories.fieldError("description") ?? " "}
              fullWidth
              inputProps={{ "data-testid": "faq-description" }}
            />
            <TextField
              label="Internal Notes"
              multiline
              rows={2}
              value={faqCategories.notes}
              onChange={(e) => faqCategories.setNotes(e.target.value)}
              fullWidth
              inputProps={{ "data-testid": "faq-notes" }}
            />
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => faqCategories.closeForm()}>Cancel</Button>
          <Button type="submit" variant="contained" disabled={faqCategories.saving} data-testid="faq-save">
            {faqCategories.saving ? "Saving…" : "Save"}
          </Button>
        </DialogActions>
      </Box>
    </Dialog>
  );
});
