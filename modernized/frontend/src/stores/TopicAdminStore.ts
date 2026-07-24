// Admin help-topic store (TS-M4-C6) — FS-030.13/.14. Owns the PAGINATED topic
// list, the form-options bundle (priorities / departments / SLA / pages / staff /
// teams / parent_topics), the create/edit form (topic, active/public, required
// dept + priority, optional SLA override, parent [top-level only], thank-you page,
// a single MUTUALLY-EXCLUSIVE auto-assign staff-OR-team control encoded as
// `s<id>`/`t<id>`, auto-response override, notes), inline 422 field errors, and
// the mass-selection Set.
//
// TDD-covered in TopicAdminStore.test.ts: paginated list meta, form-options load,
// assignTo encode/decode + staff↔team mutual exclusion, write body, 422 → inline
// errors, mass actions.
//
// @implements FS-030.13: paginated help-topic list ("Parent / Child") + mass actions.
// @implements FS-030.14: create / edit a topic (dept + priority + SLA + routing).
// @implements BS-030-20: the parent select lists top-level topics only.
// @implements BS-030-22: auto-assign is a single staff-OR-team choice (mutually exclusive).
import { makeAutoObservable, runInAction } from "mobx";
import { ApiClient } from "../api/apiClient";
import { ApiError } from "../api/types";
import { SnackbarStore } from "./SnackbarStore";
import type { IdName } from "./StaffAdminStore";
import type { PaginationMeta } from "./StaffTicketStore";

const TOPICS_PATH = "/api/staff/admin/help-topics";

/** A row in the help-topic list. */
export interface TopicRow {
  topic_id: number;
  topic: string;
  topic_pid: number | null;
  parent_topic: string | null;
  isactive: boolean;
  ispublic: boolean;
  priority_id: number | null;
  priority: string | null;
  dept_id: number | null;
  dept_name: string | null;
  sla_id: number | null;
  staff_id: number | null;
  team_id: number | null;
  page_id: number | null;
  updated: string | null;
}

/** Wire shape of the paginated GET /api/staff/admin/help-topics. */
interface TopicListPayload {
  items: TopicRow[];
  total: number;
  page: number;
  page_size: number;
}

/** Wire shape of GET /api/staff/admin/help-topics/:id. */
interface TopicDetail extends TopicRow {
  noautoresp?: boolean;
  notes?: string;
}

/** The form-options bundle. */
export interface TopicFormOptions {
  priorities: IdName[];
  departments: IdName[];
  sla: IdName[];
  pages: IdName[];
  staff: IdName[];
  teams: IdName[];
  parent_topics: IdName[];
}

export type TopicMassAction = "enable" | "disable" | "delete";

/** Decode an `assignTo` token into a {kind,id} pair (or null when unset). */
export function decodeAssignTo(assignTo: string): { kind: "staff" | "team"; id: number } | null {
  if (!assignTo) return null;
  const kind = assignTo[0];
  const id = Number(assignTo.slice(1));
  if (!Number.isFinite(id) || id <= 0) return null;
  if (kind === "s") return { kind: "staff", id };
  if (kind === "t") return { kind: "team", id };
  return null;
}

/** Encode a staff id into an `assignTo` token. */
export function encodeStaff(id: number): string {
  return `s${id}`;
}
/** Encode a team id into an `assignTo` token. */
export function encodeTeam(id: number): string {
  return `t${id}`;
}

export class TopicAdminStore {
  // --- list ---
  rows: TopicRow[] = [];
  pagination: PaginationMeta | null = null;
  page = 1;
  loadingList = false;
  listError: string | null = null;

  // --- options ---
  options: TopicFormOptions = {
    priorities: [],
    departments: [],
    sla: [],
    pages: [],
    staff: [],
    teams: [],
    parent_topics: [],
  };
  optionsLoaded = false;

  // --- selection ---
  selected = new Set<number>();

