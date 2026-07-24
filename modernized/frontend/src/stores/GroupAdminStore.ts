// Admin permission-group store (TS-M4-B4) — FS-031.7/.8. Owns the group list,
// the create/edit form (name + status + the canonical eleven Yes/No flags per
// BS-031-020 + the department-access matrix as a full-replace Set), inline 422
// field errors, and the mass-selection Set.
//
// TDD-covered in GroupAdminStore.test.ts: flag boolean defaults, dept-matrix Set
// + select-all/none, dirty tracking, full-replace payload ({flags map, deptIds}),
// 422 → inline errors, mass actions.
//
// @implements FS-031.7: group list (member/dept counts, sort, mass actions).
// @implements FS-031.8: create / edit a group (eleven flags + dept matrix).
// @implements BS-031-019: group name required, >= 3 chars, unique (inline 422).
// @implements BS-031-020: canonical eleven permission-flag set.
// @implements BS-031-021: department-access set is a full-replace sync.
// @implements BS-031-023: self-group protection surfaced on mass actions.
import { makeAutoObservable, runInAction } from "mobx";
import { ApiClient } from "../api/apiClient";
import { ApiError } from "../api/types";
import { SnackbarStore } from "./SnackbarStore";
import { AdminSettingsStore } from "./AdminSettingsStore";
import type { IdName } from "./StaffAdminStore";

const GROUPS_PATH = "/api/staff/admin/groups";

/** The canonical eleven permission flags (BS-031-020), in storage-key order. */
export const GROUP_FLAG_KEYS = [
  "can_create_tickets",
  "can_edit_tickets",
  "can_post_reply",
  "can_close_tickets",
  "can_assign_tickets",
  "can_transfer_tickets",
  "can_delete_tickets",
  "can_manage_faq",
  "can_manage_premade",
  "can_ban_emails",
  "can_view_staff_stats",
] as const;

export type GroupFlagKey = (typeof GROUP_FLAG_KEYS)[number];

/** Human labels + hint text for each flag (rendered by PermissionGrid). */
export const GROUP_FLAG_META: Record<GroupFlagKey, { label: string; hint: string }> = {
  can_create_tickets: { label: "Can Create Tickets", hint: "Ability to open tickets on behalf of users." },
  can_edit_tickets: { label: "Can Edit Tickets", hint: "Ability to edit ticket details." },
  can_post_reply: { label: "Can Post Reply", hint: "Ability to post a reply to a ticket." },
  can_close_tickets: { label: "Can Close Tickets", hint: "Ability to close tickets." },
  can_assign_tickets: { label: "Can Assign Tickets", hint: "Ability to assign tickets to staff." },
  can_transfer_tickets: { label: "Can Transfer Tickets", hint: "Ability to transfer tickets between departments." },
  can_delete_tickets: { label: "Can Delete Tickets", hint: "Ability to delete tickets permanently." },
  can_manage_faq: { label: "Can Manage FAQ", hint: "Ability to manage the knowledge base / FAQ." },
  can_manage_premade: { label: "Can Manage Premade", hint: "Ability to manage canned (premade) responses." },
  can_ban_emails: { label: "Can Ban Emails", hint: "Ability to ban email addresses." },
  can_view_staff_stats: { label: "Can View Staff Stats", hint: "Ability to view other agents' statistics." },
};

/** A row in the group list. */
export interface GroupRow {
  id: number;
  name: string;
  enabled: boolean;
  member_count: number;
  dept_count: number;
  created: string | null;
  updated: string | null;
}

/** Wire shape of GET /api/staff/admin/groups/:id. */
interface GroupDetail {
  id: number;
  name: string;
  enabled: boolean;
  notes: string;
  flags: Record<string, boolean>;
  dept_ids: number[];
}

export type GroupSortKey = "name" | "status" | "users" | "depts" | "created" | "updated";
export type SortOrder = "ASC" | "DESC";

function emptyFlags(): Record<GroupFlagKey, boolean> {
  const out = {} as Record<GroupFlagKey, boolean>;
  for (const k of GROUP_FLAG_KEYS) out[k] = false;
  return out;
}

export class GroupAdminStore {
  // --- list ---
  groups: GroupRow[] = [];
  loadingList = false;
  listError: string | null = null;
  sort: GroupSortKey = "name";
  order: SortOrder = "ASC";

  // --- selection ---
  selected = new Set<number>();

  // --- options (departments for the matrix) ---
  deptOptions: IdName[] = [];
  optionsLoaded = false;

  // --- form ---
  formOpen = false;
  editingId: number | null = null;
  name = "";
  enabled = true;
  notes = "";
  flags: Record<GroupFlagKey, boolean> = emptyFlags();
  deptIds = new Set<number>();
  fieldErrors: Record<string, string> = {};
  saving = false;
  private baselineKey = "";

  // --- mass ---
  massLoading = false;
  massError: string | null = null;

