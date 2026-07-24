// Attachments settings screen (TS-M4-A3) — a STANDALONE sub-route at
// /staff/admin/settings/attachments, deliberately not a seventh tab (FS-032.1).
// The allow_attachments master switch gates the type/size sub-fields (BS-032.6);
// that gating lives in SettingsField via the field `dependsOn`.
//
// @implements BS-032.6: attachment sub-fields validated only when attachments are on.
import { useEffect } from "react";
import { observer } from "mobx-react-lite";
import { Alert, Box, Button, CircularProgress, Typography } from "@mui/material";
import { useStores } from "../../stores/StoreContext";
import { ATTACHMENTS_TAB } from "../../stores/adminSettingsSchema";
import { SettingsTabForm } from "./SettingsTabForm";

export const AttachmentsSettingsPage = observer(function AttachmentsSettingsPage() {
  const { adminSettings } = useStores();

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

  return (
    <Box>
      <Typography variant="h5" gutterBottom>
        Attachment Settings
      </Typography>
      <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>
        Turn attachments on or off and set the allowed types and size limit.
      </Typography>
      <SettingsTabForm tab={ATTACHMENTS_TAB} />
    </Box>
  );
});
