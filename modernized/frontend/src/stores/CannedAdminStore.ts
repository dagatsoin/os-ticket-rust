// Admin canned-response store (TS-M4-H2) — FS-022. Owns the canned-response
// list, the dept-scope options (served under the premade gate, NOT the admin
// gate), the create/edit form (title, optional dept scope, %{token} body,
// enabled flag, notes, attachments), inline 422 (duplicate title), and the
// mass-selection Set. Attachments are submitted as multipart, reusing the M2
// AttachmentChip + validateAttachment; edits send keep_file_ids[] for retained
// files plus any newly-staged File parts.
//
// TDD-covered in CannedAdminStore.test.ts: list, dept-options, form dirty,
// attachment staging + validation, multipart body (title/dept/response/enabled/
// notes/files/keep_file_ids), 422 → inline title error, mass actions.
//
// @implements FS-022: canned-response admin CRUD surface (deferred out of M2).
// @implements BS-031-020: reachable via the `can_manage_premade` capability gate.
import { makeAutoObservable, runInAction } from "mobx";
import { ApiClient } from "../api/apiClient";
import { ApiError } from "../api/types";
import { SnackbarStore } from "./SnackbarStore";
import { validateAttachment } from "../utils/validateAttachment";

const CANNED_PATH = "/api/staff/canned-responses";
const DEPT_OPTIONS_PATH = "/api/staff/canned-responses/dept-options";

/** A row in the canned-response list. */
export interface CannedRow {
  id: number;
  title: string;
  dept_id: number | null;
  dept_name: string | null;
  isenabled: boolean;
  notes: string;
  attachment_count: number;
  updated: string | null;
}

/** A persisted attachment carried by a canned response. */
export interface CannedAttachment {
  id: number;
  name: string;
  size: number;
  mime: string;
}

/** Wire shape of GET /api/staff/canned-responses/:id. */
interface CannedDetail extends CannedRow {
  response: string;
  attachments: CannedAttachment[];
}

/** {id,name} department option (served under the premade gate). */
export interface DeptOption {
  id: number;
  name: string;
}

export type CannedMassAction = "enable" | "disable" | "delete";

export class CannedAdminStore {
  // --- list ---
  rows: CannedRow[] = [];
  loadingList = false;
  listError: string | null = null;

  // --- dept options ---
  deptOptions: DeptOption[] = [];
  optionsLoaded = false;

  // --- selection ---
  selected = new Set<number>();

  // --- form ---
  formOpen = false;
  editingId: number | null = null;
  loadingDetail = false;
  title = "";
  deptId: number | null = null;
  response = "";
  isenabled = true;
  notes = "";
  /** Newly-selected local files to upload (multipart). */
  stagedFiles: File[] = [];
  /** Persisted attachments (edit): those still in this set are kept (keep_file_ids). */
  existingAttachments: CannedAttachment[] = [];
  keptIds = new Set<number>();
  fileError: string | undefined = undefined;
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
    makeAutoObservable<CannedAdminStore, "api" | "snackbar" | "baselineKey">(
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
      const data = await this.api.get<CannedRow[]>(CANNED_PATH);
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

  async loadOptions(): Promise<void> {
    try {
      const data = await this.api.get<DeptOption[]>(DEPT_OPTIONS_PATH);
      runInAction(() => {
        this.deptOptions = data ?? [];
        this.optionsLoaded = true;
      });
    } catch (e) {
      this.snackbar.error(e instanceof ApiError ? e.message : "Could not load departments.");
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
      title: this.title,
      deptId: this.deptId,
      response: this.response,
      isenabled: this.isenabled,
      notes: this.notes,
      staged: this.stagedFiles.map((f) => f.name),
      kept: Array.from(this.keptIds).sort((a, b) => a - b),
    });
  }

  private resetForm(): void {
    this.title = "";
    this.deptId = null;
    this.response = "";
    this.isenabled = true;
    this.notes = "";
    this.stagedFiles = [];
    this.existingAttachments = [];
    this.keptIds = new Set<number>();
    this.fileError = undefined;
    this.fieldErrors = {};
  }

