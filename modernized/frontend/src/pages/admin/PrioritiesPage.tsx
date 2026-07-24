// Read-only priorities reference panel (TS-M4-D3) — FS-032.8 / BS-032.13. A plain
// table of the fixed, urgency-ranked priority set (name, rank, colour). KL-032.1
// is PRESERVED: there are deliberately NO Add / Edit / Delete controls and no
// selection checkboxes — this is a reference view only.
//
// @implements FS-032.8: read-only priority reference set.
// @implements BS-032.13: priorities are a fixed, urgency-ranked, non-editable set.
// @implements KL-032.1: no CRUD controls (read-only panel).
import { useEffect } from "react";
import { observer } from "mobx-react-lite";
import {
  Alert,
  Box,
  Paper,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableRow,
  Typography,
} from "@mui/material";
import { useStores } from "../../stores/StoreContext";

export const PrioritiesPage = observer(function PrioritiesPage() {
  const { priorities } = useStores();

  useEffect(() => {
    void priorities.loadList();
  }, [priorities]);

  return (
    <Box>
      <Typography variant="h5" sx={{ mb: 1 }}>
        Priorities
      </Typography>
      <Typography color="text.secondary" sx={{ mb: 2 }}>
        The ticket priority scale is a fixed, urgency-ranked set and is not editable.
      </Typography>

      {priorities.error ? <Alert severity="error" sx={{ mb: 2 }}>{priorities.error}</Alert> : null}

      <Paper variant="outlined">
        <Table size="small">
          <TableHead>
            <TableRow>
              <TableCell>Name</TableCell>
              <TableCell>Urgency Rank</TableCell>
              <TableCell>Colour</TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {priorities.rows.length === 0 ? (
              <TableRow>
                <TableCell colSpan={3}>
                  <Typography color="text.secondary">
                    {priorities.loading ? "Loading…" : "No priorities found."}
                  </Typography>
                </TableCell>
              </TableRow>
            ) : (
              priorities.rows.map((row) => (
                <TableRow key={row.priority_id} data-testid="priority-row">
                  <TableCell data-testid={`priority-name-${row.priority_id}`}>{row.priority}</TableCell>
                  <TableCell data-testid={`priority-rank-${row.priority_id}`}>{row.urgency}</TableCell>
                  <TableCell>
                    <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
                      <Box
                        sx={{
                          width: 18,
                          height: 18,
                          borderRadius: 0.5,
                          border: "1px solid",
                          borderColor: "divider",
                          bgcolor: row.priority_color,
                        }}
                        data-testid={`priority-swatch-${row.priority_id}`}
                        aria-label={`Colour ${row.priority_color}`}
                      />
                      <Typography variant="body2" color="text.secondary">
                        {row.priority_color}
                      </Typography>
                    </Box>
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </Paper>
    </Box>
  );
});
