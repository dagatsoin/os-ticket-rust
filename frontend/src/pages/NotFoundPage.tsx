// Catch-all 404. Ensures no router branch falls through to a blank screen (AC-3).
import { Button, Stack, Typography } from "@mui/material";
import { Link as RouterLink } from "react-router-dom";

export function NotFoundPage() {
  return (
    <Stack spacing={2}>
      <Typography variant="h5">Page not found</Typography>
      <Typography color="text.secondary">
        The page you were looking for does not exist.
      </Typography>
      <Button component={RouterLink} to="/" variant="outlined" sx={{ alignSelf: "flex-start" }}>
        Back to home
      </Button>
    </Stack>
  );
}
