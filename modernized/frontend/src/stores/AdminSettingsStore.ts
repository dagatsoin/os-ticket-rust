// Admin System Settings store (TS-M4-A2 / TS-M4-A3). Owns the per-tab working
// values, dirty tracking, inline field errors, and the GET/PUT wiring against
// /api/staff/admin/settings. Business logic (coercion, checkbox 0/1-by-presence
// serialization, 422 → inline field errors, master-switch gating) is TDD-covered
// in AdminSettingsStore.test.ts.
//
// @implements FS-032.2: per-tab save dispatch + validation (only this tab's fields).
// @implements FS-032.7: only the submitting tab's keys are written.
// @implements BS-032.4: checkbox settings persist as 0/1 by presence in the submission.
// @implements BS-032.6: attachment sub-fields gated by the allow_attachments master switch.
import { makeAutoObservable, runInAction, toJS } from "mobx";
import { ApiClient } from "../api/apiClient";
import { ApiError } from "../api/types";
import { SnackbarStore } from "./SnackbarStore";
import { ALL_TABS, tabDef, type TabDef } from "./adminSettingsSchema";

const SETTINGS_PATH = "/api/staff/admin/settings";

/** {id,name}-style option (departments/SLA/topics/priorities/templates/pages). */
export interface IdNameOption {
  id: number | string;
  name: string;
}
/** Email account option — labelled by its address. */
export interface EmailAccountOption {
  id: number | string;
  email: string;
  name?: string;
}
/** Timezone option — labelled by `label`. */
export interface TimezoneOption {
  id: number | string;
  label: string;
}

export interface SettingsOptions {
  departments: IdNameOption[];
  sla_plans: IdNameOption[];
  help_topics: IdNameOption[];
  priorities: IdNameOption[];
  email_accounts: EmailAccountOption[];
  template_groups: IdNameOption[];
  timezones: TimezoneOption[];
  /** Populated once EPIC-M4-F ships GET pages; optional in the pinned contract. */
  pages: IdNameOption[];
}

/** Wire shape of GET /api/staff/admin/settings. */
export interface SettingsPayload {
  tabs: Record<string, Record<string, unknown>>;
  options: Partial<SettingsOptions>;
}

function emptyOptions(): SettingsOptions {
  return {
    departments: [],
    sla_plans: [],
    help_topics: [],
    priorities: [],
    email_accounts: [],
    template_groups: [],
    timezones: [],
    pages: [],
  };
}

/** Truthy interpretation for a checkbox value arriving from the wire (0/1/"1"/bool/"on"). */
function toBool(v: unknown): boolean {
  return v === true || v === 1 || v === "1" || v === "on" || v === "true";
}

/**
 * Coerce a raw wire tab object into editable working values: checkboxes → boolean,
 * everything else → string (MUI controlled inputs). Unknown keys (not in the tab
 * schema) are dropped so a tab only ever carries its own fields (FS-032.7).
 */
function coerceTab(tab: TabDef, raw: Record<string, unknown> = {}): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const f of tab.fields) {
    const v = raw[f.key];
    out[f.key] = f.type === "checkbox" ? toBool(v) : v == null ? "" : String(v);
  }
  return out;
}

/**
 * Build the PUT body for a tab. Verified live (2026-07-24): the backend applies a
 * per-tab save as a WHOLE-tab checkbox reset — any checkbox key it belongs to that
 * is absent from the submission is written 0. Our schema only models a subset of
 * each tab's keys, so to avoid clobbering the unmodelled ones we start from the full
 * loaded tab payload (`raw`, every key at its current value — a present "0" persists
 * 0, a present "1" persists 1) and overlay the editable schema fields on top.
 *
 * Editable checkboxes are sent explicit 1/0 (BS-032.4 — 0/1); other fields are sent
 * verbatim (as strings) so invalid input such as "abc" reaches the server (FS-032.2).
 */
function buildSubmission(
  tab: TabDef,
  working: Record<string, unknown> = {},
  raw: Record<string, unknown> = {},
): Record<string, unknown> {
  // Preserve every loaded key (incl. checkboxes/text this UI does not model).
  const out: Record<string, unknown> = { ...raw };
  for (const f of tab.fields) {
    const v = working[f.key];
    out[f.key] = f.type === "checkbox" ? (v ? 1 : 0) : (v ?? "");
  }
  return out;
}

export class AdminSettingsStore {
  loading = false;
  loaded = false;
  loadError: string | null = null;

  options: SettingsOptions = emptyOptions();

