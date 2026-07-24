// Admin staff-management store (TS-M4-B2) — FS-031.2/.3/.4. Owns the staff list
// query state (q/did/gid/tid/sort/order/page), the mass-selection Set, the
// create/edit form working values + inline 422 field errors, and the option
// lists (groups + departments + timezones) that feed the form selects.
//
// TDD-covered in StaffAdminStore.test.ts: list param serialization, selection,
// mass actions, 422 → inline errors, create/edit payload (checkbox + password
// serialization, partial edit sends only changed fields), option loading.
//
// @implements FS-031.2: staff list — filter (q/did/gid/tid), sort, pagination.
// @implements FS-031.3: create / edit a staff account.
// @implements FS-031.4: mass enable / lock / delete.
// @implements BS-031-001: duplicate-username / email surfaces as an inline field error.
// @implements BS-031-014: last-active-administrator save refusal (inline on isadmin).
// @implements BS-031-015: self-action protection surfaced on mass lock/delete.
import { makeAutoObservable, runInAction, toJS } from "mobx";
import { ApiClient } from "../api/apiClient";
import { ApiError } from "../api/types";
import { SnackbarStore } from "./SnackbarStore";
import { AdminSettingsStore } from "./AdminSettingsStore";
import type { PaginationMeta } from "./StaffTicketStore";

const STAFF_PATH = "/api/staff/admin/staff";
const GROUPS_PATH = "/api/staff/admin/groups";
const TEAMS_PATH = "/api/staff/admin/teams";

/** A team the staff member belongs to (or can be added to). */
export interface StaffTeam {
  id: number;
  name: string;
}

/** Wire shape of GET /api/staff/admin/teams (only the fields the picker needs). */
interface TeamOptionRow {
  id: number;
  name: string;
}

/** Wire shape of GET /api/staff/admin/teams/:id — used to derive membership. */
interface TeamDetailPayload {
  id: number;
  name: string;
  members: Array<{ staff_id: number; name: string }>;
}

/** A row in the admin staff list. */
export interface StaffRow {
  id: number;
  name: string;
  username: string;
  isactive: boolean;
  onvacation: boolean;
  group_name: string;
  dept_name: string;
  created: string | null;
  lastlogin: string | null;
}

/** Wire shape of GET /api/staff/admin/staff. */
interface StaffListPayload {
  staff: StaffRow[];
  pagination: { page: number; per_page: number; total: number };
}

/** {id,name} option (groups / departments). */
export interface IdName {
  id: number;
  name: string;
}
/** Timezone option (id + label). */
export interface TzOption {
  id: number;
  label: string;
}

export type StaffSortKey =
  | "name"
  | "username"
  | "status"
  | "group"
  | "dept"
  | "created"
  | "login";
export type SortOrder = "ASC" | "DESC";

/** The list query state serialized into the GET querystring. */
export interface StaffListParams {
  q: string;
  did: number | null;
  gid: number | null;
  tid: number | null;
  sort: StaffSortKey;
  order: SortOrder;
  page: number;
}

/** Working values for the create/edit form (all string/boolean for MUI inputs). */
export interface StaffFormValues {
  username: string;
  firstname: string;
  lastname: string;
  email: string;
  phone: string;
  phoneExt: string;
  mobile: string;
  password: string;
  passwd2: string;
  groupId: string;
  deptId: string;
  timezoneId: string;
  isadmin: boolean;
  isactive: boolean;
  isvisible: boolean;
  onvacation: boolean;
  signature: string;
  notes: string;
}

function emptyForm(): StaffFormValues {
  return {
    username: "",
    firstname: "",
    lastname: "",
    email: "",
    phone: "",
    phoneExt: "",
    mobile: "",
    password: "",
    passwd2: "",
    groupId: "",
    deptId: "",
    timezoneId: "",
    isadmin: false,
    isactive: true,
    isvisible: true,
    onvacation: false,
    signature: "",
    notes: "",
  };
}

function defaultParams(): StaffListParams {
  return { q: "", did: null, gid: null, tid: null, sort: "name", order: "ASC", page: 1 };
}

