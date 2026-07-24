// Admin panel shell (TS-M4-A0): a permanent MUI Drawer sidebar (data-driven nav)
// + a content <Outlet/>. Capability-aware — nav entries the current staff member
// cannot use are hidden. Mounted behind RequireAdminArea, so by the time this
// renders the viewer is admin or holds a delegated capability.
import { observer } from "mobx-react-lite";
import { Box, Drawer, List, ListItemButton, ListItemText, Toolbar, Typography } from "@mui/material";
import { Link as RouterLink, Outlet, useLocation } from "react-router-dom";
import { useStores } from "../../stores/StoreContext";
import { ADMIN_NAV, type AdminNavItem } from "./adminNav";

const DRAWER_WIDTH = 220;

function isSelected(item: AdminNavItem, pathname: string): boolean {
  return item.exact ? pathname === item.path : pathname.startsWith(item.path);
}

export const AdminLayout = observer(function AdminLayout() {
  const { staffAuth } = useStores();
  const location = useLocation();

  const items = ADMIN_NAV.filter((item) =>
    item.requiredCapability ? staffAuth.isAdmin || staffAuth.can(item.requiredCapability) : staffAuth.isAdmin,
  );

  return (
    <Box sx={{ display: "flex", gap: 3, alignItems: "flex-start" }}>
      <Drawer
        variant="permanent"
        aria-label="Admin navigation"
        sx={{
          width: DRAWER_WIDTH,
          flexShrink: 0,
          "& .MuiDrawer-paper": {
            width: DRAWER_WIDTH,
            position: "static",
            boxSizing: "border-box",
            border: 0,
          },
        }}
      >
        <Toolbar disableGutters sx={{ px: 2, minHeight: "auto", py: 1 }}>
          <Typography variant="overline" color="text.secondary">
            Administration
          </Typography>
        </Toolbar>
        <List dense component="nav">
          {items.map((item) => (
            <ListItemButton
              key={item.path}
              component={RouterLink}
              to={item.path}
              selected={isSelected(item, location.pathname)}
            >
              <ListItemText primary={item.label} />
            </ListItemButton>
          ))}
        </List>
      </Drawer>

      <Box sx={{ flexGrow: 1, minWidth: 0 }}>
        <Outlet />
      </Box>
    </Box>
  );
});
