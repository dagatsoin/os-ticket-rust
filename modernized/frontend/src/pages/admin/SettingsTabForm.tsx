// A single settings tab (or the Attachments screen) rendered from its TabDef
// (TS-M4-A2/A3): the fields plus a per-tab Save button. Save is enabled only when
// the tab is dirty (dirty tracking) and disabled while a save is in flight.
import { observer } from "mobx-react-lite";
import { Box, Button, Stack } from "@mui/material";
import { useStores } from "../../stores/StoreContext";
import type { AdminSettingsStore } from "../../stores/AdminSettingsStore";
import type { TabDef } from "../../stores/adminSettingsSchema";
import { SettingsField } from "./SettingsField";

export const SettingsTabForm = observer(function SettingsTabForm({ tab }: { tab: TabDef }) {
  const { adminSettings } = useStores() as { adminSettings: AdminSettingsStore };
  const saving = adminSettings.saving(tab.id);
  const dirty = adminSettings.isDirty(tab.id);

  return (
    <Stack spacing={2.5} sx={{ maxWidth: 520 }} component="form" onSubmit={(e) => e.preventDefault()}>
      {tab.fields.map((field) => (
        <SettingsField key={field.key} tabId={tab.id} field={field} />
      ))}
      <Box>
        <Button
          type="submit"
          variant="contained"
          disabled={!dirty || saving}
          onClick={() => void adminSettings.save(tab.id)}
          data-testid={`save-${tab.id}`}
        >
          {saving ? "Saving…" : "Save"}
        </Button>
      </Box>
    </Stack>
  );
});