  // --- form ---
  formOpen = false;
  editingId: number | null = null;
  loadingDetail = false;
  topic = "";
  isactive = true;
  ispublic = true;
  deptId: number | null = null;
  priorityId: number | null = null;
  slaId: number | null = null;
  parentId: number | null = null;
  pageId: number | null = null;
  /** Encoded auto-assignee: "" | `s<id>` | `t<id>` (mutually exclusive by construction). */
  assignTo = "";
  noautoresp = false;
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
    makeAutoObservable<TopicAdminStore, "api" | "snackbar" | "baselineKey">(
      this,
      { api: false, snackbar: false, baselineKey: false },
      { autoBind: true },
    );
  }

  // ---------------------------------------------------------------------------
  // List (paginated).
  // ---------------------------------------------------------------------------

  async loadList(page: number = this.page): Promise<void> {
    this.page = page;
    this.loadingList = true;
    this.listError = null;
    try {
      const sp = new URLSearchParams({ page: String(page) });
      const data = await this.api.get<TopicListPayload>(`${TOPICS_PATH}?${sp.toString()}`);
      runInAction(() => {
        this.rows = data.items ?? [];
        const pageSize = data.page_size || 25;
        const totalCount = data.total ?? 0;
        this.pagination = {
          page: data.page ?? page,
          pageSize,
          totalCount,
          totalPages: Math.max(1, Math.ceil(totalCount / pageSize)),
        };
        const visible = new Set(this.rows.map((r) => r.topic_id));
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

  setPage(page: number): Promise<void> {
    return this.loadList(page);
  }

  async loadOptions(): Promise<void> {
    try {
      const data = await this.api.get<TopicFormOptions>(`${TOPICS_PATH}/form-options`);
      runInAction(() => {
        this.options = {
          priorities: data.priorities ?? [],
          departments: data.departments ?? [],
          sla: data.sla ?? [],
          pages: data.pages ?? [],
          staff: data.staff ?? [],
          teams: data.teams ?? [],
          parent_topics: data.parent_topics ?? [],
        };
        this.optionsLoaded = true;
      });
    } catch (e) {
      this.snackbar.error(e instanceof ApiError ? e.message : "Could not load topic options.");
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
    return this.rows.length > 0 && this.rows.every((r) => this.selected.has(r.topic_id));
  }
  get someSelected(): boolean {
    return this.selected.size > 0 && !this.allSelected;
  }
  toggleSelectAll(): void {
    if (this.allSelected) this.selected.clear();
    else for (const r of this.rows) this.selected.add(r.topic_id);
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
      topic: this.topic,
      isactive: this.isactive,
      ispublic: this.ispublic,
      deptId: this.deptId,
      priorityId: this.priorityId,
      slaId: this.slaId,
      parentId: this.parentId,
      pageId: this.pageId,
      assignTo: this.assignTo,
      noautoresp: this.noautoresp,
      notes: this.notes,
    });
  }

  private resetForm(): void {
    this.topic = "";
    this.isactive = true;
    this.ispublic = true;
    this.deptId = null;
    this.priorityId = null;
    this.slaId = null;
    this.parentId = null;
    this.pageId = null;
    this.assignTo = "";
    this.noautoresp = false;
    this.notes = "";
    this.fieldErrors = {};
  }

  openCreate(): void {
    this.editingId = null;
    this.resetForm();
    this.baselineKey = this.snapshotKey();
    this.formOpen = true;
    if (!this.optionsLoaded) void this.loadOptions();
  }

  async openEdit(id: number): Promise<void> {
    this.editingId = id;
    this.resetForm();
    this.formOpen = true;
    this.loadingDetail = true;
    if (!this.optionsLoaded) await this.loadOptions();
    try {
      const d = await this.api.get<TopicDetail>(`${TOPICS_PATH}/${id}`);
      runInAction(() => {
        this.topic = d.topic;
        this.isactive = d.isactive;
        this.ispublic = d.ispublic;
        this.deptId = d.dept_id ?? null;
        this.priorityId = d.priority_id ?? null;
        this.slaId = d.sla_id ?? null;
        this.parentId = d.topic_pid ?? null;
        this.pageId = d.page_id ?? null;
        // Decode routing: staff wins if both are (unexpectedly) present.
        this.assignTo = d.staff_id ? encodeStaff(d.staff_id) : d.team_id ? encodeTeam(d.team_id) : "";
        this.noautoresp = d.noautoresp ?? false;
        this.notes = d.notes ?? "";
        this.baselineKey = this.snapshotKey();
      });
    } catch (e) {
      this.snackbar.error(e instanceof ApiError ? e.message : "Could not load help topic.");
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

  setTopic(v: string): void {
    this.topic = v;
    this.clearFieldError("topic");
  }
  setIsactive(v: boolean): void {
    this.isactive = v;
  }
  setIspublic(v: boolean): void {
    this.ispublic = v;
  }
  setDeptId(v: number | null): void {
    this.deptId = v;
    this.clearFieldError("deptId");
  }
  setPriorityId(v: number | null): void {
    this.priorityId = v;
    this.clearFieldError("priorityId");
  }
  setSlaId(v: number | null): void {
    this.slaId = v;
  }
  setParentId(v: number | null): void {
    this.parentId = v;
  }
  setPageId(v: number | null): void {
    this.pageId = v;
  }
  setNoautoresp(v: boolean): void {
    this.noautoresp = v;
  }
  setNotes(v: string): void {
    this.notes = v;
  }

  // --- auto-assign (mutually exclusive staff-OR-team) ---
  /** Assign to a staff member; clears any team selection (BS-030-22). */
  setAssignStaff(id: number): void {
    this.assignTo = encodeStaff(id);
  }
  /** Assign to a team; clears any staff selection (BS-030-22). */
  setAssignTeam(id: number): void {
    this.assignTo = encodeTeam(id);
  }
  clearAssign(): void {
    this.assignTo = "";
  }
  get assignStaffId(): number | null {
    const d = decodeAssignTo(this.assignTo);
    return d?.kind === "staff" ? d.id : null;
  }
  get assignTeamId(): number | null {
    const d = decodeAssignTo(this.assignTo);
    return d?.kind === "team" ? d.id : null;
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

  /** Parent candidates: top-level topics only (BS-030-20), never self. */
  get parentCandidates(): IdName[] {
    return this.options.parent_topics.filter((t) => t.id !== this.editingId);
  }

  private validate(): boolean {
    const errors: Record<string, string> = {};
    if (!this.deptId) errors.deptId = "Department selection required";
    if (!this.priorityId) errors.priorityId = "Priority selection required";
    this.fieldErrors = errors;
    return Object.keys(errors).length === 0;
  }

  private writeBody(): Record<string, unknown> {
    return {
      topic: this.topic.trim(),
      isactive: this.isactive,
      ispublic: this.ispublic,
      deptId: this.deptId,
      priorityId: this.priorityId,
      slaId: this.slaId,
      parentId: this.parentId,
      assignTo: this.assignTo || null,
      pageId: this.pageId,
      noautoresp: this.noautoresp,
      notes: this.notes,
    };
  }

  async save(): Promise<boolean> {
    if (!this.validate()) return false;
    this.saving = true;
    try {
      const topic = this.topic.trim();
      if (this.editingId == null) {
        await this.api.post<{ id: number }>(TOPICS_PATH, this.writeBody());
        this.snackbar.success(`${topic} added successfully`);
      } else {
        await this.api.put<{ id: number }>(`${TOPICS_PATH}/${this.editingId}`, this.writeBody());
        this.snackbar.success(`${topic} updated successfully`);
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
        this.snackbar.error(e instanceof ApiError ? e.message : "Could not save help topic.");
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
      await this.api.delete(`${TOPICS_PATH}/${id}`);
      this.snackbar.success("Help topic deleted");
      await this.loadList();
      return true;
    } catch (e) {
      this.snackbar.error(e instanceof ApiError ? e.message : "Could not delete help topic.");
      return false;
    }
  }

  // ---------------------------------------------------------------------------
  // Mass actions.
  // ---------------------------------------------------------------------------

  async massAction(action: TopicMassAction): Promise<boolean> {
    const ids = this.selectedIds;
    if (ids.length === 0) {
      this.massError = "You must select at least one help topic.";
      this.snackbar.error(this.massError);
      return false;
    }
    this.massLoading = true;
    this.massError = null;
    try {
      const res = await this.api.post<{ affected: number; message: string }>(`${TOPICS_PATH}/mass`, {
        action,
        ids,
      });
      this.snackbar.success(res.message || "Done");
      runInAction(() => {
        this.selected.clear();
      });
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
