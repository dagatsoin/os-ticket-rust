// Own-profile store (TS-M4-B6) — FS-031.10/.11/.13. Loads the authenticated
// staff member's own record, edits the contact/preference fields (username is
// read-only), runs the ordered password-change sub-form, and derives the
// forced-password-change + on-vacation banner flags. Also the source of the
// GLOBAL forced-change banner (ensureLoaded()), since GET /api/staff/me does not
// carry change_passwd — the flag lives on GET /api/staff/profile.
//
// TDD-covered in ProfileStore.test.ts: snake→camel load mapping, camelCase update
// payload, ordered password validation, 422 → inline errors, banner derivation.
//
// @implements FS-031.10: own profile view + edit (username read-only).
// @implements FS-031.11: forced-password-change + vacation notices.
// @implements FS-031.13: ordered own-profile password-change validation.
// @implements BS-031-013: password rules; clear forced-change on success.
import { makeAutoObservable, runInAction, toJS } from "mobx";
import { ApiClient } from "../api/apiClient";
import { ApiError } from "../api/types";
import { SnackbarStore } from "./SnackbarStore";

const PROFILE_PATH = "/api/staff/profile";
const PASSWORD_PATH = "/api/staff/profile/password";

/** Wire shape of GET /api/staff/profile (snake_case). */
export interface ProfilePayload {
  id: number;
  username: string;
  firstname: string;
  lastname: string;
  email: string;
  phone: string;
  phone_ext: string;
  mobile: string;
  signature: string;
  timezone_id: number | null;
  daylight_saving: boolean;
  max_page_size: number;
  auto_refresh_rate: number;
  default_signature_type: string;
  default_paper_size: string;
  change_passwd: boolean;
  onvacation: boolean;
  dept_id: number;
}

/** Editable working values (strings/booleans for MUI inputs). */
export interface ProfileFormValues {
  firstname: string;
  lastname: string;
  email: string;
  phone: string;
  phoneExt: string;
  mobile: string;
  signature: string;
  timezoneId: string;
  daylightSaving: boolean;
  maxPageSize: string;
  autoRefreshRate: string;
  defaultSignatureType: string;
  defaultPaperSize: string;
}

/** Password sub-form working values. */
export interface PasswordFormValues {
  current: string;
  new: string;
  confirm: string;
}

function toForm(p: ProfilePayload): ProfileFormValues {
  return {
    firstname: p.firstname ?? "",
    lastname: p.lastname ?? "",
    email: p.email ?? "",
    phone: p.phone ?? "",
    phoneExt: p.phone_ext ?? "",
    mobile: p.mobile ?? "",
    signature: p.signature ?? "",
    timezoneId: p.timezone_id == null ? "" : String(p.timezone_id),
    daylightSaving: Boolean(p.daylight_saving),
    maxPageSize: p.max_page_size ? String(p.max_page_size) : "",
    autoRefreshRate: p.auto_refresh_rate ? String(p.auto_refresh_rate) : "",
    defaultSignatureType: p.default_signature_type ?? "none",
    defaultPaperSize: p.default_paper_size ?? "Letter",
  };
}

export class ProfileStore {
  profile: ProfilePayload | null = null;
  form: ProfileFormValues | null = null;
  private baseline = "";
  loading = false;
  loadError: string | null = null;
  fieldErrors: Record<string, string> = {};
  saving = false;

  // password sub-form
  password: PasswordFormValues = { current: "", new: "", confirm: "" };
  passwordErrors: Record<string, string> = {};
  changingPassword = false;

  private requested = false;

  constructor(
    private readonly api: ApiClient,
    private readonly snackbar: SnackbarStore,
  ) {
    makeAutoObservable<ProfileStore, "api" | "snackbar" | "baseline" | "requested">(
      this,
      { api: false, snackbar: false, baseline: false, requested: false },
      { autoBind: true },
    );
  }

  /** Idempotent best-effort load — used by the global forced-change banner. */
  ensureLoaded(): void {
    if (this.requested) return;
    this.requested = true;
    void this.load();
  }

  async load(): Promise<void> {
    this.loading = true;
    this.loadError = null;
    try {
      const p = await this.api.get<ProfilePayload>(PROFILE_PATH);
      runInAction(() => {
        this.profile = p;
        this.form = toForm(p);
        this.baseline = JSON.stringify(this.form);
        this.requested = true;
      });
    } catch (e) {
      runInAction(() => {
        this.loadError = e instanceof ApiError ? e.message : String(e);
      });
    } finally {
      runInAction(() => {
        this.loading = false;
      });
    }
  }

  // --- banner derivation ---
  get mustChangePassword(): boolean {
    return Boolean(this.profile?.change_passwd);
  }
  get onVacation(): boolean {
    return Boolean(this.profile?.onvacation);
  }
  get username(): string {
    return this.profile?.username ?? "";
  }