  constructor(
    private readonly api: ApiClient,
    private readonly snackbar: SnackbarStore,
    private readonly adminSettings: AdminSettingsStore,
  ) {
    makeAutoObservable<GroupAdminStore, "api" | "snackbar" | "adminSettings" | "baselineKey">(
      this,
      { api: false, snackbar: false, adminSettings: false, baselineKey: false },
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
      const sp = new URLSearchParams({ sort: this.sort, order: this.order });
      const data = await this.api.get<GroupRow[]>(`${GROUPS_PATH}?${sp.toString()}`);
      runInAction(() => {
        this.groups = data ?? [];
        const visible = new Set(this.groups.map((g) => g.id));
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

  setSort(sort: GroupSortKey): Promise<void> {
    this.order = this.sort === sort && this.order === "ASC" ? "DESC" : "ASC";
    this.sort = sort;
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
    return this.groups.length > 0 && this.groups.every((g) => this.selected.has(g.id));
  }
  get someSelected(): boolean {
    return this.selected.size > 0 && !this.allSelected;
  }
  toggleSelectAll(): void {
    if (this.allSelected) this.selected.clear();
    else for (const g of this.groups) this.selected.add(g.id);
  }
  clearSelection(): void {
    this.selected.clear();
  }
  get selectedIds(): number[] {
    return Array.from(this.selected);
  }

  // ---------------------------------------------------------------------------
  // Options (departments for the dept-access matrix).
  // ---------------------------------------------------------------------------

  async loadOptions(): Promise<void> {
    try {
      if (!this.adminSettings.loaded) await this.adminSettings.load();
      runInAction(() => {
        this.deptOptions = (this.adminSettings.options.departments ?? []).map((d) => ({
          id: Number(d.id),
          name: d.name,
        }));
        this.optionsLoaded = true;
      });
    } catch (e) {
      this.snackbar.error(e instanceof ApiError ? e.message : "Could not load departments.");
    }
  }

  // ---------------------------------------------------------------------------
  // Form.
  // ---------------------------------------------------------------------------

  private snapshotKey(): string {
    return JSON.stringify({
      name: this.name,
      enabled: this.enabled,
      notes: this.notes,
      flags: this.flags,
      depts: Array.from(this.deptIds).sort((a, b) => a - b),
    });
  }

  private resetForm(): void {
    this.name = "";
    this.enabled = true;
    this.notes = "";
    this.flags = emptyFlags();
    this.deptIds = new Set<number>();
    this.fieldErrors = {};
  }

  openCreate(): void {
    this.editingId = null;
    this.resetForm();
    this.baselineKey = this.snapshotKey();
    this.formOpen = true;
    void this.loadOptions();
  }

  /** Open the edit form and fetch the group detail (flags + dept set). */
  async openEdit(id: number): Promise<void> {
    this.editingId = id;
    this.resetForm();
    this.formOpen = true;
    await this.loadOptions();
    try {
      const d = await this.api.get<GroupDetail>(`${GROUPS_PATH}/${id}`);
      runInAction(() => {
        this.name = d.name;
        this.enabled = d.enabled;
        this.notes = d.notes ?? "";
        const f = emptyFlags();
        for (const k of GROUP_FLAG_KEYS) f[k] = Boolean(d.flags?.[k]);
        this.flags = f;
        this.deptIds = new Set(d.dept_ids ?? []);
        this.baselineKey = this.snapshotKey();
      });
    } catch (e) {
      this.snackbar.error(e instanceof ApiError ? e.message : "Could not load group.");
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
  setEnabled(v: boolean): void {
    this.enabled = v;
  }
  setNotes(v: string): void {
    this.notes = v;
  }
  setFlag(key: GroupFlagKey, v: boolean): void {
    this.flags = { ...this.flags, [key]: v };
  }

  // dept matrix (full-replace semantics on save).
  toggleDept(id: number): void {
    if (this.deptIds.has(id)) this.deptIds.delete(id);
    else this.deptIds.add(id);
  }
  isDeptChecked(id: number): boolean {
    return this.deptIds.has(id);
  }
  selectAllDepts(): void {
    this.deptIds = new Set(this.deptOptions.map((d) => d.id));
  }
  selectNoDepts(): void {
    this.deptIds = new Set<number>();
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

  /** Full-replace write body: name, enabled, the eleven flags map, deptIds, notes. */
  private writeBody(): Record<string, unknown> {
    return {
      name: this.name.trim(),
      enabled: this.enabled,
      flags: { ...this.flags },
      deptIds: Array.from(this.deptIds),
      notes: this.notes,
    };
  }

  /** Persist. Success → snackbar + reload + close. 422 → inline field errors. */
  async save(): Promise<boolean> {
    this.saving = true;
    this.fieldErrors = {};
    try {
      const name = this.name.trim();
      if (this.editingId == null) {
        await this.api.post<{ id: number }>(GROUPS_PATH, this.writeBody());
        this.snackbar.success(`${name} added successfully`);
      } else {
        await this.api.put<{ id: number }>(`${GROUPS_PATH}/${this.editingId}`, this.writeBody());
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
        this.snackbar.error(e instanceof ApiError ? e.message : "Could not save group.");
      }
      return false;
    } finally {
      runInAction(() => {
        this.saving = false;
      });
    }
  }

  // ---------------------------------------------------------------------------
  // Mass actions.
  // ---------------------------------------------------------------------------

  async massAction(action: "enable" | "disable" | "delete"): Promise<boolean> {
    const ids = this.selectedIds;
    if (ids.length === 0) {
      this.massError = "You must select at least one group.";
      this.snackbar.error(this.massError);
      return false;
    }
    this.massLoading = true;
    this.massError = null;
    try {
      const res = await this.api.post<{ affected: number; message: string }>(
        `${GROUPS_PATH}/mass`,
        { action, ids },
      );
      // The delete route reports partial/blocked outcomes via `message` with
      // affected=0 (BS-031-022); surface those as an error notice, not success.
      if (action === "delete" && res.affected === 0) {
        runInAction(() => {
          this.massError = res.message || "Unable to delete selected groups";
        });
        this.snackbar.error(res.message || "Unable to delete selected groups");
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
