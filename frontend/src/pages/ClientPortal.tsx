// Client portal branch (/tickets/*). Placeholder views — the client login + ticket
// list + read-only thread view lands with TS-M1-D2 (on the shared apiClient, clientAuth
// store, ThreadView read-only, and CredentialForm).
import { Stack, Typography } from "@mui/material";

export function ClientLoginPage() {
  return (
    <Stack spacing={2}>
      <Typography variant="h5">Client Sign In</Typography>
      <Typography color="text.secondary">
        Client login form (CredentialForm + clientAuth) lands in TS-M1-D2.
      </Typography>
    </Stack>
  );
}

export function ClientTicketsPage() {
  return (
    <Stack spacing={2}>
      <Typography variant="h5">My Tickets</Typography>
      <Typography color="text.secondary">
        The client ticket list and read-only thread view land in TS-M1-D2.
      </Typography>
    </Stack>
  );
}
