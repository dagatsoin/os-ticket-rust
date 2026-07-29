// System Settings screen (TS-M4-A2/A3): the seven-tab strip (FS-032.1) over the
// shared AdminSettingsStore. Attachments is intentionally NOT here — it is a
// separate screen at /staff/admin/settings/attachments.
//
// @implements FS-032.1: settings panel entry & tab routing.
import { useEffect, useState } from "react";
import { observer } from "mobx-react-lite";
import { Alert, Box, Button, CircularProgress, Stack, Tab, Tabs, Typography } from "@mui/material";
import { AttachFile } from "@mui/icons-material";
import { Link as RouterLink } from "react-router-dom";
import { useStores } from "../../stores/StoreContext";
import { SETTINGS_TABS } from "../../stores/adminSettingsSchema";
import { SettingsTabForm } from "./SettingsTabForm";

export const SettingsPage = observer(function SettingsPage() {
  const { adminSettings } = useStores();
  const [active, setActive] = useState(0);

  useEffect(() => {
    void adminSettings.load();
  }, [adminSettings]);

  if (adminSettings.loadError) {
    return (
      <Box>
        <Alert severity="error" sx={{ mb: 2 }}>
          {adminSettings.loadError}
        </Alert>
        <Button variant="outlined" onClick={() => void adminSettings.load()}>
          Retry
        </Button>
      </Box>
    );
  }

  if (!adminSettings.loaded) {
    return (
      <Box sx={{ display: "flex", justifyContent: "center", py: 8 }}>
        <CircularProgress />
      </Box>
    );
  }

  const tab = SETTINGS_TABS[active];

  return (
    <Box>
      {/* Header row: title + a visible link to the separate Attachment Settings
          screen (/staff/admin/settings/attachments), which is otherwise only
          reachable by typing the URL. */}
      <Stack
        direction="row"
        alignItems="center"
        justifyContent="space-between"
        sx={{ mb: 1 }}
      >
        <Typography variant="h5">System Settings</Typography>
        <Button
          variant="outlined"
          size="small"
          startIcon={<AttachFile />}
          component={RouterLink}
          to="/staff/admin/settings/attachments"
          data-testid="attachments-settings-link"
        >
          Attachment Settings
        </Button>
      </Stack>
      <Tabs
        value={active}
        onChange={(_e, v: number) => setActive(v)}
        variant="scrollable"
        scrollButtons="auto"
        sx={{ borderBottom: 1, borderColor: "divider", mb: 3 }}
      >
        {SETTINGS_TABS.map((t) => (
          <Tab key={t.id} label={t.label} />
        ))}
      </Tabs>
      <SettingsTabForm tab={tab} />
    </Box>
  );
});
