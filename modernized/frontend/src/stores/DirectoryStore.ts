// Staff directory store (TS-M4-B6) — FS-031.9. Read-only browse of the
// directory-visible staff with a server-side `q` search + pagination and a
// department filter.
//
// Department filter note: the directory rows carry `dept_name` but not
// `dept_id`, and there is no non-admin department-list endpoint, so the dept
// filter is applied CLIENT-SIDE by name over the loaded page (options are
// derived from the distinct dept names present). Search + pagination are
// server-side. (Contract gap documented in the TS-M4-B6 report.)
//
// TDD-covered in DirectoryStore.test.ts: search/pagination params, dept-option
// derivation, client-side dept filtering.
//
// @implements FS-031.9: directory browse + search + department filter + paginate.
// @implements BS-031-030: search-term shape is honored by the backend `q`.
import { makeAutoObservable, runInAction } from "mobx";
import { ApiClient } from "../api/apiClient";
import { ApiError } from "../api/types";
import type { PaginationMeta } from "./StaffTicketStore";

const DIRECTORY_PATH = "/api/staff/directory";

/** A directory row. */
export interface DirectoryRow {
  id: number;
  name: string;
  dept_name: string;
  email: string;
  phone: string;
  phone_ext: string;
  mobile: string;
}

interface DirectoryPayload {
  staff: DirectoryRow[];
  pagination: { page: number; per_page: number; total: number };
}

export type DirectorySortKey = "name" | "dept" | "email" | "phone" | "mobile" | "ext";
export type SortOrder = "ASC" | "DESC";

export class DirectoryStore {
  rows: DirectoryRow[] = [];
  pagination: PaginationMeta | null = null;
  loading = false;
  error: string | null = null;

  q = "";
  deptFilter = ""; // dept_name, "" = all (client-side)
  sort: DirectorySortKey = "name";
  order: SortOrder = "ASC";
  page = 1;

  constructor(private readonly api: ApiClient) {
    makeAutoObservable<DirectoryStore, "api">(this, { api: false }, { autoBind: true });
  }

  private buildQuery(): string {
    const sp = new URLSearchParams();
    if (this.q.trim()) sp.set("q", this.q.trim());
    sp.set("sort", this.sort);
    sp.set("order", this.order);
    sp.set("page", String(this.page));
    return sp.toString();
  }

  async loadList(): Promise<void> {
    this.loading = true;
    this.error = null;
    try {
      const data = await this.api.get<DirectoryPayload>(`${DIRECTORY_PATH}?${this.buildQuery()}`);
      runInAction(() => {
        this.rows = data.staff ?? [];
        const pg = data.pagination;
        const pageSize = pg?.per_page || 25;
        const totalCount = pg?.total ?? 0;
        this.pagination = {
          page: pg?.page ?? 1,
          pageSize,
          totalCount,
          totalPages: Math.max(1, Math.ceil(totalCount / pageSize)),
        };
      });
    } catch (e) {
      runInAction(() => {
        this.error = e instanceof ApiError ? e.message : String(e);
      });
    } finally {
      runInAction(() => {
        this.loading = false;
      });
    }
  }

  search(q: string): Promise<void> {
    this.q = q;
    this.page = 1;
    return this.loadList();
  }

  setSort(sort: DirectorySortKey): Promise<void> {
    this.order = this.sort === sort && this.order === "ASC" ? "DESC" : "ASC";
    this.sort = sort;
    this.page = 1;
    return this.loadList();
  }

  setPage(page: number): Promise<void> {
    this.page = page;
    return this.loadList();
  }

  /** Distinct department names present in the loaded rows (for the filter select). */
  get deptOptions(): string[] {
    const set = new Set<string>();
    for (const r of this.rows) if (r.dept_name) set.add(r.dept_name);
    return Array.from(set).sort((a, b) => a.localeCompare(b));
  }

  setDeptFilter(dept: string): void {
    this.deptFilter = dept;
  }

  /** Rows after the client-side department filter. */
  get visibleRows(): DirectoryRow[] {
    if (!this.deptFilter) return this.rows;
    return this.rows.filter((r) => r.dept_name === this.deptFilter);
  }
}
