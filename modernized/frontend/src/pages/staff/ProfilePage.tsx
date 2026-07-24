// Own-profile screen (TS-M4-B6) — FS-031.10/.11/.13. Read-only username, editable
// contact + preferences, a password-change section, and the forced-change /
// vacation notice banners. Non-admin gated (RequireStaff). Wired to ProfileStore.
//
// Timezone note: there is no non-admin timezone-list endpoint, so the timezone
// field is a Select for admins (options via the settings payload) and a numeric
// "Time Zone (ID)" field for plain agents — both persist `timezone_id`.
// (Contract gap documented in the TS-M4-B6 report.)
//
// @implements FS-031.10: profile edit; username read-only; prefs persist.
// @implements FS-031.11: forced-change + vacation banners.
// @implements FS-031.13: ordered password-change section.
import { useEffect, type FormEvent } from "react";
import { observer } from "mobx-react-lite";
import {
  Alert,
  Box,
  Button,
  CircularProgress,
  Divider,
  FormControl,
  FormControlLabel,
  Checkbox,
  InputLabel,
  MenuItem,
  Paper,
  Select,
  Stack,
  TextField,
  Typography,
} from "@mui/material";
import { useStores } from "../../stores/StoreContext";

const PAGE_SIZES = [10, 25, 50, 100];
const REFRESH_RATES = [
  { v: "0", label: "Disable" },
  { v: "1", label: "1 minute" },
  { v: "2", label: "2 minutes" },
  { v: "5", label: "5 minutes" },
];
const PAPER_SIZES = ["Letter", "Legal", "A4", "A3"];
const SIGNATURE_TYPES = [
  { v: "none", label: "None" },
  { v: "mine", label: "My signature" },
  { v: "dept", label: "Department signature" },
];

