// Admin SLA-plan store (TS-M4-D2) — FS-032.9/.10/.12. Owns the SLA list (with a
// client-side "Date Added" sort — KL-032.10 modernised), the create/edit form
// (name + grace_period + four boolean flags + notes), inline 422 field errors,
// the mass-selection Set, and the default-SLA delete-protection predicate
// (FS-032.12 — the default plan cannot be deleted).
//
// TDD-covered in SlaAdminStore.test.ts: list + date-sort toggle, form dirty
// tracking, 422 → inline errors, mass actions, default-delete-protection.
//
// @implements FS-032.9: SLA list + mass activate/disable/delete.
// @implements FS-032.10: create / edit a plan (name + grace period + flags + notes).
// @implements FS-032.12: the default SLA is delete-protected (client-side guard).
// @implements KL-032.10: "Date Added" is sortable (modernised — was unsortable in 1.7).
import { makeAutoObservable, runInAction } from "mobx";
import { ApiClient } from "../api/apiClient";
import { ApiError } from "../api/types";
import { SnackbarStore } from "./SnackbarStore";

const SLA_PATH = "/api/staff/admin/sla";

/** A row in the SLA list (GET /api/staff/admin/sla). */
export interface SlaRow {
  id: number;
  name: string;
  grace_period: number;
  isactive: boolean;
  enable_priority_escalation: boolean;
  transient: boolean;
  disable_overdue_alerts: boolean;
  notes: string;
  created: string | null;
  updated: string | null;
  is_default: boolean;
}

export type SlaSortKey = "name" | "grace" | "status" | "created";
export type SortOrder = "ASC" | "DESC";
export type SlaMassAction = "activate" | "disable" | "delete";

/** The default-SLA delete-protection message surfaced client-side (FS-032.12). */
export const DEFAULT_SLA_DELETE_MESSAGE = "The default SLA cannot be deleted.";

export class SlaAdminStore {
  // --- list ---
  rows: SlaRow[] = [];
  loadingList = false;
  listError: string | null = null;
  sort: SlaSortKey = "created";
  order: SortOrder = "DESC";

  // --- selection ---
  selected = new Set<number>();

  // --- form ---
  formOpen = false;
  editingId: number | null = null;
  name = "";
  gracePeriod = "";
  isactive = true;
  enablePriorityEscalation = false;
  transient = false;
  disableOverdueAlerts = false;
  notes = "";
  fieldErrors: Record<string, string> = {};
  saving = false;
  private baselineKey = "";

  // --- mass ---
  massLoading = false;
  massError: string | null = null;

  constructor(
    private readonly api: ApiClient,
    private readonly snackbar: SnackbarStore,
  ) {
    makeAutoObservable<SlaAdminStore, "api" | "snackbar" | "baselineKey">(
      this,
      { api: false, snackbar: false, baselineKey: false },
      { autoBind: true },
    );
  }

  // ---------------------------------------------------------------------------
  // List + client-side sort (the GET returns the full set; KL-032.10 modernised).
  // ---------------------------------------------------------------------------

  async loadList(): Promise<void> {
    this.loadingList = true;
    this.listError = null;
    try {
      const data = await this.api.get<SlaRow[]>(SLA_PATH);
      runInAction(() => {
        this.rows = data ?? [];
        const visible = new Set(this.rows.map((r) => r.id));
        for (const id of Array.from(this.selected)) {
          if (!visible.has(id)) this.selected.delete(id);
        }
      });
    } catch (e) {
      runInAction(() => {
        this.listError = e instanceof ApiError ? e.message : String(e);
      });
    } finally {
      runInAction(() => {
        this.loadingList = false;
      });
    }
  }