/** Split a combined display name into a best-effort first/last for the edit form. */
function splitName(name: string): { firstname: string; lastname: string } {
  const parts = name.trim().split(/\s+/);
  if (parts.length <= 1) return { firstname: parts[0] ?? "", lastname: "" };
  return { firstname: parts[0], lastname: parts.slice(1).join(" ") };
}

export class StaffAdminStore {
  // --- list ---
  staff: StaffRow[] = [];
  pagination: PaginationMeta | null = null;
  loadingList = false;
  listError: string | null = null;
  params: StaffListParams = defaultParams();

  // --- selection ---
  selected = new Set<number>();

  // --- options ---
  groupOptions: IdName[] = [];
  deptOptions: IdName[] = [];
  timezoneOptions: TzOption[] = [];
  optionsLoaded = false;

  // --- form ---
  formOpen = false;
  editingId: number | null = null;
  form: StaffFormValues = emptyForm();
  private baseline: StaffFormValues = emptyForm();
  fieldErrors: Record<string, string> = {};
  saving = false;

  // --- mass ---
  massLoading = false;
  massError: string | null = null;

  // --- team membership (BS-030-14) ---
  /** Teams the edited staff member currently belongs to (chips + Remove). */
  staffTeams: StaffTeam[] = [];
  /** All teams (feeds the "Add to team" picker via `availableTeams`). */
  allTeams: StaffTeam[] = [];
  teamsLoading = false;
  teamActionPending = false;
  teamsError: string | null = null;

  constructor(
    private readonly api: ApiClient,
    private readonly snackbar: SnackbarStore,
    private readonly adminSettings: AdminSettingsStore,
  ) {
    makeAutoObservable<StaffAdminStore, "api" | "snackbar" | "adminSettings" | "baseline">(
      this,
      { api: false, snackbar: false, adminSettings: false, baseline: false },
      { autoBind: true },
    );
  }

  // ---------------------------------------------------------------------------
  // List.
  // ---------------------------------------------------------------------------

  private buildQuery(): string {
    const p = this.params;
    const sp = new URLSearchParams();
    if (p.q.trim()) sp.set("q", p.q.trim());
    if (p.did != null) sp.set("did", String(p.did));
    if (p.gid != null) sp.set("gid", String(p.gid));
    if (p.tid != null) sp.set("tid", String(p.tid));
    sp.set("sort", p.sort);
    sp.set("order", p.order);
    sp.set("page", String(p.page));
    return sp.toString();
  }

