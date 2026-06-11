// Realm-agnostic presentational primitive: a generic credential form driven by a field
// config. NO store/realm awareness. Staff and client each supply their own fields +
// submit handler. Maps the shared error envelope's per-field errors (422) onto inputs
// and shows the top-level message. Consumed by TS-M1-C4 (staff) and TS-M1-D2 (client).
import { useState, type FormEvent } from "react";
import { Alert, Box, Button, Stack, TextField } from "@mui/material";
import { ApiError } from "../api/types";

export interface CredentialField {
  /** Field name — must match the key used in the error envelope's `fields`. */
  name: string;
  label: string;
  type?: "text" | "email" | "password";
  required?: boolean;
  autoComplete?: string;
}

export interface CredentialFormProps {
  fields: CredentialField[];
  submitLabel?: string;
  /**
   * Submit handler. Resolves on success; should throw an ApiError on failure so the
   * form can surface the top-level message and per-field (422) errors.
   */
  onSubmit: (values: Record<string, string>) => Promise<void>;
}

export function CredentialForm({ fields, submitLabel = "Submit", onSubmit }: CredentialFormProps) {
  const [values, setValues] = useState<Record<string, string>>(() =>
    Object.fromEntries(fields.map((f) => [f.name, ""])),
  );
  const [fieldErrors, setFieldErrors] = useState<Record<string, string>>({});
  const [topError, setTopError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    setSubmitting(true);
    setFieldErrors({});
    setTopError(null);
    try {
      await onSubmit(values);
    } catch (err) {
      if (err instanceof ApiError) {
        // 422 field-error mapping: per-field messages onto inputs, message at the top.
        setFieldErrors(err.fields);
        setTopError(err.message);
      } else {
        setTopError(err instanceof Error ? err.message : "Something went wrong.");
      }
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <Box component="form" onSubmit={handleSubmit} noValidate data-testid="credential-form">
      <Stack spacing={2}>
        {topError ? (
          <Alert severity="error" data-testid="credential-form-error">
            {topError}
          </Alert>
        ) : null}
        {fields.map((field) => (
          <TextField
            key={field.name}
            name={field.name}
            label={field.label}
            type={field.type ?? "text"}
            required={field.required}
            autoComplete={field.autoComplete}
            value={values[field.name] ?? ""}
            onChange={(e) =>
              setValues((prev) => ({ ...prev, [field.name]: e.target.value }))
            }
            error={Boolean(fieldErrors[field.name])}
            helperText={fieldErrors[field.name] ?? " "}
            fullWidth
          />
        ))}
        <Button type="submit" variant="contained" disabled={submitting}>
          {submitting ? "Please wait…" : submitLabel}
        </Button>
      </Stack>
    </Box>
  );
}