  /** Editable per-tab values (schema fields only), keyed by tab id. */
  private workingByTab: Record<string, Record<string, unknown>> = {};
  /** Full loaded per-tab payload (EVERY key, incl. unmodelled ones), preserved for save. */
  private rawByTab: Record<string, Record<string, unknown>> = {};
  /** Last-saved baseline per tab, for dirty tracking. */
  private baselineByTab: Record<string, Record<string, unknown>> = {};
  /** Per-tab, per-field inline errors (422). */
  fieldErrorsByTab: Record<string, Record<string, string>> = {};
  /** Per-tab in-flight save flags. */
  savingByTab: Record<string, boolean> = {};

  constructor(
    private readonly api: ApiClient,
    private readonly snackbar: SnackbarStore,
  ) {
    makeAutoObservable<AdminSettingsStore, "api" | "snackbar">(
      this,
      { api: false, snackbar: false },
      { autoBind: true },
    );
  }

  /** Fetch the full settings payload and seed per-tab working values + options. */
  async load(): Promise<void> {
    this.loading = true;
    this.loadError = null;
    try {
      const payload = await this.api.get<SettingsPayload>(SETTINGS_PATH);
      runInAction(() => {
        this.options = { ...emptyOptions(), ...(payload.options ?? {}) };
        this.workingByTab = {};
        this.rawByTab = {};
        this.baselineByTab = {};
        this.fieldErrorsByTab = {};
        for (const tab of ALL_TABS) {
          const rawTab = payload.tabs?.[tab.id] ?? {};
          const coerced = coerceTab(tab, rawTab);
          this.rawByTab[tab.id] = { ...rawTab };
          this.workingByTab[tab.id] = coerced;
          this.baselineByTab[tab.id] = { ...coerced };
        }
        this.loaded = true;
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

  /** Current working values for a tab (empty object before load). */
  values(tabId: string): Record<string, unknown> {
    return this.workingByTab[tabId] ?? {};
  }

  /** Inline error for a single field, if any. */
  fieldError(tabId: string, key: string): string | undefined {
    return this.fieldErrorsByTab[tabId]?.[key];
  }

  saving(tabId: string): boolean {
    return Boolean(this.savingByTab[tabId]);
  }

  /** A tab is dirty when its working values diverge from the saved baseline. */
  isDirty(tabId: string): boolean {
    return JSON.stringify(toJS(this.workingByTab[tabId] ?? {})) !== JSON.stringify(this.baselineByTab[tabId] ?? {});
  }

  /** Update one field's working value; clears that field's inline error. */
  setValue(tabId: string, key: string, value: unknown): void {
    const tab = this.workingByTab[tabId] ?? {};
    this.workingByTab[tabId] = { ...tab, [key]: value };
    const errs = this.fieldErrorsByTab[tabId];
    if (errs && key in errs) {
      const rest: Record<string, string> = {};
      for (const k of Object.keys(errs)) {
        if (k !== key) rest[k] = errs[k];
      }
      this.fieldErrorsByTab[tabId] = rest;
    }
  }

  /** BS-032.6: is the attachment master switch on? Gates the dependent sub-fields. */
  get attachmentsEnabled(): boolean {
    return toBool(this.workingByTab["attach"]?.["allow_attachments"]);
  }

  /**
   * PUT the tab's values. Success → refresh baseline + snackbar success. A 422
   * maps `error.fields` to inline per-field errors (no navigation, no success
   * banner, FS-032.2). Any other failure surfaces an error snackbar.
   */
  async save(tabId: string): Promise<void> {
    const tab = tabDef(tabId);
    if (!tab) return;
    this.savingByTab[tabId] = true;
    this.fieldErrorsByTab[tabId] = {};
    try {
      const values = buildSubmission(tab, this.workingByTab[tabId], this.rawByTab[tabId]);
      await this.api.put<{ tab: string; saved?: number; values?: Record<string, unknown> }>(
        SETTINGS_PATH,
        { tab: tabId, values },
      );
      runInAction(() => {
        // The submission (which the server accepted) becomes the new source of truth:
        // full raw payload + a clean baseline. The server echo is partial, so we don't
        // rely on it. Working values are unchanged and now clean (dirty → false).
        this.rawByTab[tabId] = { ...values };
        this.baselineByTab[tabId] = { ...this.workingByTab[tabId] };
      });
      this.snackbar.success(`${tab.label} settings updated`);
    } catch (e) {
      if (e instanceof ApiError && e.status === 422) {
        runInAction(() => {
          this.fieldErrorsByTab[tabId] = e.fields ?? {};
        });
      } else {
        const msg = e instanceof ApiError ? e.message : "Could not save settings. Please try again.";
        this.snackbar.error(msg);
      }
    } finally {
      runInAction(() => {
        this.savingByTab[tabId] = false;
      });
    }
  }
}
