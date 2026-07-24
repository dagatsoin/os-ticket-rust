// One settings field bound to the AdminSettingsStore (TS-M4-A2/A3). Renders the
// right MUI control for the field type, wires value/onChange through the store,
// surfaces inline 422 errors, and disables master-switch-gated sub-fields (BS-032.6).
import { observer } from "mobx-react-lite";
import {
  Checkbox,
  FormControl,
  FormControlLabel,
  FormHelperText,
  InputLabel,
  MenuItem,
  Select,
  TextField,
} from "@mui/material";
import { useStores } from "../../stores/StoreContext";
import type { AdminSettingsStore, SettingsOptions } from "../../stores/AdminSettingsStore";
import type { FieldDef } from "../../stores/adminSettingsSchema";

/** Map a field's option source to concrete {value,label} pairs from the loaded options. */
function optionsFor(field: FieldDef, options: SettingsOptions): { value: string; label: string }[] {
  if (field.staticOptions) {
    return field.staticOptions.map((o) => ({ value: o.id, label: o.label }));
  }
  switch (field.optionSource) {
    case "email_accounts":
      return options.email_accounts.map((o) => ({ value: String(o.id), label: o.email }));
    case "timezones":
      return options.timezones.map((o) => ({ value: String(o.id), label: o.label }));
    case "departments":
    case "sla_plans":
    case "help_topics":
    case "priorities":
    case "template_groups":
    case "pages":
      return options[field.optionSource].map((o) => ({ value: String(o.id), label: o.name }));
    default:
      return [];
  }
}

export const SettingsField = observer(function SettingsField({
  tabId,
  field,
}: {
  tabId: string;
  field: FieldDef;
}) {
  const { adminSettings } = useStores() as { adminSettings: AdminSettingsStore };
  const value = adminSettings.values(tabId)[field.key];
  const error = adminSettings.fieldError(tabId, field.key);

  // BS-032.6: a field gated by a master switch is disabled while the switch is off.
  const disabled = field.dependsOn ? !adminSettings.values(tabId)[field.dependsOn] : false;

  if (field.type === "checkbox") {
    return (
      <FormControl error={Boolean(error)} disabled={disabled}>
        <FormControlLabel
          control={
            <Checkbox
              checked={Boolean(value)}
              onChange={(e) => adminSettings.setValue(tabId, field.key, e.target.checked)}
              data-testid={`field-${field.key}`}
            />
          }
          label={field.label}
        />
        {(error || field.helper) && <FormHelperText>{error ?? field.helper}</FormHelperText>}
      </FormControl>
    );
  }

  if (field.type === "select") {
    const opts = optionsFor(field, adminSettings.options);
    const labelId = `settings-${tabId}-${field.key}-label`;
    return (
      <FormControl fullWidth size="small" error={Boolean(error)} disabled={disabled}>
        <InputLabel id={labelId}>{field.label}</InputLabel>
        <Select
          labelId={labelId}
          label={field.label}
          value={value == null ? "" : String(value)}
          onChange={(e) => adminSettings.setValue(tabId, field.key, e.target.value)}
          data-testid={`field-${field.key}`}
        >
          {!field.required && (
            <MenuItem value="">
              <em>None</em>
            </MenuItem>
          )}
          {opts.map((o) => (
            <MenuItem key={o.value} value={o.value}>
              {o.label}
            </MenuItem>
          ))}
        </Select>
        {(error || field.helper) && <FormHelperText>{error ?? field.helper}</FormHelperText>}
      </FormControl>
    );
  }

  // text / number — number rendered as a text input (inputMode numeric) so that
  // invalid input like "abc" is entered and sent to the server to be rejected (FS-032.2).
  return (
    <TextField
      label={field.label}
      value={value == null ? "" : String(value)}
      onChange={(e) => adminSettings.setValue(tabId, field.key, e.target.value)}
      error={Boolean(error)}
      helperText={error ?? field.helper}
      disabled={disabled}
      size="small"
      fullWidth
      inputProps={{
        "data-testid": `field-${field.key}`,
        ...(field.type === "number" ? { inputMode: "numeric" } : {}),
      }}
    />
  );
});