  async loadList(patch: Partial<StaffListParams> = {}): Promise<void> {
    this.params = { ...this.params, ...patch };
    this.loadingList = true;
    this.listError = null;
    try {
      const data = await this.api.get<StaffListPayload>(`${STAFF_PATH}?${this.buildQuery()}`);
      runInAction(() => {
        this.staff = data.staff ?? [];
        const pg = data.pagination;
        const pageSize = pg?.per_page || 25;
        const totalCount = pg?.total ?? 0;
        this.pagination = {
          page: pg?.page ?? 1,
          pageSize,
          totalCount,
          totalPages: Math.max(1, Math.ceil(totalCount / pageSize)),
        };
        // Drop selections no longer on the page.
        const visible = new Set(this.staff.map((s) => s.id));
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

  /** Submit a free-text filter; resets to page 1. */
  search(q: string): Promise<void> {
    return this.loadList({ q, page: 1 });
  }

  /** Toggle-sort a column: same column flips order, new column starts ASC. */
  setSort(sort: StaffSortKey): Promise<void> {
    const order: SortOrder =
      this.params.sort === sort && this.params.order === "ASC" ? "DESC" : "ASC";
    return this.loadList({ sort, order, page: 1 });
  }

  setPage(page: number): Promise<void> {
    return this.loadList({ page });
  }

  setFilter(key: "did" | "gid" | "tid", value: number | null): Promise<void> {
    return this.loadList({ [key]: value, page: 1 } as Partial<StaffListParams>);
  }

  // ---------------------------------------------------------------------------
  // Selection.
  // ---------------------------------------------------------------------------

  toggleSelect(id: number): void {
    if (this.selected.has(id)) this.selected.delete(id);
    else this.selected.add(id);
  }

  get allSelected(): boolean {
    return this.staff.length > 0 && this.staff.every((s) => this.selected.has(s.id));
  }
  get someSelected(): boolean {
    return this.selected.size > 0 && !this.allSelected;
  }

  toggleSelectAll(): void {
    if (this.allSelected) this.selected.clear();
    else for (const s of this.staff) this.selected.add(s.id);
  }

  clearSelection(): void {
    this.selected.clear();
  }

  get selectedIds(): number[] {
    return Array.from(this.selected);
  }

  // ---------------------------------------------------------------------------
  // Options (groups from admin groups list; depts + timezones from settings).
  // ---------------------------------------------------------------------------

  async loadOptions(): Promise<void> {
    try {
      const groups = await this.api.get<Array<{ id: number; name: string }>>(GROUPS_PATH);
      if (!this.adminSettings.loaded) await this.adminSettings.load();
      runInAction(() => {
        this.groupOptions = (groups ?? []).map((g) => ({ id: g.id, name: g.name }));
        this.deptOptions = (this.adminSettings.options.departments ?? []).map((d) => ({
          id: Number(d.id),
          name: d.name,
        }));
        this.timezoneOptions = (this.adminSettings.options.timezones ?? []).map((t) => ({
          id: Number(t.id),
          label: t.label,
        }));
        this.optionsLoaded = true;
      });
    } catch (e) {
      runInAction(() => {
        this.snackbar.error(e instanceof ApiError ? e.message : "Could not load form options.");
      });
    }
  }

  // ---------------------------------------------------------------------------
  // Form.
  // ---------------------------------------------------------------------------

  openCreate(): void {
    this.editingId = null;
    this.form = emptyForm();
    this.baseline = emptyForm();
    this.fieldErrors = {};
    this.resetTeams();
    this.formOpen = true;
    void this.loadOptions();
  }

  /**
   * Open the edit form for a list row. There is no GET single-staff endpoint, so
   * the form prefills from the row (username, status, vacation, group/dept via
   * name→option match, name split into first/last). `ownIsAdmin` supplies the
   * isadmin/isvisible seed when the admin edits their own account. The PUT is a
   * partial update — only fields the user actually changes are submitted.
   */
  openEdit(row: StaffRow, opts?: { ownIsAdmin?: boolean; ownIsVisible?: boolean }): void {
    this.editingId = row.id;
    const { firstname, lastname } = splitName(row.name);
    const group = this.groupOptions.find((g) => g.name === row.group_name);
    const dept = this.deptOptions.find((d) => d.name === row.dept_name);
    const seeded: StaffFormValues = {
      ...emptyForm(),
      username: row.username,
      firstname,
      lastname,
      isactive: row.isactive,
      onvacation: row.onvacation,
      isadmin: opts?.ownIsAdmin ?? false,
      isvisible: opts?.ownIsVisible ?? true,
      groupId: group ? String(group.id) : "",
      deptId: dept ? String(dept.id) : "",
    };
    this.form = seeded;
    this.baseline = { ...seeded };
    this.fieldErrors = {};
    this.resetTeams();
    this.formOpen = true;
    // The Teams section (BS-030-14) is loaded by the dialog effect on editingId.
    if (!this.optionsLoaded) {
      void this.loadOptions().then(() => {
        // Re-match group/dept once the option lists arrive.
        runInAction(() => {
          if (!this.form.groupId) {
            const g = this.groupOptions.find((x) => x.name === row.group_name);
            if (g) {
              this.form.groupId = String(g.id);
              this.baseline.groupId = String(g.id);
            }
          }
          if (!this.form.deptId) {
            const d = this.deptOptions.find((x) => x.name === row.dept_name);
            if (d) {
              this.form.deptId = String(d.id);
              this.baseline.deptId = String(d.id);
            }
          }
        });
      });
    }
  }

  closeForm(): void {
    this.formOpen = false;
    this.editingId = null;
    this.fieldErrors = {};
    this.form = emptyForm();
    this.baseline = emptyForm();
    this.resetTeams();
  }

  private resetTeams(): void {
    this.staffTeams = [];
    this.allTeams = [];
    this.teamsError = null;
    this.teamActionPending = false;
  }

  setField<K extends keyof StaffFormValues>(key: K, value: StaffFormValues[K]): void {
    this.form[key] = value;
    if (this.fieldErrors[key as string]) {
      const { [key as string]: _drop, ...rest } = this.fieldErrors;
      this.fieldErrors = rest;
    }
  }

  fieldError(key: keyof StaffFormValues | string): string | undefined {
    return this.fieldErrors[key as string];
  }

  get isEditing(): boolean {
    return this.editingId !== null;
  }

  get formDirty(): boolean {
    return JSON.stringify(toJS(this.form)) !== JSON.stringify(this.baseline);
  }

  /** Build the create body (all fields; booleans + password sent explicitly). */
  private createBody(): Record<string, unknown> {
    const f = this.form;
    return {
      username: f.username.trim(),
      firstname: f.firstname.trim(),
      lastname: f.lastname.trim(),
      email: f.email.trim(),
      phone: f.phone.trim(),
      phoneExt: f.phoneExt.trim(),
      mobile: f.mobile.trim(),
      password: f.password,
      passwd2: f.passwd2,
      groupId: f.groupId ? Number(f.groupId) : null,
      deptId: f.deptId ? Number(f.deptId) : null,
      timezoneId: f.timezoneId ? Number(f.timezoneId) : null,
      isadmin: f.isadmin,
      isactive: f.isactive,
      isvisible: f.isvisible,
      onvacation: f.onvacation,
      signature: f.signature,
      notes: f.notes,
    };
  }

  /** Build the edit body — only fields changed from the baseline (partial PUT). */
  private updateBody(): Record<string, unknown> {
    const f = this.form;
    const b = this.baseline;
    const out: Record<string, unknown> = {};
    const strKeys: Array<keyof StaffFormValues> = [
      "username",
      "firstname",
      "lastname",
      "email",
      "phone",
      "phoneExt",
      "mobile",
      "signature",
      "notes",
    ];
    for (const k of strKeys) {
      if (f[k] !== b[k]) out[k] = String(f[k]).trim();
    }
    if (f.groupId !== b.groupId && f.groupId) out.groupId = Number(f.groupId);
    if (f.deptId !== b.deptId && f.deptId) out.deptId = Number(f.deptId);
    if (f.timezoneId !== b.timezoneId && f.timezoneId) out.timezoneId = Number(f.timezoneId);
    if (f.isadmin !== b.isadmin) out.isadmin = f.isadmin;
    if (f.isactive !== b.isactive) out.isactive = f.isactive;
    if (f.isvisible !== b.isvisible) out.isvisible = f.isvisible;
    if (f.onvacation !== b.onvacation) out.onvacation = f.onvacation;
    // Password only when a new value was typed.
    if (f.password) {
      out.password = f.password;
      out.passwd2 = f.passwd2;
    }
    return out;
  }

  /**
   * Persist the form. Success → snackbar + reload + close. A 422 maps
   * `error.fields` to inline per-field errors (dialog stays open, no reload).
   * Returns true on success.
   */
  async save(): Promise<boolean> {
    this.saving = true;
    this.fieldErrors = {};
    try {
      const name = `${this.form.firstname} ${this.form.lastname}`.trim() || this.form.username;
      if (this.editingId == null) {
        await this.api.post<{ id: number }>(STAFF_PATH, this.createBody());
        this.snackbar.success(`${name} added successfully`);
      } else {
        await this.api.put<{ id: number }>(`${STAFF_PATH}/${this.editingId}`, this.updateBody());
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
        this.snackbar.error(e instanceof ApiError ? e.message : "Could not save staff.");
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

  /**
   * Run a mass action over the current selection. Success → snackbar + reload +
   * clear selection. A 422 (self/last-admin/empty) surfaces via the error
   * snackbar and `massError`. Returns true on success.
   */
  async massAction(action: "enable" | "lock" | "delete"): Promise<boolean> {
    const ids = this.selectedIds;
    if (ids.length === 0) {
      this.massError = "You must select at least one staff member.";
      this.snackbar.error(this.massError);
      return false;
    }
    this.massLoading = true;
    this.massError = null;
    try {
      const res = await this.api.post<{ affected: number; message: string }>(
        `${STAFF_PATH}/mass`,
        { action, ids },
      );
      this.snackbar.success(res.message || "Done");
      runInAction(() => {
        this.selected.clear();
      });
      await this.loadList();
      return true;
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

  // ---------------------------------------------------------------------------
  // Team membership from the staff profile (BS-030-14).
  //
  // There is no GET single-staff endpoint, so a member's teams are DERIVED from
  // the team roster: fetch every team (the picker source) + each team's members,
  // then keep the teams whose roster contains this staff id. Add/remove hit the
  // dedicated staff-side membership routes and refetch to re-derive.
  // ---------------------------------------------------------------------------

  /** Teams the member is NOT yet in — the "Add to team" picker source. */
  get availableTeams(): StaffTeam[] {
    const inIds = new Set(this.staffTeams.map((t) => t.id));
    return this.allTeams.filter((t) => !inIds.has(t.id));
  }

  /**
   * Load `allTeams` and derive `staffTeams` (the teams `staffId` belongs to) from
   * each team's member roster. Errors surface via `teamsError` (no throw).
   */
  async loadStaffTeams(staffId: number): Promise<void> {
    this.teamsLoading = true;
    this.teamsError = null;
    try {
      const teams = (await this.api.get<TeamOptionRow[]>(TEAMS_PATH)) ?? [];
      const details = await Promise.all(
        teams.map((t) => this.api.get<TeamDetailPayload>(`${TEAMS_PATH}/${t.id}`)),
      );
      const memberOf = details
        .filter((d) => d.members.some((m) => m.staff_id === staffId))
        .map((d) => ({ id: d.id, name: d.name }));
      runInAction(() => {
        this.allTeams = teams.map((t) => ({ id: t.id, name: t.name }));
        this.staffTeams = memberOf;
      });
    } catch (e) {
      runInAction(() => {
        this.teamsError = e instanceof ApiError ? e.message : String(e);
      });
    } finally {
      runInAction(() => {
        this.teamsLoading = false;
      });
    }
  }

  /**
   * Add `staffId` to `teamId` (BS-030-14) → refetch + success snackbar. Any error
   * surfaces via `teamsError` + the error snackbar. Returns true on success.
   */
  async addToTeam(staffId: number, teamId: number): Promise<boolean> {
    this.teamActionPending = true;
    this.teamsError = null;
    try {
      await this.api.post(`${STAFF_PATH}/${staffId}/teams`, { teamId });
      const name = this.allTeams.find((t) => t.id === teamId)?.name ?? "team";
      await this.loadStaffTeams(staffId);
      this.snackbar.success(`Added to ${name}`);
      return true;
    } catch (e) {
      const msg = e instanceof ApiError ? e.message : "Could not add to team.";
      runInAction(() => {
        this.teamsError = msg;
      });
      this.snackbar.error(msg);
      return false;
    } finally {
      runInAction(() => {
        this.teamActionPending = false;
      });
    }
  }

  /**
   * Remove `staffId` from `teamId` (BS-030-14) → refetch + success snackbar.
   * Returns true on success.
   */
  async removeFromTeam(staffId: number, teamId: number): Promise<boolean> {
    this.teamActionPending = true;
    this.teamsError = null;
    const name = this.staffTeams.find((t) => t.id === teamId)?.name ?? "team";
    try {
      await this.api.delete(`${STAFF_PATH}/${staffId}/teams/${teamId}`);
      await this.loadStaffTeams(staffId);
      this.snackbar.success(`Removed from ${name}`);
      return true;
    } catch (e) {
      const msg = e instanceof ApiError ? e.message : "Could not remove from team.";
      runInAction(() => {
        this.teamsError = msg;
      });
      this.snackbar.error(msg);
      return false;
    } finally {
      runInAction(() => {
        this.teamActionPending = false;
      });
    }
  }
}
