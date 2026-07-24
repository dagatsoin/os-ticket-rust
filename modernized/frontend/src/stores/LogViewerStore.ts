// System-log viewer store (TS-M4-G3) — FS-033.1/.2/.3/.4/.5/.6/.8. Owns the log
// query state (type + date span + sort + pagination), the results list, the
// selected-detail fetch (content AJAX), the row-selection Set + bulk delete, and
// the manual grace-period "Purge now" trigger.
//
// TDD-covered in LogViewerStore.test.ts: query-param serialization, filter apply,
// sort toggle + paging, detail fetch, selection + bulk delete, purge trigger.
//
// @implements FS-033.2: filter by type + date span.
// @implements FS-033.3: results table (type, title, date).
// @implements FS-033.4: sortable columns.
// @implements FS-033.5: pagination.
// @implements FS-033.6: bulk manual deletion of selected entries.
// @implements FS-033.8: single-record detail (content AJAX).
// @implements BS-033.6: grace-period purge sweep (manual trigger stands in for the M6 cron).
import { makeAutoObservable, runInAction } from "mobx";
import { ApiClient } from "../api/apiClient";
import { ApiError } from "../api/types";
import { SnackbarStore } from "./SnackbarStore";
import type { PaginationMeta } from "./StaffTicketStore";

const LOGS_PATH = "/api/staff/admin/logs";
/**
 * Manual purge trigger. NOTE (contract gap): the pinned backend contract only
 * exposes the dev grace-period sweep here; there is no dedicated admin
 * "purge-now" route yet. The UI button is wired to this endpoint for now — see
 * the report. When a real `POST /api/staff/admin/logs/purge` lands, swap this.
 */
const PURGE_PATH = "/api/dev/purge-logs";

/** A row in the log list. */
export interface LogRow {
  id: number;
  log_type: string;
  title: string;
  created: string | null;
  ip_address: string | null;
}

/** Full log detail (GET /logs/:id) — the list fields plus the body. */
export interface LogDetail extends LogRow {
  log: string;
}

/** Wire shape of GET /api/staff/admin/logs. */
interface LogListPayload {
  items: LogRow[];
  total: number;
  page: number;
  per_page: number;
}

export type LogSortKey = "created" | "log_type" | "title";
export type SortOrder = "ASC" | "DESC";

/** The known log types (FS-033.2 filter options). */
export const LOG_TYPES = ["Error", "Warning", "Debug"] as const;

const PER_PAGE = 25;

export class LogViewerStore {
  // --- query state ---
  filterType = "";
  filterFrom = "";
  filterTo = "";
  sort: LogSortKey = "created";
  order: SortOrder = "DESC";
  page = 1;
  perPage = PER_PAGE;
  total = 0;

  // --- list ---
  rows: LogRow[] = [];
  loadingList = false;
  listError: string | null = null;

  // --- selection ---
  selected = new Set<number>();

  // --- detail ---
  detail: LogDetail | null = null;
  detailOpen = false;
  loadingDetail = false;

  // --- actions ---
  deleting = false;
  purging = false;

  constructor(
    private readonly api: ApiClient,
    private readonly snackbar: SnackbarStore,
  ) {
    makeAutoObservable<LogViewerStore, "api" | "snackbar">(
      this,
      { api: false, snackbar: false },
      { autoBind: true },
    );
  }

  // ---------------------------------------------------------------------------
  // List (type/date filter + server sort + pagination).
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
      if (this.filterType) sp.set("type", this.filterType);
      if (this.filterFrom) sp.set("from", this.filterFrom);
      if (this.filterTo) sp.set("to", this.filterTo);
      const data = await this.api.get<LogListPayload>(`${LOGS_PATH}?${sp.toString()}`);
      runInAction(() => {
        this.rows = data.items ?? [];
        this.total = data.total ?? 0;
        this.page = data.page ?? this.page;
        this.perPage = data.per_page ?? this.perPage;
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

  get pagination(): PaginationMeta {
    return {
      page: this.page,
      pageSize: this.perPage,
      totalCount: this.total,
      totalPages: Math.max(1, Math.ceil(this.total / this.perPage)),
    };
  }

  setFilterType(v: string): void {
    this.filterType = v;
  }
  setFilterFrom(v: string): void {
    this.filterFrom = v;
  }
  setFilterTo(v: string): void {
    this.filterTo = v;
  }

  /** Apply the current filters (FS-033.2) — resets to page 1 and reloads. */
  applyFilters(): Promise<void> {
    this.page = 1;
    return this.loadList();
  }

  setSort(sort: LogSortKey): Promise<void> {
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

  // ---------------------------------------------------------------------------
  // Detail (content AJAX, FS-033.8).
  // ---------------------------------------------------------------------------

  async openDetail(id: number): Promise<void> {
    this.detailOpen = true;
    this.loadingDetail = true;
    this.detail = null;
    try {
      const d = await this.api.get<LogDetail>(`${LOGS_PATH}/${id}`);
      runInAction(() => {
        this.detail = d;
      });
    } catch (e) {
      this.snackbar.error(e instanceof ApiError ? e.message : "Could not load log entry.");
      runInAction(() => {
        this.detailOpen = false;
      });
    } finally {
      runInAction(() => {
        this.loadingDetail = false;
      });
    }
  }

  closeDetail(): void {
    this.detailOpen = false;
    this.detail = null;
  }

  // ---------------------------------------------------------------------------
  // Bulk delete (FS-033.6).
  // ---------------------------------------------------------------------------

  async deleteSelected(): Promise<boolean> {
    const ids = this.selectedIds;
    if (ids.length === 0) return false;
    this.deleting = true;
    try {
      const res = await this.api.post<{ affected: number }>(`${LOGS_PATH}/delete`, { ids });
      this.snackbar.success(`${res.affected} log entr${res.affected === 1 ? "y" : "ies"} deleted`);
      runInAction(() => {
        this.selected.clear();
      });
      await this.loadList();
      return true;
    } catch (e) {
      this.snackbar.error(e instanceof ApiError ? e.message : "Could not delete log entries.");
      return false;
    } finally {
      runInAction(() => {
        this.deleting = false;
      });
    }
  }

  // ---------------------------------------------------------------------------
  // Purge sweep (BS-033.6) — the manual "Purge now" trigger.
  // ---------------------------------------------------------------------------

  async purge(): Promise<boolean> {
    this.purging = true;
    try {
      await this.api.post(PURGE_PATH, {});
      this.snackbar.success("Over-age log entries purged");
      runInAction(() => {
        this.selected.clear();
      });
      await this.loadList();
      return true;
    } catch (e) {
      this.snackbar.error(e instanceof ApiError ? e.message : "Could not purge logs.");
      return false;
    } finally {
      runInAction(() => {
        this.purging = false;
      });
    }
  }
}
