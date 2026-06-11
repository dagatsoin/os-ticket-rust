// Staff area branch (/staff/*). Placeholder views — the staff login + queue + ticket
// detail UI lands with TS-M1-C4 (on the shared apiClient, staffAuth store, ThreadView,
// and CredentialForm).
import { Stack, Typography } from "@mui/material";

export function StaffLoginPage() {
  return (
    <Stack spacing={2}>
      <Typography variant="h5">Staff Sign In</Typography>
      <Typography color="text.secondary">
        Staff login form (CredentialForm + staffAuth) lands in TS-M1-C4.
      </Typography>
    </Stack>
  );
}

export function StaffDashboardPage() {
  return (
    <Stack spacing={2}>
      <Typography variant="h5">Staff Dashboard</Typography>
      <Typography color="text.secondary">
        The ticket queue and detail/reply views land in TS-M1-C4.
      </Typography>
    </Stack>
  );
}
