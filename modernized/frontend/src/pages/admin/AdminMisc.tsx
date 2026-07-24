// Small shared admin-area screens (TS-M4-A0):
//  - AdminIndex: the /staff/admin index. Admins land on System Settings; a
//    delegated non-admin (in the area via a capability flag) sees a neutral home
//    instead of the admin-only settings, without being redirected out.
//  - AdminAreaHome: the neutral "pick a section" landing for delegated non-admins.
//  - AdminPlaceholder: catch-all body for admin nav entries whose screens land in
//    later M4 epics — prevents a blank outlet when such a nav link is followed.
import { observer } from "mobx-react-lite";
import { Box, Typography } from "@mui/material";
import { useStores } from "../../stores/StoreContext";
import { SettingsPage } from "./SettingsPage";

export function AdminAreaHome() {
  return (
    <Box>
      <Typography variant="h5" gutterBottom>
        Administration
      </Typography>
      <Typography color="text.secondary">Select a section from the menu.</Typography>
    </Box>
  );
}

export const AdminIndex = observer(function AdminIndex() {
  const { staffAuth } = useStores();
  return staffAuth.isAdmin ? <SettingsPage /> : <AdminAreaHome />;
});

export function AdminPlaceholder() {
  return (
    <Box>
      <Typography variant="h5" gutterBottom>
        Coming soon
      </Typography>
      <Typography color="text.secondary">
        This section is delivered in a later part of the admin milestone.
      </Typography>
    </Box>
  );
}
