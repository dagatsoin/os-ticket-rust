// Base layout shell: MUI AppBar + content container. Hosts the health indicator,
// the staff user menu (Profile / Directory / Logout, TS-M4-B6), and the global
// forced-password-change banner. Renders the matched route via <Outlet/>.
// Realm-agnostic (the three branches share it); the staff menu + banner only
// appear once a staff session is authenticated.
//
// @implements FS-031.11: forced-password-change banner (global, from the profile
//   `change_passwd` flag which /api/staff/me does not carry).
import { useEffect, useState, type MouseEvent } from "react";
import { observer } from "mobx-react-lite";
import {
  Alert,
  AppBar,
  Box,
  Button,
  Container,
  IconButton,
  Menu,
  MenuItem,
  Toolbar,
  Typography,
} from "@mui/material";
import { AccountCircle } from "@mui/icons-material";
import { Link as RouterLink, Outlet, useNavigate } from "react-router-dom";
import { HealthIndicator } from "./HealthIndicator";
import { AppSnackbar } from "./AppSnackbar";
import { useStores } from "../stores/StoreContext";
import { DELEGATED_CAPABILITIES } from "../routes/adminCapabilities";

/** Staff account menu — Administration / Profile / Directory / Logout (TS-M4-B6). */
const StaffUserMenu = observer(function StaffUserMenu() {
  const { staffAuth } = useStores();
  const navigate = useNavigate();
  const [anchorEl, setAnchorEl] = useState<null | HTMLElement>(null);

  if (!staffAuth.isAuthenticated) return null;

  // Mirror the RequireAdminArea gate (routes/guards.tsx): admin OR any delegated
  // capability may reach /staff/admin, so only those users see the entry. A plain
  // agent never sees it. Ensure the profile is loaded so the capability flags are
  // available (the forced-password banner already triggers this, but a menu opened
  // before that resolves would otherwise mis-hide the link).
  const canReachAdminArea =
    staffAuth.isAdmin || DELEGATED_CAPABILITIES.some((f) => staffAuth.can(f));

  const open = (e: MouseEvent<HTMLElement>) => setAnchorEl(e.currentTarget);
  const close = () => setAnchorEl(null);
  const go = (path: string) => {
    close();
    navigate(path);
  };
  const logout = async () => {
    close();
    await staffAuth.logout();
    navigate("/staff/login");
  };

  return (
    <>
      <IconButton
        color="inherit"
        onClick={open}
        aria-label="Staff account menu"
        data-testid="staff-user-menu-button"
      >
        <AccountCircle />
      </IconButton>
      <Menu anchorEl={anchorEl} open={Boolean(anchorEl)} onClose={close}>
        {canReachAdminArea && (
          <MenuItem onClick={() => go("/staff/admin")} data-testid="menu-admin">
            Administration
          </MenuItem>
        )}
        <MenuItem onClick={() => go("/staff/profile")} data-testid="menu-profile">
          Profile
        </MenuItem>
        <MenuItem onClick={() => go("/staff/directory")} data-testid="menu-directory">
          Directory
        </MenuItem>
        <MenuItem onClick={logout} data-testid="menu-logout">
          Logout
        </MenuItem>
      </Menu>
    </>
  );
});

/** Global forced-password-change banner (TS-M4-B6 / FS-031.11). */
const ForcedPasswordBanner = observer(function ForcedPasswordBanner() {
  const { staffAuth, profile } = useStores();

  // Once a staff session is authenticated, ensure the profile is loaded — /me
  // does not carry `change_passwd`, so the banner flag lives on the profile.
  useEffect(() => {
    if (staffAuth.isAuthenticated) profile.ensureLoaded();
  }, [staffAuth.isAuthenticated, profile]);

  if (!staffAuth.isAuthenticated || !profile.mustChangePassword) return null;

  return (
    <Alert
      severity="warning"
      data-testid="forced-password-banner"
      sx={{ borderRadius: 0 }}
      action={
        <Button color="inherit" size="small" component={RouterLink} to="/staff/profile">
          Change password
        </Button>
      }
    >
      You must change your password to continue!
    </Alert>
  );
});

export const AppShell = observer(function AppShell() {
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
          <StaffUserMenu />
        </Toolbar>
      </AppBar>
      <ForcedPasswordBanner />
      <Container maxWidth="lg" sx={{ py: 4, flexGrow: 1 }} component="main">
        <Outlet />
      </Container>
      <AppSnackbar />
    </Box>
  );
});
