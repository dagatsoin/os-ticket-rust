// Read-only priority reference store (TS-M4-D3) — FS-032.8 / BS-032.13. The
// priority set is fixed and urgency-ranked (KL-032.1 preserved: NO CRUD). This
// store is a thin fetch of the ordered list; the backend already returns it
// `ORDER BY urgency DESC` (Low, Normal, High, Emergency).
//
// @implements FS-032.8: read-only ticket-priority reference set.
// @implements BS-032.13: priorities are a fixed, urgency-ranked, non-editable set.
// @implements KL-032.1: no admin CRUD for priorities (read-only panel).
import { makeAutoObservable, runInAction } from "mobx";
import { ApiClient } from "../api/apiClient";
import { ApiError } from "../api/types";

const PRIORITIES_PATH = "/api/staff/admin/priorities";

/** A fixed priority (GET /api/staff/admin/priorities, already urgency-ordered). */
export interface PriorityRow {
  priority_id: number;
  priority: string;
  priority_desc: string;
  priority_color: string;
  urgency: number;
  ispublic: boolean;
}

export class PriorityStore {
  rows: PriorityRow[] = [];
  loading = false;
  error: string | null = null;

  constructor(private readonly api: ApiClient) {
    makeAutoObservable<PriorityStore, "api">(this, { api: false }, { autoBind: true });
  }

  async loadList(): Promise<void> {
    this.loading = true;
    this.error = null;
    try {
      const data = await this.api.get<PriorityRow[]>(PRIORITIES_PATH);
      runInAction(() => {
        this.rows = data ?? [];
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
}