  /** The rows sorted by the active column/order (pure, derived). */
  get sortedRows(): SlaRow[] {
    const dir = this.order === "ASC" ? 1 : -1;
    const copy = [...this.rows];
    copy.sort((a, b) => {
      let cmp: number;
      switch (this.sort) {
        case "name":
          cmp = a.name.localeCompare(b.name);
          break;
        case "grace":
          cmp = a.grace_period - b.grace_period;
          break;
        case "status":
          cmp = Number(a.isactive) - Number(b.isactive);
          break;
        case "created":
        default:
          cmp = String(a.created ?? "").localeCompare(String(b.created ?? ""));
          break;
      }
      if (cmp === 0) cmp = a.id - b.id;
      return cmp * dir;
    });
    return copy;
  }

  setSort(sort: SlaSortKey): void {
    if (this.sort === sort) {
      this.order = this.order === "ASC" ? "DESC" : "ASC";
    } else {
      this.sort = sort;
      this.order = "ASC";
    }
  }

  // ---------------------------------------------------------------------------
  // Selection + default-SLA delete protection (FS-032.12).
  // ---------------------------------------------------------------------------

  toggleSelect(id: number): void {
    if (this.selected.has(id)) this.selected.delete(id);
    else this.selected.add(id);
  }
  get allSelected(): boolean {
    return this.rows.length > 0 && this.rows.every((r) => this.selected.has(r.id));
  }
  get someSelected(): boolean {
    return this.selected.size > 0 && !this.allSelected;
  }
  toggleSelectAll(): void {
    if (this.allSelected) this.selected.clear();
    else for (const r of this.rows) this.selected.add(r.id);
  }
  clearSelection(): void {
    this.selected.clear();
  }
  get selectedIds(): number[] {
    return Array.from(this.selected);
  }

  /** True when the current selection contains the delete-protected default SLA. */
  get selectionHasDefault(): boolean {
    return this.rows.some((r) => r.is_default && this.selected.has(r.id));
  }

  /** Whether a Delete mass action is currently permitted (no default selected). */
  get canDeleteSelection(): boolean {
    return this.selectedIds.length > 0 && !this.selectionHasDefault;
  }

  // ---------------------------------------------------------------------------
  // Form.
  // ---------------------------------------------------------------------------

  private snapshotKey(): string {
    return JSON.stringify({
      name: this.name,
      gracePeriod: this.gracePeriod,
      isactive: this.isactive,
      esc: this.enablePriorityEscalation,
      transient: this.transient,
      noAlerts: this.disableOverdueAlerts,
      notes: this.notes,
    });
  }

  private resetForm(): void {
    this.name = "";
    this.gracePeriod = "";
    this.isactive = true;
    this.enablePriorityEscalation = false;
    this.transient = false;
    this.disableOverdueAlerts = false;
    this.notes = "";
    this.fieldErrors = {};
  }

  openCreate(): void {
    this.editingId = null;
    this.resetForm();
    this.baselineKey = this.snapshotKey();
    this.formOpen = true;
  }

  openEdit(row: SlaRow): void {
    this.editingId = row.id;
    this.name = row.name;
    this.gracePeriod = String(row.grace_period ?? "");
    this.isactive = row.isactive;
    this.enablePriorityEscalation = row.enable_priority_escalation;
    this.transient = row.transient;
    this.disableOverdueAlerts = row.disable_overdue_alerts;
    this.notes = row.notes ?? "";
    this.fieldErrors = {};
    this.baselineKey = this.snapshotKey();
    this.formOpen = true;
  }

  closeForm(): void {
    this.formOpen = false;
    this.editingId = null;
    this.resetForm();
  }

  setName(v: string): void {
    this.name = v;
    this.clearFieldError("name");
  }
  setGracePeriod(v: string): void {
    this.gracePeriod = v;
    this.clearFieldError("grace_period");
  }
  setIsactive(v: boolean): void {
    this.isactive = v;
  }
  setEnablePriorityEscalation(v: boolean): void {
    this.enablePriorityEscalation = v;
  }
  setTransient(v: boolean): void {
    this.transient = v;
  }
  setDisableOverdueAlerts(v: boolean): void {
    this.disableOverdueAlerts = v;
  }
  setNotes(v: string): void {
    this.notes = v;
  }

