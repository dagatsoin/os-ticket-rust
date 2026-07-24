// Admin team store (TS-M4-C4) — FS-030.8/.9. Owns the team list, the create/edit
// form (name, enabled, assignment-alert override, lead select from CURRENT members
// only, and a remove-only member roster — there is NO add-member control here;
// members are added from a staff profile per BS-030-14), inline 422 field errors,
// and the mass-selection Set.
//
// TDD-covered in TeamAdminStore.test.ts: list load, member-detail load into the
// roster, remove-toggle, lead reset when the lead is marked for removal, lead
// options exclude removed members, write body ({name,isenabled,leadId,noalerts,
// notes,removeMemberIds}), 422 → inline errors, mass actions.
//
// @implements FS-030.8: team list (status, member count, lead) + mass actions.
// @implements FS-030.9: create / edit a team (lead from members + remove roster).
// @implements BS-030-14: no add-member control on the team form; removing the lead resets it.
import { makeAutoObservable, runInAction } from "mobx";
import { ApiClient } from "../api/apiClient";
import { ApiError } from "../api/types";
import { SnackbarStore } from "./SnackbarStore";

const TEAMS_PATH = "/api/staff/admin/teams";

/** A row in the team list. */
export interface TeamRow {
  id: number;
  name: string;
  isenabled: boolean;
  lead_id: number | null;
  lead_name: string | null;
  member_count: number;
  updated: string | null;
}

/** A team member ({staff_id,name}). */
export interface TeamMember {
  staff_id: number;
  name: string;
}

/** Wire shape of GET /api/staff/admin/teams/:id (row + members). */
interface TeamDetail extends TeamRow {
  members: TeamMember[];
  noalerts?: boolean;
  notes?: string;
}

export type TeamMassAction = "enable" | "disable" | "delete";

export class TeamAdminStore {
  // --- list ---
  rows: TeamRow[] = [];
  loadingList = false;
  listError: string | null = null;

  // --- selection ---
  selected = new Set<number>();

  // --- form ---
  formOpen = false;
  editingId: number | null = null;
  loadingDetail = false;
  name = "";
  isenabled = true;
  noalerts = false;
  leadId: number | null = null;
  notes = "";
  members: TeamMember[] = [];
  removeMemberIds = new Set<number>();
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
    makeAutoObservable<TeamAdminStore, "api" | "snackbar" | "baselineKey">(
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
      const data = await this.api.get<TeamRow[]>(TEAMS_PATH);
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
      isenabled: this.isenabled,
      noalerts: this.noalerts,
      leadId: this.leadId,
      notes: this.notes,
      remove: Array.from(this.removeMemberIds).sort((a, b) => a - b),
    });
  }

  private resetForm(): void {
    this.name = "";
    this.isenabled = true;
    this.noalerts = false;
    this.leadId = null;
    this.notes = "";
    this.members = [];
    this.removeMemberIds = new Set<number>();
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
      const d = await this.api.get<TeamDetail>(`${TEAMS_PATH}/${id}`);
      runInAction(() => {
        this.name = d.name;
        this.isenabled = d.isenabled;
        this.noalerts = d.noalerts ?? false;
        this.leadId = d.lead_id ?? null;
        this.notes = d.notes ?? "";
        this.members = d.members ?? [];
        this.baselineKey = this.snapshotKey();
      });
    } catch (e) {
      this.snackbar.error(e instanceof ApiError ? e.message : "Could not load team.");
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
  setIsenabled(v: boolean): void {
    this.isenabled = v;
  }
  setNoalerts(v: boolean): void {
    this.noalerts = v;
  }
  setLeadId(v: number | null): void {
    this.leadId = v;
  }
  setNotes(v: string): void {
    this.notes = v;
  }

  /**
   * Toggle a member's removal flag. Marking the current lead for removal resets
   * the lead to null (BS-030-14) — the lead must be a retained member.
   */
  toggleRemoveMember(staffId: number): void {
    if (this.removeMemberIds.has(staffId)) {
      this.removeMemberIds.delete(staffId);
    } else {
      this.removeMemberIds.add(staffId);
      if (this.leadId === staffId) this.leadId = null;
    }
  }
  isMarkedForRemoval(staffId: number): boolean {
    return this.removeMemberIds.has(staffId);
  }

  /** Lead candidates: current members NOT marked for removal. */
  get leadCandidates(): TeamMember[] {
    return this.members.filter((m) => !this.removeMemberIds.has(m.staff_id));
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
      isenabled: this.isenabled,
      leadId: this.leadId,
      noalerts: this.noalerts,
      notes: this.notes,
      removeMemberIds: Array.from(this.removeMemberIds),
    };
  }

  async save(): Promise<boolean> {
    this.saving = true;
    this.fieldErrors = {};
    try {
      const name = this.name.trim();
      if (this.editingId == null) {
        await this.api.post<{ id: number }>(TEAMS_PATH, this.writeBody());
        this.snackbar.success(`${name} added successfully`);
      } else {
        await this.api.put<{ id: number }>(`${TEAMS_PATH}/${this.editingId}`, this.writeBody());
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
        this.snackbar.error(e instanceof ApiError ? e.message : "Could not save team.");
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
      await this.api.delete(`${TEAMS_PATH}/${id}`);
      this.snackbar.success("Team deleted");
      await this.loadList();
      return true;
    } catch (e) {
      this.snackbar.error(e instanceof ApiError ? e.message : "Could not delete team.");
      return false;
    }
  }

  // ---------------------------------------------------------------------------
  // Mass actions.
  // ---------------------------------------------------------------------------

  async massAction(action: TeamMassAction): Promise<boolean> {
    const ids = this.selectedIds;
    if (ids.length === 0) {
      this.massError = "You must select at least one team.";
      this.snackbar.error(this.massError);
      return false;
    }
    this.massLoading = true;
    this.massError = null;
    try {
      const res = await this.api.post<{ affected: number; message: string }>(`${TEAMS_PATH}/mass`, {
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
