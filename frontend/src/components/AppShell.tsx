// Base layout shell: MUI AppBar + content container. Hosts the health indicator and
// renders the matched route via <Outlet/>. Realm-agnostic (the three branches share it).
import { AppBar, Box, Container, Toolbar, Typography } from "@mui/material";
import { Link as RouterLink, Outlet } from "react-router-dom";
import { HealthIndicator } from "./HealthIndicator";

export function AppShell() {
  return (
    <Box sx={{ display: "flex", flexDirection: "column", minHeight: "100vh" }}>
      <AppBar position="static" color="secondary" enableColorOnDark>
        <Toolbar sx={{ gap: 2 }}>
          <Typography
            variant="h6"
            component={RouterLink}
            to="/"
            sx={{ color: "inherit", textDecoration: "none", flexGrow: 1 }}
          >
            osTicket
          </Typography>
          <HealthIndicator />
        </Toolbar>
      </AppBar>
      <Container maxWidth="md" sx={{ py: 4, flexGrow: 1 }} component="main">
        <Outlet />
      </Container>
    </Box>
  );
}