  private clearFieldError(key: string): void {
    if (this.fieldErrors[key]) {
      const { [key]: _drop, ...rest } = this.fieldErrors;
      this.fieldErrors = rest;
    }
  }
  fieldError(key: string): string | undefined {
    return this.fieldErrors[key];
  }

  get isEditing(): boolean {
    return this.editingId !== null;
  }
  get formDirty(): boolean {
    return this.snapshotKey() !== this.baselineKey;
  }

  /**
   * The write body. Keys are camelCase to match the backend `SlaWriteRequest`
   * (`#[serde(rename_all="camelCase")]` in backend/api/src/sla.rs) — snake_case
   * keys deserialize as absent and wrongly trip "grace period required". NOTE the
   * asymmetry: the 422 error-envelope field keys are snake_case (`name`,
   * `grace_period`) per US-M4-D1 AC-5, so the inline field-error lookups stay
   * snake_case; only the request body is camelCase.
   */
  private writeBody(): Record<string, unknown> {
    const g = this.gracePeriod.trim();
    const grace = g === "" ? null : Number.isNaN(Number(g)) ? g : Number(g);
    return {
      name: this.name.trim(),
      gracePeriod: grace,
      isactive: this.isactive,
      enablePriorityEscalation: this.enablePriorityEscalation,
      transient: this.transient,
      disableOverdueAlerts: this.disableOverdueAlerts,
      notes: this.notes,
    };
  }

  async save(): Promise<boolean> {
    this.saving = true;
    this.fieldErrors = {};
    try {
      const name = this.name.trim();
      if (this.editingId == null) {
        await this.api.post<{ id: number }>(SLA_PATH, this.writeBody());
        this.snackbar.success(`${name} added successfully`);
      } else {
        await this.api.put<{ id: number }>(`${SLA_PATH}/${this.editingId}`, this.writeBody());
        this.snackbar.success(`${name} updated successfully`);
      }
      runInAction(() => {
        this.formOpen = false;
        this.editingId = null;
      });
      await this.loadList();
      return true;
    } catch (e) {
      if (e instanceof ApiError && e.status === 422) {
        runInAction(() => {
          this.fieldErrors = e.fields ?? {};
        });
      } else {
        this.snackbar.error(e instanceof ApiError ? e.message : "Could not save SLA plan.");
      }
      return false;
    } finally {
      runInAction(() => {
        this.saving = false;
      });
    }
  }

  // ---------------------------------------------------------------------------
  // Mass actions (activate / disable / delete).
  // ---------------------------------------------------------------------------

  async massAction(action: SlaMassAction): Promise<boolean> {
    const ids = this.selectedIds;
    if (ids.length === 0) {
      this.massError = "You must select at least one SLA plan.";
      this.snackbar.error(this.massError);
      return false;
    }
    // Client-side default-SLA delete guard (FS-032.12): refuse before any round trip.
    if (action === "delete" && this.selectionHasDefault) {
      this.massError = DEFAULT_SLA_DELETE_MESSAGE;
      this.snackbar.error(DEFAULT_SLA_DELETE_MESSAGE);
      return false;
    }
    this.massLoading = true;
    this.massError = null;
    try {
      const res = await this.api.post<{ affected: number; message: string }>(`${SLA_PATH}/mass`, {
        action,
        ids,
      });
      if (action === "delete" && res.affected === 0) {
        runInAction(() => {
          this.massError = res.message || "Unable to delete selected SLA plans";
        });
        this.snackbar.error(res.message || "Unable to delete selected SLA plans");
      } else {
        this.snackbar.success(res.message || "Done");
        runInAction(() => {
          this.selected.clear();
        });
      }
      await this.loadList();
      return res.affected > 0;
    } catch (e) {
      const msg = e instanceof ApiError ? e.message : "Mass action failed.";
      runInAction(() => {
        this.massError = msg;
      });
      this.snackbar.error(msg);
      return false;
    } finally {
      runInAction(() => {
        this.massLoading = false;
      });
    }
  }
}
