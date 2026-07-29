// Staff ticket-options reference store: loads GET /api/staff/ticket-options once
// and exposes the six id+name reference lists that power the ticket Transfer /
// Assign / Edit-properties dialogs and the advanced-search filters. Available to
// ANY authenticated staff session (not admin-gated) — see the backend handler
// `staff::ticket_options`.
//
// This replaces the hardcoded mock arrays (literal DB ids) that previously drove
// those dialogs in StaffArea. The store fetches once and caches; call `load(true)`
// to force a refresh.
//
// @implements FS-032.10/.11/.12: staff transfer/assign/edit reference data.
// @implements BS-020.2: read-only reference lists available to any staff session.
import { makeAutoObservable, runInAction } from "mobx";
import { ApiClient } from "../api/apiClient";
import { ApiError } from "../api/types";

const TICKET_OPTIONS_PATH = "/api/staff/ticket-options";

/** A basic reference option: numeric DB id + display name. */
export interface IdNameOption {
  id: number;
  name: string;
}

/** A combined staff-or-team assignee, id namespaced by kind (`s<id>` / `t<id>`). */
export interface AssigneeOption {
  /** Prefixed id the assign route expects: `s<staffId>` or `t<teamId>`. */
  id: string;
  name: string;
  type: "staff" | "team";
}

/** Wire shape of GET /api/staff/ticket-options. */
interface TicketOptionsPayload {
  departments: IdNameOption[];
  agents: IdNameOption[];
  teams: IdNameOption[];
  help_topics: IdNameOption[];
  priorities: IdNameOption[];
  sla_plans: IdNameOption[];
}

export class TicketOptionsStore {
  departments: IdNameOption[] = [];
  agents: IdNameOption[] = [];
  teams: IdNameOption[] = [];
  helpTopics: IdNameOption[] = [];
  priorities: IdNameOption[] = [];
  slaPlans: IdNameOption[] = [];

  loading = false;
  loaded = false;
  error: string | null = null;

  constructor(private readonly api: ApiClient) {
    makeAutoObservable<TicketOptionsStore, "api">(this, { api: false }, { autoBind: true });
  }

  /**
   * Fetch the reference lists once. Subsequent calls are no-ops while already
   * loaded (the reference data is stable within a session); pass `force` to
   * refetch. An in-flight load is not duplicated.
   */
  async load(force = false): Promise<void> {
    if (this.loading) return;
    if (this.loaded && !force) return;
    this.loading = true;
    this.error = null;
    try {
      const data = await this.api.get<TicketOptionsPayload>(TICKET_OPTIONS_PATH);
      runInAction(() => {
        this.departments = data.departments ?? [];
        this.agents = data.agents ?? [];
        this.teams = data.teams ?? [];
        this.helpTopics = data.help_topics ?? [];
        this.priorities = data.priorities ?? [];
        this.slaPlans = data.sla_plans ?? [];
        this.loaded = true;
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

  /**
   * Combined assignee list for the Assign dialog: active staff (namespaced `s<id>`,
   * type "staff") followed by enabled teams (`t<id>`, type "team"). The prefixed id
   * is exactly what POST …/assign expects as its `assignee`.
   */
  get assignees(): AssigneeOption[] {
    return [
      ...this.agents.map((a) => ({ id: `s${a.id}`, name: a.name, type: "staff" as const })),
      ...this.teams.map((t) => ({ id: `t${t.id}`, name: t.name, type: "team" as const })),
    ];
  }
}