export const ProfilePage = observer(function ProfilePage() {
  const { profile, staffAuth, adminSettings } = useStores();

  useEffect(() => {
    void profile.load();
    // Admins can hydrate a proper timezone picker from the settings options.
    if (staffAuth.isAdmin && !adminSettings.loaded) void adminSettings.load();
  }, [profile, staffAuth.isAdmin, adminSettings]);

  if (profile.loading && !profile.form) {
    return (
      <Box sx={{ display: "flex", justifyContent: "center", py: 8 }}>
        <CircularProgress />
      </Box>
    );
  }

  if (profile.loadError && !profile.form) {
    return (
      <Box>
        <Alert severity="error" sx={{ mb: 2 }}>{profile.loadError}</Alert>
        <Button variant="outlined" onClick={() => void profile.load()}>Retry</Button>
      </Box>
    );
  }

  const f = profile.form;
  if (!f) return null;
  const err = (k: string) => profile.fieldError(k);
  const tzOptions = adminSettings.options.timezones ?? [];

  const onSaveProfile = async (e: FormEvent) => {
    e.preventDefault();
    await profile.save();
  };
  const onChangePassword = async (e: FormEvent) => {
    e.preventDefault();
    await profile.changePassword();
  };

  return (
    <Stack spacing={3} sx={{ maxWidth: 720 }}>
      <Typography variant="h5">My Profile</Typography>

      {profile.mustChangePassword ? (
        <Alert severity="warning" data-testid="profile-forced-banner">
          You must change your password to continue!
        </Alert>
      ) : null}
      {profile.onVacation ? (
        <Alert severity="info" data-testid="profile-vacation-banner">
          You are currently marked as on vacation.
        </Alert>
      ) : null}

      {/* Profile form */}
      <Paper variant="outlined" sx={{ p: 3 }}>
        <Box component="form" onSubmit={onSaveProfile} noValidate>
          <Stack spacing={2}>
            <TextField
              label="Username"
              value={profile.username}
              disabled
              fullWidth
              inputProps={{ "data-testid": "profile-username" }}
              helperText="Username cannot be changed."
            />
            <Stack direction={{ xs: "column", sm: "row" }} spacing={2}>
              <TextField
                label="First name"
                value={f.firstname}
                onChange={(e) => profile.setField("firstname", e.target.value)}
                error={Boolean(err("firstname"))}
                helperText={err("firstname") ?? " "}
                fullWidth
              />
              <TextField
                label="Last name"
                value={f.lastname}
                onChange={(e) => profile.setField("lastname", e.target.value)}
                error={Boolean(err("lastname"))}
                helperText={err("lastname") ?? " "}
                fullWidth
              />
            </Stack>
            <TextField
              label="Email"
              type="email"
              value={f.email}
              onChange={(e) => profile.setField("email", e.target.value)}
              error={Boolean(err("email"))}
              helperText={err("email") ?? " "}
              fullWidth
              inputProps={{ "data-testid": "profile-email" }}
            />
            <Stack direction={{ xs: "column", sm: "row" }} spacing={2}>
              <TextField label="Phone" value={f.phone} onChange={(e) => profile.setField("phone", e.target.value)} fullWidth />
              <TextField label="Ext" value={f.phoneExt} onChange={(e) => profile.setField("phoneExt", e.target.value)} sx={{ maxWidth: 120 }} />
              <TextField label="Mobile" value={f.mobile} onChange={(e) => profile.setField("mobile", e.target.value)} fullWidth />
            </Stack>

            <Divider textAlign="left"><Typography variant="overline">Preferences</Typography></Divider>

            <Stack direction={{ xs: "column", sm: "row" }} spacing={2}>
              {tzOptions.length > 0 ? (
                <FormControl fullWidth>
                  <InputLabel id="profile-tz-label">Time Zone</InputLabel>
                  <Select
                    labelId="profile-tz-label"
                    label="Time Zone"
                    value={f.timezoneId}
                    onChange={(e) => profile.setField("timezoneId", String(e.target.value))}
                    data-testid="profile-timezone"
                  >
                    <MenuItem value=""><em>Default</em></MenuItem>
                    {tzOptions.map((t) => (
                      <MenuItem key={t.id} value={String(t.id)}>{t.label}</MenuItem>
                    ))}
                  </Select>
                </FormControl>
              ) : (
                <TextField
                  label="Time Zone (ID)"
                  type="number"
                  value={f.timezoneId}
                  onChange={(e) => profile.setField("timezoneId", e.target.value)}
                  fullWidth
                  inputProps={{ "data-testid": "profile-timezone" }}
                  helperText="Numeric time-zone id."
                />
              )}
              <FormControl fullWidth>
                <InputLabel id="profile-pagesize-label">Default Page Size</InputLabel>
                <Select
                  labelId="profile-pagesize-label"
                  label="Default Page Size"
                  value={f.maxPageSize}
                  onChange={(e) => profile.setField("maxPageSize", String(e.target.value))}
                  data-testid="profile-pagesize"
                >
                  {PAGE_SIZES.map((n) => (
                    <MenuItem key={n} value={String(n)}>{n}</MenuItem>
                  ))}
                </Select>
              </FormControl>
              <FormControl fullWidth>
                <InputLabel id="profile-refresh-label">Auto Refresh</InputLabel>
                <Select
                  labelId="profile-refresh-label"
                  label="Auto Refresh"
                  value={f.autoRefreshRate || "0"}
                  onChange={(e) => profile.setField("autoRefreshRate", String(e.target.value))}
                  data-testid="profile-refresh"
                >
                  {REFRESH_RATES.map((r) => (
                    <MenuItem key={r.v} value={r.v}>{r.label}</MenuItem>
                  ))}
                </Select>
              </FormControl>
            </Stack>

            <Stack direction={{ xs: "column", sm: "row" }} spacing={2}>
              <FormControl fullWidth>
                <InputLabel id="profile-sigtype-label">Signature Usage</InputLabel>
                <Select
                  labelId="profile-sigtype-label"
                  label="Signature Usage"
                  value={f.defaultSignatureType || "none"}
                  onChange={(e) => profile.setField("defaultSignatureType", String(e.target.value))}
                >
                  {SIGNATURE_TYPES.map((s) => (
                    <MenuItem key={s.v} value={s.v}>{s.label}</MenuItem>
                  ))}
                </Select>
              </FormControl>
              <FormControl fullWidth>
                <InputLabel id="profile-paper-label">Paper Size</InputLabel>
                <Select
                  labelId="profile-paper-label"
                  label="Paper Size"
                  value={f.defaultPaperSize || "Letter"}
                  onChange={(e) => profile.setField("defaultPaperSize", String(e.target.value))}
                >
                  {PAPER_SIZES.map((p) => (
                    <MenuItem key={p} value={p}>{p}</MenuItem>
                  ))}
                </Select>
              </FormControl>
            </Stack>

            <TextField
              label="Signature"
              multiline
              rows={3}
              value={f.signature}
              onChange={(e) => profile.setField("signature", e.target.value)}
              fullWidth
            />
            <FormControlLabel
              control={<Checkbox checked={f.daylightSaving} onChange={(e) => profile.setField("daylightSaving", e.target.checked)} />}
              label="Observe daylight saving"
            />

            <Box>
              <Button type="submit" variant="contained" disabled={profile.saving} data-testid="profile-save">
                {profile.saving ? "Saving…" : "Save Profile"}
              </Button>
            </Box>
          </Stack>
        </Box>
      </Paper>

      {/* Password change */}
      <Paper variant="outlined" sx={{ p: 3 }}>
        <Typography variant="h6" gutterBottom>Change Password</Typography>
        <Box component="form" onSubmit={onChangePassword} noValidate>
          <Stack spacing={2}>
            <TextField
              label="Current password"
              type="password"
              value={profile.password.current}
              onChange={(e) => profile.setPasswordField("current", e.target.value)}
              error={Boolean(profile.passwordError("current"))}
              helperText={profile.passwordError("current") ?? " "}
              fullWidth
              inputProps={{ "data-testid": "pw-current" }}
            />
            <TextField
              label="New password"
              type="password"
              value={profile.password.new}
              onChange={(e) => profile.setPasswordField("new", e.target.value)}
              error={Boolean(profile.passwordError("new"))}
              helperText={profile.passwordError("new") ?? " "}
              fullWidth
              inputProps={{ "data-testid": "pw-new" }}
            />
            <TextField
              label="Confirm new password"
              type="password"
              value={profile.password.confirm}
              onChange={(e) => profile.setPasswordField("confirm", e.target.value)}
              error={Boolean(profile.passwordError("confirm"))}
              helperText={profile.passwordError("confirm") ?? " "}
              fullWidth
              inputProps={{ "data-testid": "pw-confirm" }}
            />
            <Box>
              <Button type="submit" variant="contained" disabled={profile.changingPassword} data-testid="pw-save">
                {profile.changingPassword ? "Saving…" : "Change Password"}
              </Button>
            </Box>
          </Stack>
        </Box>
      </Paper>
    </Stack>
  );
});
