// Staff directory screen (TS-M4-B6) — FS-031.9. A read-only table of
// directory-visible colleagues with a search box (submit "Filter"), a department
// filter, sortable headers, and pagination. No edit/add/delete controls.
// Non-admin gated (RequireStaff). Wired to DirectoryStore.
//
// @implements FS-031.9: directory browse + search + dept filter + paginate.
import { useEffect, useState, type FormEvent } from "react";
import { observer } from "mobx-react-lite";
import {
  Alert,
  Box,
  Button,
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
import type { DirectorySortKey } from "../../stores/DirectoryStore";

const COLUMNS: Array<{ key: DirectorySortKey; label: string }> = [
  { key: "name", label: "Name" },
  { key: "dept", label: "Department" },
  { key: "email", label: "Email" },
  { key: "phone", label: "Phone" },
  { key: "ext", label: "Ext" },
  { key: "mobile", label: "Mobile" },
];

export const DirectoryPage = observer(function DirectoryPage() {
  const { directory } = useStores();
  const [term, setTerm] = useState("");

  useEffect(() => {
    void directory.loadList();
  }, [directory]);

  const onSearch = (e: FormEvent) => {
    e.preventDefault();
    void directory.search(term);
  };

  return (
    <Box>
      <Typography variant="h5" sx={{ mb: 2 }}>Staff Directory</Typography>

      <Stack direction={{ xs: "column", sm: "row" }} spacing={2} sx={{ mb: 2 }} component="form" onSubmit={onSearch}>
        <TextField
          size="small"
          placeholder="Search name, email, or phone…"
          value={term}
          onChange={(e) => setTerm(e.target.value)}
          inputProps={{ "data-testid": "directory-search-input" }}
          sx={{ minWidth: 280 }}
        />
        <Button type="submit" variant="contained" data-testid="directory-search-submit">
          Filter
        </Button>
        <FormControl size="small" sx={{ minWidth: 200 }}>
          <InputLabel id="directory-dept-label">Department</InputLabel>
          <Select
            labelId="directory-dept-label"
            label="Department"
            value={directory.deptFilter}
            onChange={(e) => directory.setDeptFilter(String(e.target.value))}
            data-testid="directory-dept-filter"
          >
            <MenuItem value="">All departments</MenuItem>
            {directory.deptOptions.map((d) => (
              <MenuItem key={d} value={d}>{d}</MenuItem>
            ))}
          </Select>
        </FormControl>
      </Stack>

      {directory.error ? <Alert severity="error" sx={{ mb: 2 }}>{directory.error}</Alert> : null}

      <Paper variant="outlined">
        <Table size="small">
          <TableHead>
            <TableRow>
              {COLUMNS.map((c) => (
                <TableCell key={c.key} sortDirection={directory.sort === c.key ? (directory.order === "ASC" ? "asc" : "desc") : false}>
                  <TableSortLabel
                    active={directory.sort === c.key}
                    direction={directory.sort === c.key && directory.order === "DESC" ? "desc" : "asc"}
                    onClick={() => void directory.setSort(c.key)}
                    data-testid={`directory-sort-${c.key}`}
                  >
                    {c.label}
                  </TableSortLabel>
                </TableCell>
              ))}
            </TableRow>
          </TableHead>
          <TableBody>
            {directory.visibleRows.length === 0 ? (
              <TableRow>
                <TableCell colSpan={COLUMNS.length}>
                  <Typography color="text.secondary">
                    {directory.loading ? "Loading…" : "No staff found."}
                  </Typography>
                </TableCell>
              </TableRow>
            ) : (
              directory.visibleRows.map((r) => (
                <TableRow key={r.id} hover data-testid="directory-row">
                  <TableCell>{r.name}</TableCell>
                  <TableCell>{r.dept_name}</TableCell>
                  <TableCell>{r.email}</TableCell>
                  <TableCell>{r.phone}</TableCell>
                  <TableCell>{r.phone_ext}</TableCell>
                  <TableCell>{r.mobile}</TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </Paper>

      {directory.pagination ? (
        <Pagination pagination={directory.pagination} onPageChange={(p) => void directory.setPage(p)} />
      ) : null}
    </Box>
  );
});