  // --- profile form ---
  setField<K extends keyof ProfileFormValues>(key: K, value: ProfileFormValues[K]): void {
    if (!this.form) return;
    this.form[key] = value;
    if (this.fieldErrors[key as string]) {
      const { [key as string]: _drop, ...rest } = this.fieldErrors;
      this.fieldErrors = rest;
    }
  }
  fieldError(key: string): string | undefined {
    return this.fieldErrors[key];
  }
  get dirty(): boolean {
    return this.form != null && JSON.stringify(toJS(this.form)) !== this.baseline;
  }

  private updateBody(): Record<string, unknown> {
    const f = this.form!;
    return {
      firstname: f.firstname.trim(),
      lastname: f.lastname.trim(),
      email: f.email.trim(),
      phone: f.phone.trim(),
      phoneExt: f.phoneExt.trim(),
      mobile: f.mobile.trim(),
      signature: f.signature,
      timezoneId: f.timezoneId ? Number(f.timezoneId) : null,
      daylightSaving: f.daylightSaving,
      maxPageSize: f.maxPageSize ? Number(f.maxPageSize) : null,
      autoRefreshRate: f.autoRefreshRate ? Number(f.autoRefreshRate) : null,
      defaultSignatureType: f.defaultSignatureType,
      defaultPaperSize: f.defaultPaperSize,
    };
  }

  /** Save profile edits. PUT echoes the refreshed record; 422 → inline errors. */
  async save(): Promise<boolean> {
    if (!this.form) return false;
    this.saving = true;
    this.fieldErrors = {};
    try {
      const updated = await this.api.put<ProfilePayload>(PROFILE_PATH, this.updateBody());
      runInAction(() => {
        this.profile = updated;
        this.form = toForm(updated);
        this.baseline = JSON.stringify(this.form);
      });
      this.snackbar.success("Profile updated successfully");
      return true;
    } catch (e) {
      if (e instanceof ApiError && e.status === 422) {
        runInAction(() => {
          this.fieldErrors = e.fields ?? {};
        });
      } else {
        this.snackbar.error(e instanceof ApiError ? e.message : "Could not update profile.");
      }
      return false;
    } finally {
      runInAction(() => {
        this.saving = false;
      });
    }
  }

  // --- password sub-form ---
  setPasswordField(key: keyof PasswordFormValues, value: string): void {
    this.password[key] = value;
    if (this.passwordErrors[key]) {
      const { [key]: _drop, ...rest } = this.passwordErrors;
      this.passwordErrors = rest;
    }
  }
  passwordError(key: string): string | undefined {
    return this.passwordErrors[key];
  }

  /**
   * Ordered client-side validation mirroring the backend order (FS-031.13):
   * new required → >=6 → confirm match → current required. Returns the first
   * error as a {field,message} pair, or null when the client-side checks pass.
   */
  validatePassword(): { field: keyof PasswordFormValues; message: string } | null {
    if (!this.password.new) return { field: "new", message: "New password required" };
    if (this.password.new.length < 6)
      return { field: "new", message: "Must be at least 6 characters" };
    if (this.password.new !== this.password.confirm)
      return { field: "confirm", message: "Password(s) do not match" };
    if (!this.password.current)
      return { field: "current", message: "Current password required" };
    return null;
  }

  /**
   * Change the password. Runs the ordered client validation first (inline error,
   * no round-trip), then posts; a 422 maps server-side field errors (wrong
   * current / same-as-current). On success clears the forced-change banner.
   */
  async changePassword(): Promise<boolean> {
    const clientErr = this.validatePassword();
    if (clientErr) {
      this.passwordErrors = { [clientErr.field]: clientErr.message };
      return false;
    }
    this.changingPassword = true;
    this.passwordErrors = {};
    try {
      await this.api.put(PASSWORD_PATH, {
        current: this.password.current,
        new: this.password.new,
        confirm: this.password.confirm,
      });
      runInAction(() => {
        this.password = { current: "", new: "", confirm: "" };
        if (this.profile) this.profile = { ...this.profile, change_passwd: false };
      });
      this.snackbar.success("Password changed successfully");
      return true;
    } catch (e) {
      if (e instanceof ApiError && e.status === 422) {
        runInAction(() => {
          this.passwordErrors = e.fields ?? {};
          if (Object.keys(e.fields ?? {}).length === 0 && e.message) {
            this.passwordErrors = { current: e.message };
          }
        });
      } else {
        this.snackbar.error(e instanceof ApiError ? e.message : "Could not change password.");
      }
      return false;
    } finally {
      runInAction(() => {
        this.changingPassword = false;
      });
    }
  }
}
