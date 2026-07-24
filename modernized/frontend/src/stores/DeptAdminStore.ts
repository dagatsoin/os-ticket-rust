// Admin department store (TS-M4-C2) — FS-030.3/.4. Owns the department list, the
// form-options bundle (email accounts / template groups / SLA / staff / groups),
// the create/edit form (name, public/private, required Email + Template selects,
// optional SLA + Manager, auto-response toggles, dept signature, and the
// group-access matrix as a full-replace Set), inline 422 field errors + a
// client-side pre-check (required Email/Template, default-private block), single
// delete (home-staff block surfaced), and the mass-selection Set.
//
// TDD-covered in DeptAdminStore.test.ts: list load + default-row protection,
// form-options load, create/edit camelCase body ({emailId,tplId,slaId,managerId,
// groupIds,…}), group-matrix Set + select-all/none, required-field pre-check,
// default-private inline block, 422 → inline errors, delete-block message,
// mass actions.
//
// @implements FS-030.3: department list (public flag, manager, user count) + mass actions.
// @implements FS-030.4: create / edit a department (Email + Template + SLA + Manager + groups).
// @implements BS-030-02: Email selection is required.
// @implements BS-030-03: Template selection is required.
// @implements BS-030-04: the system default department cannot be made private.
// @implements BS-030-05: the default department row is protected from selection / delete.
import { makeAutoObservable, runInAction } from "mobx";
import { ApiClient } from "../api/apiClient";
import { ApiError } from "../api/types";
import { SnackbarStore } from "./SnackbarStore";
import type { IdName } from "./StaffAdminStore";

const DEPTS_PATH = "/api/staff/admin/departments";

/** A row in the department list. */
export interface DeptRow {
  id: number;
  name: string;
  ispublic: boolean;
  is_default: boolean;
  manager_id: number | null;
  manager_name: string | null;
  email_id: number | null;
  tpl_id: number | null;
  sla_id: number | null;
  sla_name: string | null;
  user_count: number;
  updated: string | null;
}

/** Wire shape of GET /api/staff/admin/departments/:id (row + group_ids). */
interface DeptDetail extends DeptRow {
  group_ids: number[];
  ticket_auto_response?: boolean;
  message_auto_response?: boolean;
  autoresp_email_id?: number | null;
  dept_signature?: string;
  group_membership?: number;
}

/** An email-account option ({id,email,name}). */
export interface EmailOption {
  id: number;
  email: string;
  name: string;
}

/** The form-options bundle served by GET .../departments/form-options. */
export interface DeptFormOptions {
  email_accounts: EmailOption[];
  template_groups: IdName[];
  sla: IdName[];
  staff: IdName[];
  groups: IdName[];
}

export type DeptMassAction = "delete" | "makepublic" | "makeprivate";

export class DeptAdminStore {
  // --- list ---
  rows: DeptRow[] = [];
  loadingList = false;
  listError: string | null = null;

  // --- options ---
  options: DeptFormOptions = {
    email_accounts: [],
    template_groups: [],
    sla: [],
    staff: [],
    groups: [],
  };
  optionsLoaded = false;

  // --- selection ---
  selected = new Set<number>();

