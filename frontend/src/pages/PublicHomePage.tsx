// Public landing — links into the open-ticket form and the two portals.
// Placeholder; the open-ticket form UI lands with TS-M1-B3.
import { Box, Button, Stack, Typography } from "@mui/material";
import { Link as RouterLink } from "react-router-dom";

export function PublicHomePage() {
  return (
    <Stack spacing={3}>
      <Typography variant="h4">Support Center</Typography>
      <Typography color="text.secondary">
        Open a support ticket, or sign in to an existing account.
      </Typography>
      <Box>
        <Stack direction="row" spacing={2}>
          <Button component={RouterLink} to="/open" variant="contained">
            Open a Ticket
          </Button>
          <Button component={RouterLink} to="/tickets" variant="outlined">
            Client Portal
          </Button>
          <Button component={RouterLink} to="/staff" variant="text">
            Staff Sign In
          </Button>
        </Stack>
      </Box>
    </Stack>
  );
}