  openCreate(): void {
    this.editingId = null;
    this.resetForm();
    this.baselineKey = this.snapshotKey();
    this.formOpen = true;
    void this.loadOptions();
  }

  async openEdit(id: number): Promise<void> {
    this.editingId = id;
    this.resetForm();
    this.formOpen = true;
    this.loadingDetail = true;
    await this.loadOptions();
    try {
      const d = await this.api.get<CannedDetail>(`${CANNED_PATH}/${id}`);
      runInAction(() => {
        this.title = d.title;
        this.deptId = d.dept_id ?? null;
        this.response = d.response ?? "";
        this.isenabled = d.isenabled;
        this.notes = d.notes ?? "";
        this.existingAttachments = d.attachments ?? [];
        this.keptIds = new Set(this.existingAttachments.map((a) => a.id));
        this.baselineKey = this.snapshotKey();
      });
    } catch (e) {
      this.snackbar.error(e instanceof ApiError ? e.message : "Could not load canned response.");
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

  setTitle(v: string): void {
    this.title = v;
    this.clearFieldError("title");
  }
  setDeptId(v: number | null): void {
    this.deptId = v;
  }
  setResponse(v: string): void {
    this.response = v;
    this.clearFieldError("response");
  }
  setIsenabled(v: boolean): void {
    this.isenabled = v;
  }
  setNotes(v: string): void {
    this.notes = v;
  }

  /** Stage a locally-selected file after the shared client-side pre-check. */
  addFile(file: File): void {
    const err = validateAttachment(file);
    if (err) {
      this.fileError = err;
      return;
    }
    this.fileError = undefined;
    this.stagedFiles = [...this.stagedFiles, file];
  }
  removeStagedFile(index: number): void {
    this.stagedFiles = this.stagedFiles.filter((_, i) => i !== index);
  }
  /** Drop a persisted attachment (excluded from keep_file_ids on save). */
  removeExisting(id: number): void {
    this.keptIds.delete(id);
  }
  isKept(id: number): boolean {
    return this.keptIds.has(id);
  }
  get keptAttachments(): CannedAttachment[] {
    return this.existingAttachments.filter((a) => this.keptIds.has(a.id));
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

  /** Build the multipart body: text fields + new files + keep_file_ids on edit. */
  private buildForm(): FormData {
    const form = new FormData();
    form.append("title", this.title.trim());
    if (this.deptId != null) form.append("dept_id", String(this.deptId));
    form.append("response", this.response);
    form.append("isenabled", this.isenabled ? "1" : "0");
    form.append("notes", this.notes);
    // 2-arg append: the File carries its own name (3-arg hangs undici under jsdom).
    for (const f of this.stagedFiles) form.append("files", f);
    if (this.editingId != null) {
      for (const id of this.keptIds) form.append("keep_file_ids[]", String(id));
    }
    return form;
  }

  async save(): Promise<boolean> {
    if (this.fileError) return false;
    this.saving = true;
    this.fieldErrors = {};
    try {
      const title = this.title.trim();
      if (this.editingId == null) {
        await this.api.post<{ id: number }>(CANNED_PATH, this.buildForm());
        this.snackbar.success(`${title} added successfully`);
      } else {
        await this.api.put<{ id: number }>(`${CANNED_PATH}/${this.editingId}`, this.buildForm());
        this.snackbar.success(`${title} updated successfully`);
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
        this.snackbar.error(e instanceof ApiError ? e.message : "Could not save canned response.");
      }
      return false;
    } finally {
      runInAction(() => {
        this.saving = false;
      });
    }
  }

  // ---------------------------------------------------------------------------
  // Mass actions (enable / disable / delete).
  // ---------------------------------------------------------------------------

  async massAction(action: CannedMassAction): Promise<boolean> {
    const ids = this.selectedIds;
    if (ids.length === 0) {
      this.massError = "You must select at least one canned response.";
      this.snackbar.error(this.massError);
      return false;
    }
    this.massLoading = true;
    this.massError = null;
    try {
      const res = await this.api.post<{ affected: number; message: string }>(
        `${CANNED_PATH}/mass`,
        { action, ids },
      );
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
