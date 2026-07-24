// FAQ-category store (TS-M4-E2) — FS-032.13/.14. Owns the FAQ-category list, the
// create/edit form (name, public/internal, description, notes), inline 422 field
// errors, single delete, and the mass-selection Set. Reachable via the
// `can_manage_faq` capability gate — its base path is /api/staff/faq-categories
// (NOT under /admin) so a delegated non-admin FAQ manager may call it.
//
// TDD-covered in FaqCategoryStore.test.ts: list load, create/edit body
// ({name,ispublic,description,notes}), dirty tracking, 422 → inline errors,
// delete, mass actions (delete / makepublic / makeprivate).
//
// @implements FS-032.13: FAQ-category list (public/internal, description) + mass actions.
// @implements FS-032.14: create / edit a FAQ category.
import { makeAutoObservable, runInAction } from "mobx";
import { ApiClient } from "../api/apiClient";
import { ApiError } from "../api/types";
import { SnackbarStore } from "./SnackbarStore";

const FAQ_PATH = "/api/staff/faq-categories";

/** A row in the FAQ-category list. */
export interface FaqCategoryRow {
  id: number;
  name: string;
  ispublic: boolean;
  description: string;
  notes: string;
  updated: string | null;
}

export type FaqMassAction = "delete" | "makepublic" | "makeprivate";

export class FaqCategoryStore {
  // --- list ---
  rows: FaqCategoryRow[] = [];
  loadingList = false;
  listError: string | null = null;

  // --- selection ---
  selected = new Set<number>();

  // --- form ---
  formOpen = false;
  editingId: number | null = null;
  loadingDetail = false;
  name = "";
  ispublic = true;
  description = "";
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
    makeAutoObservable<FaqCategoryStore, "api" | "snackbar" | "baselineKey">(
      this,
      { api: false, snackbar: false, baselineKey: false },
      { autoBind: true },
    );
  }

  // ---------------------------------------------------------------------------
  // List.
  // ---------------------------------------------------------------------------

  async loadList(): Promise<void> {
    this.loadingList = true;
    this.listError = null;
    try {
      const data = await this.api.get<FaqCategoryRow[]>(FAQ_PATH);
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

  // ---------------------------------------------------------------------------
  // Selection.
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

  // ---------------------------------------------------------------------------
  // Form.
  // ---------------------------------------------------------------------------

  private snapshotKey(): string {
    return JSON.stringify({
      name: this.name,
      ispublic: this.ispublic,
      description: this.description,
      notes: this.notes,
    });
  }

  private resetForm(): void {
    this.name = "";
    this.ispublic = true;
    this.description = "";
    this.notes = "";
    this.fieldErrors = {};
  }

  openCreate(): void {
    this.editingId = null;
    this.resetForm();
    this.baselineKey = this.snapshotKey();
    this.formOpen = true;
  }

  async openEdit(id: number): Promise<void> {
    this.editingId = id;
    this.resetForm();
    this.formOpen = true;
    this.loadingDetail = true;
    try {
      const d = await this.api.get<FaqCategoryRow>(`${FAQ_PATH}/${id}`);
      runInAction(() => {
        this.name = d.name;
        this.ispublic = d.ispublic;
        this.description = d.description ?? "";
        this.notes = d.notes ?? "";
        this.baselineKey = this.snapshotKey();
      });
    } catch (e) {
      this.snackbar.error(e instanceof ApiError ? e.message : "Could not load FAQ category.");
    } finally {
      runInAction(() => {
        this.loadingDetail = false;
      });
    }
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
  setIspublic(v: boolean): void {
    this.ispublic = v;
  }
  setDescription(v: string): void {
    this.description = v;
    this.clearFieldError("description");
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

  private writeBody(): Record<string, unknown> {
    return {
      name: this.name.trim(),
      ispublic: this.ispublic,
      description: this.description,
      notes: this.notes,
    };
  }

  async save(): Promise<boolean> {
    this.saving = true;
    this.fieldErrors = {};
    try {
      const name = this.name.trim();
      if (this.editingId == null) {
        await this.api.post<{ id: number }>(FAQ_PATH, this.writeBody());
        this.snackbar.success(`${name} added successfully`);
      } else {
        await this.api.put<{ id: number }>(`${FAQ_PATH}/${this.editingId}`, this.writeBody());
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
        this.snackbar.error(e instanceof ApiError ? e.message : "Could not save FAQ category.");
      }
      return false;
    } finally {
      runInAction(() => {
        this.saving = false;
      });
    }
  }

  async deleteOne(id: number): Promise<boolean> {
    try {
      await this.api.delete(`${FAQ_PATH}/${id}`);
      this.snackbar.success("FAQ category deleted");
      await this.loadList();
      return true;
    } catch (e) {
      this.snackbar.error(e instanceof ApiError ? e.message : "Could not delete FAQ category.");
      return false;
    }
  }

  // ---------------------------------------------------------------------------
  // Mass actions.
  // ---------------------------------------------------------------------------

  async massAction(action: FaqMassAction): Promise<boolean> {
    const ids = this.selectedIds;
    if (ids.length === 0) {
      this.massError = "You must select at least one category.";
      this.snackbar.error(this.massError);
      return false;
    }
    this.massLoading = true;
    this.massError = null;
    try {
      const res = await this.api.post<{ affected: number; message: string }>(`${FAQ_PATH}/mass`, {
        action,
        ids,
      });
      if (action === "delete" && res.affected === 0) {
        runInAction(() => {
          this.massError = res.message || "Unable to delete selected categories";
        });
        this.snackbar.error(res.message || "Unable to delete selected categories");
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
