// Admin site-pages store (TS-M4-F2) — FS-033.11/.12/.13/.14/.15/.16. Owns the
// server-sorted + server-paginated page list, the create/edit form (name, type,
// body, active, notes), inline 422 (duplicate-name / disable-guard), the
// mass-selection Set, and the in-use delete/disable protection (BS-033.9).
//
// TDD-covered in PageAdminStore.test.ts: list param serialization + pagination,
// sort toggle, form dirty, 422 → inline, mass actions, in-use guard predicate.
//
// @implements FS-033.11: site-pages list + access gate.
// @implements FS-033.12: results table — sorting + pagination.
// @implements FS-033.13: create / edit a page.
// @implements FS-033.14: type ∈ {landing,offline,thank-you,other}; name required.
// @implements FS-033.15: enable / disable / delete bulk actions.
// @implements BS-033.9: an in-use page — delete AND disable are refused.
// @implements BS-033.10: page name is unique (duplicate → inline error).
import { makeAutoObservable, runInAction } from "mobx";
import { ApiClient } from "../api/apiClient";
import { ApiError } from "../api/types";
import { SnackbarStore } from "./SnackbarStore";
import type { PaginationMeta } from "./StaffTicketStore";

const PAGES_PATH = "/api/staff/admin/pages";

/** The four page types (FS-033.14). */
export const PAGE_TYPES = ["landing", "offline", "thank-you", "other"] as const;
export type PageType = (typeof PAGE_TYPES)[number];

/** A row in the page list. */
export interface PageRow {
  id: number;
  name: string;
  type: PageType;
  isactive: boolean;
  in_use: boolean;
  created: string | null;
  updated: string | null;
}

/** Wire shape of GET /api/staff/admin/pages. */
interface PageListPayload {
  items: PageRow[];
  pagination: { page: number; per_page: number; total: number };
}

/** Wire shape of GET /api/staff/admin/pages/:id (detail incl. body). */
interface PageDetail extends PageRow {
  body: string;
  notes: string;
}

export type PageSortKey = "name" | "type" | "status" | "created";
export type SortOrder = "ASC" | "DESC";
export type PageMassAction = "enable" | "disable" | "delete";

const PER_PAGE = 25;

export class PageAdminStore {
  // --- list ---
  rows: PageRow[] = [];
  loadingList = false;
  listError: string | null = null;
  sort: PageSortKey = "name";
  order: SortOrder = "ASC";
  page = 1;
  perPage = PER_PAGE;
  total = 0;

  // --- selection ---
  selected = new Set<number>();

  // --- form ---
  formOpen = false;
  editingId: number | null = null;
  loadingDetail = false;
  name = "";
  type: PageType = "landing";
  body = "";
  isactive = true;
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
    makeAutoObservable<PageAdminStore, "api" | "snackbar" | "baselineKey">(
      this,
      { api: false, snackbar: false, baselineKey: false },
      { autoBind: true },
    );
  }

  // ---------------------------------------------------------------------------
  // List (server sort + pagination).
  // ---------------------------------------------------------------------------

  async loadList(): Promise<void> {
    this.loadingList = true;
    this.listError = null;
    try {
      const sp = new URLSearchParams({
        sort: this.sort,
        order: this.order,
        page: String(this.page),
        per_page: String(this.perPage),
      });
      const data = await this.api.get<PageListPayload>(`${PAGES_PATH}?${sp.toString()}`);
      runInAction(() => {
        this.rows = data.items ?? [];
        this.page = data.pagination?.page ?? this.page;
        this.perPage = data.pagination?.per_page ?? this.perPage;
        this.total = data.pagination?.total ?? 0;
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

  /** Pagination metadata for the reusable <Pagination/> control. */
  get pagination(): PaginationMeta {
    return {
      page: this.page,
      pageSize: this.perPage,
      totalCount: this.total,
      totalPages: Math.max(1, Math.ceil(this.total / this.perPage)),
    };
  }

  setSort(sort: PageSortKey): Promise<void> {
    this.order = this.sort === sort && this.order === "ASC" ? "DESC" : "ASC";
    this.sort = sort;
    this.page = 1;
    return this.loadList();
  }

  setPage(page: number): Promise<void> {
    this.page = page;
    return this.loadList();
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

  /** True when the current selection includes an in-use (protected) page. */
  get selectionHasInUse(): boolean {
    return this.rows.some((r) => r.in_use && this.selected.has(r.id));
  }

  // ---------------------------------------------------------------------------
  // Form.
  // ---------------------------------------------------------------------------

  private snapshotKey(): string {
    return JSON.stringify({
      name: this.name,
      type: this.type,
      body: this.body,
      isactive: this.isactive,
      notes: this.notes,
    });
  }

  private resetForm(): void {
    this.name = "";
    this.type = "landing";
    this.body = "";
    this.isactive = true;
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
      const d = await this.api.get<PageDetail>(`${PAGES_PATH}/${id}`);
      runInAction(() => {
        this.name = d.name;
        this.type = d.type;
        this.body = d.body ?? "";
        this.isactive = d.isactive;
        this.notes = d.notes ?? "";
        this.baselineKey = this.snapshotKey();
      });
    } catch (e) {
      this.snackbar.error(e instanceof ApiError ? e.message : "Could not load page.");
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
  setType(v: PageType): void {
    this.type = v;
  }
  setBody(v: string): void {
    this.body = v;
    this.clearFieldError("body");
  }
  setIsactive(v: boolean): void {
    this.isactive = v;
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
      type: this.type,
      body: this.body,
      isactive: this.isactive,
      notes: this.notes,
    };
  }

  async save(): Promise<boolean> {
    this.saving = true;
    this.fieldErrors = {};
    try {
      const name = this.name.trim();
      if (this.editingId == null) {
        await this.api.post<{ id: number }>(PAGES_PATH, this.writeBody());
        this.snackbar.success(`${name} added successfully`);
      } else {
        await this.api.put<{ id: number }>(`${PAGES_PATH}/${this.editingId}`, this.writeBody());
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
        this.snackbar.error(e instanceof ApiError ? e.message : "Could not save page.");
      }
      return false;
    } finally {
      runInAction(() => {
        this.saving = false;
      });
    }
  }

  // ---------------------------------------------------------------------------
  // Mass actions (enable / disable / delete — in-use rows are guarded).
  // ---------------------------------------------------------------------------

  async massAction(action: PageMassAction): Promise<boolean> {
    const ids = this.selectedIds;
    if (ids.length === 0) {
      this.massError = "You must select at least one page.";
      this.snackbar.error(this.massError);
      return false;
    }
    // Client-side in-use guard (BS-033.9): disable AND delete are refused for
    // bound pages before any round trip; the backend re-checks (422) as well.
    if ((action === "delete" || action === "disable") && this.selectionHasInUse) {
      const msg = `A page in use cannot be ${action === "delete" ? "deleted" : "disabled"}.`;
      this.massError = msg;
      this.snackbar.error(msg);
      return false;
    }
    this.massLoading = true;
    this.massError = null;
    try {
      const res = await this.api.post<{ affected: number; message: string }>(`${PAGES_PATH}/mass`, {
        action,
        ids,
      });
      if ((action === "delete" || action === "disable") && res.affected === 0) {
        runInAction(() => {
          this.massError = res.message || `Unable to ${action} selected pages`;
        });
        this.snackbar.error(res.message || `Unable to ${action} selected pages`);
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
