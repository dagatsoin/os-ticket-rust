// Public open-ticket form. Placeholder — the real form UI lands with TS-M1-B3
// (built on the shared apiClient + CredentialForm/field patterns).
import { Stack, Typography } from "@mui/material";

export function OpenTicketPage() {
  return (
    <Stack spacing={2}>
      <Typography variant="h5">Open a New Ticket</Typography>
      <Typography color="text.secondary">
        The ticket submission form will be implemented in TS-M1-B3.
      </Typography>
    </Stack>
  );
}
