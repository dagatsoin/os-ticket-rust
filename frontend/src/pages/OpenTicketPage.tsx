// Public "Open a New Ticket" page (TS-M1-B3).
//
// Renders the M1 field set (name / email / subject / message), surfaces the
// store's client-side + server (422) field errors inline, gates submit until the
// form is client-valid, and swaps to a confirmation view showing the returned
// ticket number on success.
//
// @implements BS-011: public open-a-ticket form + confirmation.
// @implements FS-011.8: required-field + email validation surfaced inline.
import { useMemo } from "react";
import { observer } from "mobx-react-lite";
import {
  Alert,
  Box,
  Button,
  Paper,
  Stack,
  TextField,
  Typography,
} from "@mui/material";
import { Link as RouterLink } from "react-router-dom";
import { useStores } from "../stores/StoreContext";
import { OpenTicketStore, type TicketField } from "../stores/OpenTicketStore";

interface FieldSpec {
  name: TicketField;
  label: string;
  type?: "text" | "email";
  multiline?: boolean;
  rows?: number;
  autoComplete?: string;
}

const FIELD_SPECS: FieldSpec[] = [
  { name: "name", label: "Full Name", autoComplete: "name" },
  { name: "email", label: "Email Address", type: "email", autoComplete: "email" },
  { name: "subject", label: "Subject" },
  { name: "message", label: "Message", multiline: true, rows: 6 },
];

export const OpenTicketPage = observer(function OpenTicketPage() {
  const { clientApi } = useStores();
  // A fresh store per mount so each visit starts clean (and resets after submit).
  const store = useMemo(() => new OpenTicketStore(clientApi), [clientApi]);

  if (store.submitted && store.ticketNumber) {
    return (
      <Stack spacing={3} data-testid="open-ticket-confirmation">
        <Typography variant="h5">Ticket Created</Typography>
        <Alert severity="success">
          Your ticket <strong>{store.ticketNumber}</strong> has been created. A confirmation
          has been sent to your email address.
        </Alert>
        <Typography color="text.secondary">
          Reference this number when you sign in to the client portal to follow up.
        </Typography>
        <Box>
          <Stack direction="row" spacing={2}>
            <Button component={RouterLink} to="/tickets/login" variant="contained">
              Go to Client Portal
            </Button>
            <Button component={RouterLink} to="/" variant="text">
              Back Home
            </Button>
          </Stack>
        </Box>
      </Stack>
    );
  }

  return (
    <Stack spacing={3}>
      <Typography variant="h5">Open a New Ticket</Typography>
      <Typography color="text.secondary">
        Fields marked with <span aria-hidden>*</span> are required.
      </Typography>

      <Paper variant="outlined" sx={{ p: 3 }}>
        <Box
          component="form"
          noValidate
          data-testid="open-ticket-form"
          onSubmit={(e) => {
            e.preventDefault();
            void store.submit();
          }}
        >
          <Stack spacing={2}>
            {store.topError ? (
              <Alert severity="error" data-testid="open-ticket-error">
                {store.topError}
              </Alert>
            ) : null}

            {FIELD_SPECS.map((spec) => (
              <TextField
                key={spec.name}
                name={spec.name}
                label={spec.label}
                type={spec.type ?? "text"}
                required
                multiline={spec.multiline}
                rows={spec.rows}
                autoComplete={spec.autoComplete}
                value={store.values[spec.name]}
                onChange={(e) => store.setValue(spec.name, e.target.value)}
                onBlur={() => store.touch(spec.name)}
                error={Boolean(store.fieldErrors[spec.name])}
                helperText={store.fieldErrors[spec.name] ?? " "}
                fullWidth
              />
            ))}

            <Button
              type="submit"
              variant="contained"
              disabled={!store.canSubmit}
            >
              {store.submitting ? "Submitting…" : "Submit Ticket"}
            </Button>
          </Stack>
        </Box>
      </Paper>
    </Stack>
  );
});