  // --- form ---
  formOpen = false;
  editingId: number | null = null;
  editingDefault = false;
  loadingDetail = false;
  name = "";
  ispublic = true;
  emailId: number | null = null;
  tplId: number | null = null;
  slaId: number | null = null;
  managerId: number | null = null;
  ticketAutoResponse = true;
  messageAutoResponse = true;
  autorespEmailId: number | null = null;
  deptSignature = "";
  // Numeric membership-extension flag (backend DeptWriteRequest.group_membership
  // is Option<i32>). No dedicated form control — defaults to 0 (disabled). Sent as
  // a number so serde never sees a boolean where it expects i32.
  groupMembership = 0;
  groupIds = new Set<number>();
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
    makeAutoObservable<DeptAdminStore, "api" | "snackbar" | "baselineKey">(
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
      const data = await this.api.get<DeptRow[]>(DEPTS_PATH);
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
      const data = await this.api.get<DeptFormOptions>(`${DEPTS_PATH}/form-options`);
      runInAction(() => {
        this.options = {
          email_accounts: data.email_accounts ?? [],
          template_groups: data.template_groups ?? [],
          sla: data.sla ?? [],
          staff: data.staff ?? [],
          groups: data.groups ?? [],
        };
        this.optionsLoaded = true;
      });
    } catch (e) {
      this.snackbar.error(e instanceof ApiError ? e.message : "Could not load department options.");
    }
  }

  // ---------------------------------------------------------------------------
  // Selection. The default department row is protected (BS-030-05).
  // ---------------------------------------------------------------------------

  isSelectable(row: DeptRow): boolean {
    return !row.is_default;
  }
  toggleSelect(id: number): void {
    const row = this.rows.find((r) => r.id === id);
    if (row && !this.isSelectable(row)) return;
    if (this.selected.has(id)) this.selected.delete(id);
    else this.selected.add(id);
  }
  private get selectableRows(): DeptRow[] {
    return this.rows.filter((r) => this.isSelectable(r));
  }
  get allSelected(): boolean {
    const rows = this.selectableRows;
    return rows.length > 0 && rows.every((r) => this.selected.has(r.id));
  }
  get someSelected(): boolean {
    return this.selected.size > 0 && !this.allSelected;
  }
  toggleSelectAll(): void {
    if (this.allSelected) this.selected.clear();
    else for (const r of this.selectableRows) this.selected.add(r.id);
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
      emailId: this.emailId,
      tplId: this.tplId,
      slaId: this.slaId,
      managerId: this.managerId,
      ticketAutoResponse: this.ticketAutoResponse,
      messageAutoResponse: this.messageAutoResponse,
      autorespEmailId: this.autorespEmailId,
      deptSignature: this.deptSignature,
      groupMembership: this.groupMembership,
      groupIds: Array.from(this.groupIds).sort((a, b) => a - b),
    });
  }

  private resetForm(): void {
    this.name = "";
    this.ispublic = true;
    this.emailId = null;
    this.tplId = null;
    this.slaId = null;
    this.managerId = null;
    this.ticketAutoResponse = true;
    this.messageAutoResponse = true;
    this.autorespEmailId = null;
    this.deptSignature = "";
    this.groupMembership = 0;
    this.groupIds = new Set<number>();
    this.fieldErrors = {};
    this.editingDefault = false;
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
      const d = await this.api.get<DeptDetail>(`${DEPTS_PATH}/${id}`);
      runInAction(() => {
        this.name = d.name;
        this.ispublic = d.ispublic;
        this.emailId = d.email_id ?? null;
        this.tplId = d.tpl_id ?? null;
        this.slaId = d.sla_id ?? null;
        this.managerId = d.manager_id ?? null;
        this.ticketAutoResponse = d.ticket_auto_response ?? true;
        this.messageAutoResponse = d.message_auto_response ?? true;
        this.autorespEmailId = d.autoresp_email_id ?? null;
        this.deptSignature = d.dept_signature ?? "";
        this.groupMembership = d.group_membership ?? 0;
        this.groupIds = new Set(d.group_ids ?? []);
        this.editingDefault = Boolean(d.is_default);
        this.baselineKey = this.snapshotKey();
      });
    } catch (e) {
      this.snackbar.error(e instanceof ApiError ? e.message : "Could not load department.");
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
    this.clearFieldError("ispublic");
  }
  setEmailId(v: number | null): void {
    this.emailId = v;
    this.clearFieldError("emailId");
  }
  setTplId(v: number | null): void {
    this.tplId = v;
    this.clearFieldError("tplId");
  }
  setSlaId(v: number | null): void {
    this.slaId = v;
  }
  setManagerId(v: number | null): void {
    this.managerId = v;
  }
  setTicketAutoResponse(v: boolean): void {
    this.ticketAutoResponse = v;
  }
  setMessageAutoResponse(v: boolean): void {
    this.messageAutoResponse = v;
  }
  setAutorespEmailId(v: number | null): void {
    this.autorespEmailId = v;
  }
  setDeptSignature(v: string): void {
    this.deptSignature = v;
  }
  setGroupMembership(v: number): void {
    this.groupMembership = v;
  }

  // group-access matrix (full-replace on save).
  toggleGroup(id: number): void {
    if (this.groupIds.has(id)) this.groupIds.delete(id);
    else this.groupIds.add(id);
  }
  isGroupChecked(id: number): boolean {
    return this.groupIds.has(id);
  }
  selectAllGroups(): void {
    this.groupIds = new Set(this.options.groups.map((g) => g.id));
  }
  selectNoGroups(): void {
    this.groupIds = new Set<number>();
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

  /** Client-side pre-check (BS-030-02/03/04). Returns true when the form is valid. */
  private validate(): boolean {
    const errors: Record<string, string> = {};
    if (!this.emailId) errors.emailId = "Email selection required";
    if (!this.tplId) errors.tplId = "Template selection required";
    if (this.editingDefault && !this.ispublic) {
      errors.ispublic = "System default department cannot be private";
    }
    this.fieldErrors = errors;
    return Object.keys(errors).length === 0;
  }

  /** camelCase write body per the pinned backend contract. */
  private writeBody(): Record<string, unknown> {
    return {
      name: this.name.trim(),
      ispublic: this.ispublic,
      emailId: this.emailId,
      tplId: this.tplId,
      slaId: this.slaId,
      managerId: this.managerId,
      groupMembership: this.groupMembership,
      ticketAutoResponse: this.ticketAutoResponse,
      messageAutoResponse: this.messageAutoResponse,
      autorespEmailId: this.autorespEmailId,
      deptSignature: this.deptSignature,
      groupIds: Array.from(this.groupIds),
    };
  }

  async save(): Promise<boolean> {
    if (!this.validate()) return false;
    this.saving = true;
    try {
      const name = this.name.trim();
      if (this.editingId == null) {
        await this.api.post<{ id: number }>(DEPTS_PATH, this.writeBody());
        this.snackbar.success(`${name} added successfully`);
      } else {
        await this.api.put<{ id: number }>(`${DEPTS_PATH}/${this.editingId}`, this.writeBody());
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
        this.snackbar.error(e instanceof ApiError ? e.message : "Could not save department.");
      }
      return false;
    } finally {
      runInAction(() => {
        this.saving = false;
      });
    }
  }

  // ---------------------------------------------------------------------------
  // Delete (single) — a department with home staff is blocked (message surfaced).
  // ---------------------------------------------------------------------------

  async deleteOne(id: number): Promise<boolean> {
    try {
      await this.api.delete(`${DEPTS_PATH}/${id}`);
      this.snackbar.success("Department deleted");
      await this.loadList();
      return true;
    } catch (e) {
      this.snackbar.error(e instanceof ApiError ? e.message : "Could not delete department.");
      return false;
    }
  }

  // ---------------------------------------------------------------------------
  // Mass actions.
  // ---------------------------------------------------------------------------

  async massAction(action: DeptMassAction): Promise<boolean> {
    const ids = this.selectedIds;
    if (ids.length === 0) {
      this.massError = "You must select at least one department.";
      this.snackbar.error(this.massError);
      return false;
    }
    this.massLoading = true;
    this.massError = null;
    try {
      const res = await this.api.post<{ affected: number; message: string }>(`${DEPTS_PATH}/mass`, {
        action,
        ids,
      });
      if (action === "delete" && res.affected === 0) {
        runInAction(() => {
          this.massError = res.message || "Unable to delete selected departments";
        });
        this.snackbar.error(res.message || "Unable to delete selected departments");
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
