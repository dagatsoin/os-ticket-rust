// Staff queue + ticket-detail/reply store (TS-M1-C4, TDD target).
//
// Loads the open-tickets queue (served newest-first), a single ticket detail
// (full M/R/N thread, created ASC), and posts a reply that REFRESHES the thread
// from the reply RESPONSE (the backend returns the updated detail) — no
// optimistic local append.
//
// @implements BS-020: open-tickets queue (number/subject/email/created).
// @implements BS-021: ticket detail (ordered full thread) + reply-then-refresh.
// @implements FS-022.14: canned-response selection fills the reply + carries attachments (TS-M2-D3).
// @implements BS-022.2: canned list is enabled + dept-scoped (served by the route).
// @implements FS-020.2: queue tabs with counts (TS-M3-A4).
// @implements FS-020.6: pagination (TS-M3-B4).
// @implements BS-020.8: sticky sort (TS-M3-B4).
// @implements FS-020.3: overdue badge display (TS-M3-D5).
// @implements FS-020.7: search box + advanced search (TS-M3-F3).
// @implements BS-020.9: bulk selection + action bar (TS-M3-G2).
// @implements FS-021.15: edit form (TS-M3-I4).
// @implements FS-021.19: delete confirmation (TS-M3-I5).
// @implements FS-021.18: lock UI (TS-M3-I6).
import { makeAutoObservable, runInAction } from "mobx";
import { ApiClient } from "../api/apiClient";
import { ApiError } from "../api/types";
import type { Attachment } from "../utils/downloadAttachment";
import { validateAttachment } from "../utils/validateAttachment";

/** Queue status values accepted by the backend. */
export type QueueStatus = "open" | "answered" | "assigned" | "overdue" | "closed";

/** Sort direction. */
export type SortOrder = "ASC" | "DESC";

/** Valid sort keys per TS-M3-B1. */
export type SortKey = "date" | "ID" | "subj" | "name" | "pri" | "assignee" | "staff" | "dept";

/** Quick stats returned by GET /api/staff/tickets/stats. */
export interface QueueStats {
  open: number;
  answered: number;
  overdue: number;
  assigned: number;
  closed: number;
  /** When true, answered tickets are folded into the Open count; Answered tab is hidden. */
  show_answered_tickets: boolean;
  /** When true, assigned tickets are included in Open queue. */
  show_assigned_tickets: boolean;
}

/** Pagination metadata from the queue response. */
export interface PaginationMeta {
  page: number;
  pageSize: number;
  totalCount: number;
  totalPages: number;
}

/** Sort preferences per queue (from GET /api/staff/tickets/sort-prefs). */
export interface SortPrefs {
  [queue: string]: { sort: SortKey; order: SortOrder } | undefined;
}

/** Params for loading the queue. */
export interface LoadQueueParams {
  status?: QueueStatus;
  sort?: SortKey;
  order?: SortOrder;
  page?: number;
  limit?: number;
  /** If true, also refresh stats after loading queue. */
  refreshStats?: boolean;
}

/** One enabled, dept-scoped canned response (GET .../canned). */
export interface CannedSummary {
  id: number;
  title: string;
}

/** A selected canned response's substituted body + carried attachments (GET .../canned/:id). */
export interface CannedDetail {
  /** The body with `%{token}`s already substituted server-side. */
  body: string;
  attachments: Attachment[];
}

/** One row of the staff open-tickets queue (GET /api/staff/tickets). */
export interface QueueItem {
  id: number;
  number: number;
  subject: string;
  email: string;
  created: string;
  /** Answered / Unanswered flag shown as a queue badge (ROADMAP M2 §3). */
  isanswered?: boolean;
  /** True if the ticket is past its due date (TS-M3-D5). */
  isoverdue?: boolean;
  /** Due date/time for SLA (ISO string, TS-M3-D5). */
  duedate?: string;
  /** Priority name (e.g., "Normal", "High"). */
  priority?: string;
  /** Variable column: assigned staff/team name (when rightmost_column=assigned_to). */
  assigned_to_name?: string;
  /** Variable column: closing staff name (when rightmost_column=closed_by). */
  closed_by_name?: string;
  /** Variable column: department name (when rightmost_column=department). */
  department_name?: string;
}

/** Possible rightmost column variants per queue (TS-M3-A3). */
export type RightmostColumn = "assigned_to" | "closed_by" | "department";

/** Full queue response shape from backend. */
interface QueueResponse {
  tickets: QueueItem[];
  rightmostColumn: RightmostColumn;
  pagination: PaginationMeta;
}

/** One thread entry in a staff ticket detail. */
export interface StaffThreadEntry {
  id: number;
  /** `M` (client message) / `R` (staff response) / `N` (internal note). */
  threadType: "M" | "R" | "N";
  poster: string;
  /** Optional title (typically used for internal notes). */
  title?: string;
  body: string;
  /** Attachments carried on this entry (§7); empty/absent when none. */
  attachments?: Attachment[];
}

/** A staff ticket detail (GET /api/staff/tickets/:id; also the reply response).
 * @implements TS-M3-C4: workflow UI requires assignment and status info.
 * @implements TS-M3-E3: note form/display requires thread entries with N type.
 */
export interface TicketDetail {
  id: number;
  number: number;
  subject: string;
  email: string;
  name: string;
  status: string;
  created: string;
  entries: StaffThreadEntry[];
  /** Answered / Unanswered flag shown as a detail badge (ROADMAP M2 §3). */
  isanswered?: boolean;
  /** True if the ticket is past its due date (TS-M3-C4). */
  isoverdue?: boolean;
  /** Due date/time for SLA (ISO string). */
  duedate?: string;
  /** SLA plan id if assigned. */
  slaId?: number;
  /** SLA plan name if assigned. */
  slaName?: string;
  /** Staff ID assigned to this ticket (null = unassigned). */
  staffId?: number | null;
  /** Team ID assigned to this ticket (null = no team). */
  teamId?: number | null;
  /** Display name of assigned staff/team (TS-M3-C4). */
  assignedToName?: string;
  /** Display name of staff who closed the ticket (TS-M3-C4). */
  closedByName?: string;
  /** Department ID of the ticket. */
  deptId?: number;
  /** Department name of the ticket. */
  deptName?: string;
}

/** Staff member for assignment selector (TS-M3-C4). */
export interface StaffOption {
  id: number;
  name: string;
  username: string;
}

/** Team for assignment selector (TS-M3-C4). */
export interface TeamOption {
  id: number;
  name: string;
}

/** Department for transfer selector (TS-M3-C4). */
export interface DepartmentOption {
  id: number;
  name: string;
}

/** Workflow action success response. */
interface WorkflowSuccess {
  success: boolean;
  message: string;
}

/** Transfer action success response with access_lost flag. */
interface TransferSuccess extends WorkflowSuccess {
  accessLost: boolean;
}

/** Note state change options (TS-M3-E3). */
export type NoteState =
  | "unchanged"
  | "closed"
  | "open"
  | "answered"
  | "unanswered"
  | "overdue"
  | "notdue";

/** Note response from POST /api/staff/tickets/:id/note (TS-M3-E3). */
export interface NoteResponse {
  id: number;
  type: string;
  body: string;
  title?: string;
  staffId: number;
  posterName: string;
  stateChanged: boolean;
}

/**
 * Response body of `POST /api/staff/tickets/:id/lock`, covering both shapes of the
 * shared lock contract (the backend half is landing in parallel):
 *  - success carries a lock id — `lockId` (camelCase, current backend) or `lock_id`;
 *  - locked-by-another may be signalled on the body via a `locked_by_other` flag
 *    (nested under `lock` or flat) carrying `locked_by_name`.
 */
interface LockAcquireResponse {
  lockId?: number;
  lock_id?: number;
  remainingSeconds?: number;
  locked_by_other?: boolean;
  locked_by_name?: string;
  lock?: {
    lock_id?: number;
    lockId?: number;
    locked_by_other?: boolean;
    locked_by_name?: string;
  };
}

/**
 * Extract the holder name when the lock-acquire success body indicates the ticket
 * is locked by ANOTHER staff member; `null` when it is not a locked-by-other body.
 */
function lockedByOtherFrom(r: LockAcquireResponse): string | null {
  if (r.lock?.locked_by_other) return r.lock.locked_by_name ?? "another agent";
  if (r.locked_by_other) return r.locked_by_name ?? "another agent";
  return null;
}

/** Extract the acquired lock id from a success body (either casing / nesting). */
function lockIdFrom(r: LockAcquireResponse): number | null {
  return r.lockId ?? r.lock_id ?? r.lock?.lockId ?? r.lock?.lock_id ?? null;
}

/**
 * If the acquire error is a locked-by-another conflict (409 Conflict / 423 Locked),
 * return the holder's display name; otherwise `null` (a genuine failure). Prefers a
 * structured field, falling back to parsing "… locked by <name>" from the message.
 */
function lockConflictHolder(e: unknown): string | null {
  if (!(e instanceof ApiError) || (e.status !== 409 && e.status !== 423)) return null;
  const field = e.fields.locked_by_name ?? e.fields.lockedByName ?? e.fields.name;
  if (field) return field;
  const match = /locked by\s+(.+?)[.!]?$/i.exec(e.message);
  return match ? match[1].trim() : "another agent";
}

export class StaffTicketStore {
  private readonly api: ApiClient;

  queue: QueueItem[] = [];
  loadingQueue = false;
  queueError: string | null = null;

  detail: TicketDetail | null = null;
  loadingDetail = false;
  detailError: string | null = null;

  replying = false;
  replyError: string | null = null;

  // --- Reply composer state (TS-M2-D3) ---
  /** The reply textarea contents (filled by a canned response or typed). */
  replyBody = "";
  /** Available canned responses for this ticket (enabled + dept-scoped). */
  cannedList: CannedSummary[] = [];
  loadingCanned = false;
  /** The currently-selected canned id, retained for the multipart POST. */
  selectedCannedId: number | null = null;
  /** Read-only attachments carried by the selected canned response. */
  cannedAttachments: Attachment[] = [];
  /** The agent's own optional attachment. */
  replyFile: File | null = null;
  /** Client-side pre-check error for the own-file (reuses validateAttachment). */
  replyFileError: string | undefined = undefined;

  // --- Queue tabs + stats (TS-M3-A4) ---
  /** Quick stats for queue tab badges. */
  stats: QueueStats | null = null;
  loadingStats = false;
  statsError: string | null = null;

  // --- Pagination + sort (TS-M3-B4) ---
  /** Pagination metadata from the last queue load. */
  pagination: PaginationMeta | null = null;
  /** The variable rightmost column for the current queue (TS-M3-A3). */
  rightmostColumn: RightmostColumn = "department";
  /** The current sort key (from URL or sticky). */
  currentSort: SortKey | null = null;
  /** The current sort order. */
  currentOrder: SortOrder | null = null;
  /** Sticky sort preferences per queue. */
  sortPrefs: SortPrefs = {};
  loadingSortPrefs = false;
  /** True once sortPrefs have been loaded (even if empty). */
  sortPrefsLoaded = false;

  // --- Workflow state (TS-M3-C4) ---
  /** Loading state for workflow actions. */
  workflowLoading = false;
  /** Error from a workflow action. */
  workflowError: string | null = null;
  /** Success message from a workflow action. */
  workflowMessage: string | null = null;
  /** Available staff for assignment (loaded on dialog open). */
  staffOptions: StaffOption[] = [];
  /** Available teams for assignment (loaded on dialog open). */
  teamOptions: TeamOption[] = [];
  /** Available departments for transfer (loaded on dialog open). */
  deptOptions: DepartmentOption[] = [];

  // --- Note composer state (TS-M3-E3) ---
  /** Note body text. */
  noteBody = "";
  /** Note optional title. */
  noteTitle = "";
  /** Note state change selector. */
  noteState: NoteState = "unchanged";
  /** Loading state for note submission. */
  noteSending = false;
  /** Error from note submission. */
  noteError: string | null = null;

  constructor(api: ApiClient) {
    this.api = api;
    makeAutoObservable(this, {}, { autoBind: true });
  }

  /** True when a non-empty reply can be sent (and the own-file passes pre-check). */
  get canReply(): boolean {
    return this.replyBody.trim() !== "" && this.replyFileError === undefined && !this.replying;
  }

  setReplyBody(value: string): void {
    this.replyBody = value;
  }

  setSelectedCannedId(id: number | null): void {
    this.selectedCannedId = id;
  }

  /** Select (or replace) the own-file attachment, running the shared pre-check. */
  setReplyFile(file: File): void {
    this.replyFile = file;
    this.replyFileError = validateAttachment(file);
  }

  clearReplyFile(): void {
    this.replyFile = null;
    this.replyFileError = undefined;
  }

  /** GET .../canned — the enabled, dept-scoped responses for this ticket. */
  async loadCanned(id: number): Promise<void> {
    this.loadingCanned = true;
    try {
      const list = await this.api.get<CannedSummary[]>(`/api/staff/tickets/${id}/canned`);
      runInAction(() => {
        this.cannedList = list;
      });
    } catch (e) {
      runInAction(() => {
        // A canned-list failure is non-fatal: the plain reply still works.
        this.replyError = messageOf(e);
      });
    } finally {
      runInAction(() => {
        this.loadingCanned = false;
      });
    }
  }

  /**
   * GET .../canned/:cannedId — fetch the substituted body + carried attachments,
   * FILL the reply textarea, surface the carried chips read-only, and RETAIN the
   * cannedId so the reply POST lets the backend re-render + bind by file id.
   */
  async selectCanned(id: number, cannedId: number): Promise<void> {
    try {
      const detail = await this.api.get<CannedDetail>(
        `/api/staff/tickets/${id}/canned/${cannedId}`,
      );
      runInAction(() => {
        this.replyBody = detail.body;
        this.cannedAttachments = detail.attachments ?? [];
        this.selectedCannedId = cannedId;
      });
    } catch (e) {
      runInAction(() => {
        this.replyError = messageOf(e);
      });
    }
  }

  /** Clear the canned selection (body left as-is; chips + id dropped). */
  clearCanned(): void {
    this.selectedCannedId = null;
    this.cannedAttachments = [];
  }

  /**
   * GET /api/staff/tickets/stats — quick counts per queue for tab badges.
   * @implements FS-020.2: queue tabs with counts (TS-M3-A4)
   */
  async loadStats(): Promise<void> {
    this.loadingStats = true;
    this.statsError = null;
    try {
      const stats = await this.api.get<QueueStats>("/api/staff/tickets/stats");
      runInAction(() => {
        this.stats = stats;
      });
    } catch (e) {
      runInAction(() => {
        this.statsError = messageOf(e);
      });
    } finally {
      runInAction(() => {
        this.loadingStats = false;
      });
    }
  }

  /**
   * GET /api/staff/tickets/sort-prefs — sticky sort preferences per queue.
   * @implements BS-020.8: sticky sort (TS-M3-B4)
   */
  async loadSortPrefs(): Promise<void> {
    this.loadingSortPrefs = true;
    try {
      const prefs = await this.api.get<SortPrefs>("/api/staff/tickets/sort-prefs");
      runInAction(() => {
        this.sortPrefs = prefs;
        this.sortPrefsLoaded = true;
      });
    } catch {
      // Sort prefs are non-critical; use defaults if unavailable.
      runInAction(() => {
        this.sortPrefsLoaded = true;
      });
    } finally {
      runInAction(() => {
        this.loadingSortPrefs = false;
      });
    }
  }

  /**
   * Ensure sortPrefs are loaded before returning. Used to avoid race conditions
   * when the queue load depends on sticky sort preferences.
   * @implements BS-020.8: sticky sort (TS-M3-B4)
   */
  async ensureSortPrefsLoaded(): Promise<void> {
    if (this.sortPrefsLoaded) return;
    if (this.loadingSortPrefs) {
      // Wait for in-flight request to complete
      await new Promise<void>((resolve) => {
        const check = () => {
          if (this.sortPrefsLoaded) {
            resolve();
          } else {
            setTimeout(check, 10);
          }
        };
        check();
      });
      return;
    }
    await this.loadSortPrefs();
  }

  /**
   * GET /api/staff/tickets — the queue with optional filters, sort, and pagination.
   * @implements FS-020.2: queue tabs (TS-M3-A4)
   * @implements FS-020.6: pagination (TS-M3-B4)
   * @implements BS-020.8: sticky sort (TS-M3-B4)
   */
  async loadQueue(params: LoadQueueParams = {}): Promise<void> {
    this.loadingQueue = true;
    this.queueError = null;
    try {
      const searchParams = new URLSearchParams();
      if (params.status) searchParams.set("status", params.status);
      if (params.sort) searchParams.set("sort", params.sort);
      if (params.order) searchParams.set("order", params.order);
      if (params.page) searchParams.set("p", String(params.page));
      if (params.limit) searchParams.set("limit", String(params.limit));

      const query = searchParams.toString();
      const url = `/api/staff/tickets${query ? `?${query}` : ""}`;
      const response = await this.api.get<QueueResponse | QueueItem[]>(url);

      runInAction(() => {
        // Handle both M1 (plain array) and M3 (wrapped response) shapes.
        if (Array.isArray(response)) {
          this.queue = response;
          this.rightmostColumn = "department";
          this.pagination = null;
        } else {
          this.queue = response.tickets;
          this.rightmostColumn = response.rightmostColumn;
          this.pagination = response.pagination;
        }
        // Track current sort for UI indicators.
        if (params.sort) {
          this.currentSort = params.sort;
          this.currentOrder = params.order ?? "DESC";
        }
      });

      // Optionally refresh stats to keep tab badges in sync.
      if (params.refreshStats) {
        void this.loadStats();
      }
    } catch (e) {
      runInAction(() => {
        this.queueError = messageOf(e);
      });
    } finally {
      runInAction(() => {
        this.loadingQueue = false;
      });
    }
  }

  /** GET /api/staff/tickets/:id — the full ordered thread (all M/R/N). */
  async loadDetail(id: number): Promise<void> {
    this.loadingDetail = true;
    this.detailError = null;
    try {
      const detail = await this.api.get<TicketDetail>(`/api/staff/tickets/${id}`);
      runInAction(() => {
        this.detail = detail;
      });
    } catch (e) {
      runInAction(() => {
        this.detailError = messageOf(e);
      });
    } finally {
      runInAction(() => {
        this.loadingDetail = false;
      });
    }
  }

  /**
   * POST a reply (driven by composer state), then replace the thread with the
   * reply RESPONSE (the backend returns the refreshed detail incl. isanswered).
   * No optimistic local append. Switches to multipart/form-data when a canned
   * response is selected OR an own-file is attached (§2, §10); otherwise the M1
   * JSON `{ body }` path is preserved unchanged. The composer is reset on success.
   */
  async reply(id: number): Promise<void> {
    if (this.replyFileError !== undefined) return;
    this.replying = true;
    this.replyError = null;
    try {
      const detail = await this.api.post<TicketDetail>(
        `/api/staff/tickets/${id}/reply`,
        this.buildReplyBody(),
      );
      runInAction(() => {
        this.detail = detail;
        this.resetComposer();
      });
    } catch (e) {
      runInAction(() => {
        this.replyError = messageOf(e);
      });
      throw e;
    } finally {
      runInAction(() => {
        this.replying = false;
      });
    }
  }

  /**
   * Build the reply request body. Plain reply (no canned, no file) → the M1 JSON
   * `{ body }`. Otherwise → multipart with `body` + optional `cannedId` (retained
   * so the backend re-renders + binds canned attachments by id) + optional own
   * `attachment` part.
   */
  private buildReplyBody(): FormData | { body: string } {
    const body = this.replyBody.trim();
    if (this.selectedCannedId === null && !this.replyFile) {
      return { body };
    }
    const form = new FormData();
    form.append("body", body);
    if (this.selectedCannedId !== null) form.append("cannedId", String(this.selectedCannedId));
    // 2-arg append: the File carries its own name (3-arg hangs undici under jsdom).
    if (this.replyFile) form.append("attachment", this.replyFile);
    return form;
  }

  /** Reset the reply composer after a successful post. */
  private resetComposer(): void {
    this.replyBody = "";
    this.selectedCannedId = null;
    this.cannedAttachments = [];
    this.replyFile = null;
    this.replyFileError = undefined;
  }

  clearDetail(): void {
    this.detail = null;
    this.detailError = null;
    this.replyError = null;
    this.resetComposer();
    this.cannedList = [];
    this.clearWorkflowState();
    this.clearNoteComposer();
  }

  // --- Workflow Actions (TS-M3-C4) ---

  /** Clear workflow state. */
  clearWorkflowState(): void {
    this.workflowLoading = false;
    this.workflowError = null;
    this.workflowMessage = null;
  }

  /**
   * POST /api/staff/tickets/{id}/claim - self-assign a ticket.
   * @implements TS-M3-C4 AC-4: clicking Claim calls API and shows success message.
   */
  async claimTicket(id: number): Promise<boolean> {
    this.workflowLoading = true;
    this.workflowError = null;
    this.workflowMessage = null;
    try {
      const result = await this.api.post<WorkflowSuccess>(
        `/api/staff/tickets/${id}/claim`,
        {},
      );
      runInAction(() => {
        this.workflowMessage = result.message;
        // Refresh detail to show updated assignment.
        void this.loadDetail(id);
      });
      return true;
    } catch (e) {
      runInAction(() => {
        this.workflowError = messageOf(e);
      });
      return false;
    } finally {
      runInAction(() => {
        this.workflowLoading = false;
      });
    }
  }

  /**
   * POST /api/staff/tickets/{id}/assign - assign to a staff member or team.
   * @implements TS-M3-C4 AC-5/6/7: assign dialog with validation.
   */
  async assignTicket(
    id: number,
    assignee: string,
    comments: string,
  ): Promise<boolean> {
    this.workflowLoading = true;
    this.workflowError = null;
    this.workflowMessage = null;
    try {
      const result = await this.api.post<WorkflowSuccess>(
        `/api/staff/tickets/${id}/assign`,
        { assignee, comments },
      );
      runInAction(() => {
        this.workflowMessage = result.message;
      });
      return true;
    } catch (e) {
      runInAction(() => {
        this.workflowError = messageOf(e);
      });
      return false;
    } finally {
      runInAction(() => {
        this.workflowLoading = false;
      });
    }
  }

  /**
   * POST /api/staff/tickets/{id}/transfer - transfer to a different department.
   * @implements TS-M3-C4 AC-8/9/10: transfer dialog with validation.
   */
  async transferTicket(
    id: number,
    deptId: number,
    comments: string,
  ): Promise<{ success: boolean; accessLost: boolean }> {
    this.workflowLoading = true;
    this.workflowError = null;
    this.workflowMessage = null;
    try {
      const result = await this.api.post<TransferSuccess>(
        `/api/staff/tickets/${id}/transfer`,
        { dept_id: deptId, comments },
      );
      runInAction(() => {
        this.workflowMessage = result.message;
        if (!result.accessLost) {
          // Refresh detail to show updated department.
          void this.loadDetail(id);
        }
      });
      return { success: true, accessLost: result.accessLost };
    } catch (e) {
      runInAction(() => {
        this.workflowError = messageOf(e);
      });
      return { success: false, accessLost: false };
    } finally {
      runInAction(() => {
        this.workflowLoading = false;
      });
    }
  }

  /**
   * POST /api/staff/tickets/{id}/close - close a ticket.
   * @implements TS-M3-C4 AC-11/12/13: close button/dialog.
   */
  async closeTicket(id: number, comments?: string): Promise<boolean> {
    this.workflowLoading = true;
    this.workflowError = null;
    this.workflowMessage = null;
    try {
      const result = await this.api.post<WorkflowSuccess>(
        `/api/staff/tickets/${id}/close`,
        comments ? { comments } : {},
      );
      runInAction(() => {
        this.workflowMessage = result.message;
      });
      return true;
    } catch (e) {
      runInAction(() => {
        this.workflowError = messageOf(e);
      });
      return false;
    } finally {
      runInAction(() => {
        this.workflowLoading = false;
      });
    }
  }

  /**
   * POST /api/staff/tickets/{id}/reopen - reopen a closed ticket.
   * @implements TS-M3-C4 AC-14/15/16: reopen button/dialog.
   */
  async reopenTicket(id: number, comments?: string): Promise<boolean> {
    this.workflowLoading = true;
    this.workflowError = null;
    this.workflowMessage = null;
    try {
      const result = await this.api.post<WorkflowSuccess>(
        `/api/staff/tickets/${id}/reopen`,
        comments ? { comments } : {},
      );
      runInAction(() => {
        this.workflowMessage = result.message;
        // Refresh detail to show updated status.
        void this.loadDetail(id);
      });
      return true;
    } catch (e) {
      runInAction(() => {
        this.workflowError = messageOf(e);
      });
      return false;
    } finally {
      runInAction(() => {
        this.workflowLoading = false;
      });
    }
  }

  // --- Note Actions (TS-M3-E3) ---

  /** Set the note body. */
  setNoteBody(value: string): void {
    this.noteBody = value;
  }

  /** Set the note title. */
  setNoteTitle(value: string): void {
    this.noteTitle = value;
  }

  /** Set the note state change option. */
  setNoteState(value: NoteState): void {
    this.noteState = value;
  }

  /** Clear the note composer. */
  clearNoteComposer(): void {
    this.noteBody = "";
    this.noteTitle = "";
    this.noteState = "unchanged";
    this.noteError = null;
  }

  /** True when a non-empty note can be sent. */
  get canSendNote(): boolean {
    return this.noteBody.trim() !== "" && !this.noteSending;
  }

  /**
   * POST /api/staff/tickets/{id}/note - post an internal note.
   * @implements TS-M3-E3 AC-1/2/3/4/5/6: note form with optional state change.
   */
  async postNote(id: number): Promise<{ success: boolean; stateChanged: boolean }> {
    if (!this.canSendNote) return { success: false, stateChanged: false };

    this.noteSending = true;
    this.noteError = null;
    try {
      const result = await this.api.post<NoteResponse>(
        `/api/staff/tickets/${id}/note`,
        {
          body: this.noteBody.trim(),
          title: this.noteTitle.trim() || undefined,
          state: this.noteState === "unchanged" ? undefined : this.noteState,
        },
      );
      runInAction(() => {
        this.clearNoteComposer();
        // Refresh detail to show the new note in thread.
        void this.loadDetail(id);
      });
      return { success: true, stateChanged: result.stateChanged };
    } catch (e) {
      runInAction(() => {
        this.noteError = messageOf(e);
      });
      return { success: false, stateChanged: false };
    } finally {
      runInAction(() => {
        this.noteSending = false;
      });
    }
  }

  // --- Search State (TS-M3-F3) ---

  /** True when viewing search results. */
  isSearchMode = false;
  /** Search query string. */
  searchQuery = "";
  /** Search error (e.g., "must be > 3 chars"). */
  searchError: string | null = null;
  /** True when Status column should be shown instead of Priority (search results with no status filter). */
  showStatusColumn = false;

  /** Advanced search filters. */
  advancedFilters: {
    status?: string;
    deptId?: number;
    assigneeId?: number;
    topicId?: number;
    startDate?: string;
    endDate?: string;
  } = {};

  /**
   * Set search mode and query.
   * @implements FS-020.7: search box (TS-M3-F3)
   */
  setSearchMode(isSearch: boolean, query = ""): void {
    this.isSearchMode = isSearch;
    this.searchQuery = query;
    if (!isSearch) {
      this.searchError = null;
      this.showStatusColumn = false;
      this.advancedFilters = {};
    }
  }

  /**
   * Validate search query (must be > 3 chars).
   * @implements FS-020.8: search validation (TS-M3-F3)
   */
  validateSearchQuery(query: string): boolean {
    if (query.trim().length < 3) {
      this.searchError = "Search term must be more than 3 chars";
      return false;
    }
    this.searchError = null;
    return true;
  }

  /**
   * Set advanced filters.
   */
  setAdvancedFilters(filters: typeof this.advancedFilters): void {
    this.advancedFilters = filters;
  }

  /**
   * Load search results.
   * @implements FS-020.7-8: search endpoint (TS-M3-F3)
   */
  async loadSearchResults(params: {
    query?: string;
    status?: string;
    deptId?: number;
    assigneeId?: number;
    topicId?: number;
    startDate?: string;
    endDate?: string;
    page?: number;
    limit?: number;
    sort?: SortKey;
    order?: SortOrder;
  }): Promise<void> {
    this.loadingQueue = true;
    this.queueError = null;
    try {
      const searchParams = new URLSearchParams();
      searchParams.set("a", "search");
      if (params.query) searchParams.set("query", params.query);
      if (params.status && params.status !== "any") searchParams.set("status", params.status);
      if (params.deptId) searchParams.set("dept_id", String(params.deptId));
      if (params.assigneeId) searchParams.set("assignee_id", String(params.assigneeId));
      if (params.topicId) searchParams.set("topic_id", String(params.topicId));
      if (params.startDate) searchParams.set("start_date", params.startDate);
      if (params.endDate) searchParams.set("end_date", params.endDate);
      if (params.page) searchParams.set("p", String(params.page));
      if (params.limit) searchParams.set("limit", String(params.limit));
      if (params.sort) searchParams.set("sort", params.sort);
      if (params.order) searchParams.set("order", params.order);

      const url = `/api/staff/tickets?${searchParams.toString()}`;
      const response = await this.api.get<{
        tickets: QueueItem[];
        rightmostColumn: RightmostColumn;
        pagination: PaginationMeta;
        statusColumn?: boolean;
      }>(url);

      runInAction(() => {
        this.queue = response.tickets;
        this.rightmostColumn = response.rightmostColumn;
        this.pagination = response.pagination;
        this.isSearchMode = true;
        this.searchQuery = params.query ?? "";
        this.showStatusColumn = response.statusColumn ?? false;
        if (params.sort) {
          this.currentSort = params.sort;
          this.currentOrder = params.order ?? "DESC";
        }
      });
    } catch (e) {
      runInAction(() => {
        this.queueError = messageOf(e);
      });
    } finally {
      runInAction(() => {
        this.loadingQueue = false;
      });
    }
  }

  // --- Bulk Selection State (TS-M3-G2) ---

  /** Selected ticket IDs for bulk actions. */
  selectedTicketIds: Set<number> = new Set();
  /** Loading state for bulk actions. */
  bulkLoading = false;
  /** Error from bulk action. */
  bulkError: string | null = null;
  /** Success message from bulk action. */
  bulkMessage: string | null = null;

  /** Toggle selection of a single ticket. */
  toggleTicketSelection(id: number): void {
    if (this.selectedTicketIds.has(id)) {
      this.selectedTicketIds.delete(id);
    } else {
      this.selectedTicketIds.add(id);
    }
    // Trigger reactivity.
    this.selectedTicketIds = new Set(this.selectedTicketIds);
  }

  /** Select all tickets in the current queue. */
  selectAllTickets(): void {
    this.selectedTicketIds = new Set(this.queue.map((t) => t.id));
  }

  /** Deselect all tickets. */
  deselectAllTickets(): void {
    this.selectedTicketIds = new Set();
  }

  /** True when all current queue items are selected. */
  get allSelected(): boolean {
    return this.queue.length > 0 && this.queue.every((t) => this.selectedTicketIds.has(t.id));
  }

  /** True when some but not all tickets are selected. */
  get someSelected(): boolean {
    return this.selectedTicketIds.size > 0 && !this.allSelected;
  }

  /**
   * Execute bulk action.
   * @implements BS-020.9: bulk action (TS-M3-G2)
   */
  async bulkAction(
    action: "close" | "reopen" | "delete",
    ticketIds: number[],
  ): Promise<{ success: boolean; affected: number; total: number }> {
    this.bulkLoading = true;
    this.bulkError = null;
    this.bulkMessage = null;
    try {
      // The backend mass-process endpoint returns `{ succeeded, failed, message }`
      // (backend/api/src/staff.rs::BulkActionResponse). Map `succeeded` → affected
      // and `succeeded + failed` → total so the toast interpolates the real count
      // (never "undefined tickets closed"). US-M3-G1 AC-4.
      const result = await this.api.post<{
        succeeded: number;
        failed: number;
        message: string;
      }>("/api/staff/tickets/bulk", { action, ticket_ids: ticketIds });
      const affected = result.succeeded;
      const total = result.succeeded + result.failed;
      runInAction(() => {
        this.bulkMessage = result.message;
        this.deselectAllTickets();
      });
      return { success: true, affected, total };
    } catch (e) {
      runInAction(() => {
        this.bulkError = messageOf(e);
      });
      return { success: false, affected: 0, total: ticketIds.length };
    } finally {
      runInAction(() => {
        this.bulkLoading = false;
      });
    }
  }

  // --- Edit Form State (TS-M3-I4) ---

  /** Loading state for update. */
  updateLoading = false;
  /** Error from update. */
  updateError: string | null = null;
  /** Success message from update. */
  updateMessage: string | null = null;

  /**
   * Update ticket.
   * @implements FS-021.15: edit form (TS-M3-I4)
   */
  async updateTicket(
    id: number,
    updates: {
      name?: string;
      email?: string;
      phone?: string;
      phone_ext?: string;
      dept_id?: number;
      topic_id?: number;
      priority_id?: number;
      sla_id?: number;
      source?: string;
      duedate?: string;
      reason: string;
    },
  ): Promise<boolean> {
    this.updateLoading = true;
    this.updateError = null;
    this.updateMessage = null;
    try {
      const result = await this.api.put<{ success: boolean; message: string }>(
        `/api/staff/tickets/${id}`,
        updates,
      );
      runInAction(() => {
        this.updateMessage = result.message;
        // Refresh detail to show updated values.
        void this.loadDetail(id);
      });
      return true;
    } catch (e) {
      runInAction(() => {
        this.updateError = messageOf(e);
      });
      return false;
    } finally {
      runInAction(() => {
        this.updateLoading = false;
      });
    }
  }

  // --- Delete State (TS-M3-I5) ---

  /** Loading state for delete. */
  deleteLoading = false;
  /** Error from delete. */
  deleteError: string | null = null;

  /**
   * Delete ticket.
   * @implements FS-021.19: delete (TS-M3-I5)
   */
  async deleteTicket(id: number): Promise<boolean> {
    this.deleteLoading = true;
    this.deleteError = null;
    try {
      await this.api.delete(`/api/staff/tickets/${id}`);
      return true;
    } catch (e) {
      runInAction(() => {
        this.deleteError = messageOf(e);
      });
      return false;
    } finally {
      runInAction(() => {
        this.deleteLoading = false;
      });
    }
  }

  // --- Lock State (TS-M3-I6) ---

  /** Current lock ID if we hold the lock. */
  lockId: number | null = null;
  /** Staff name who holds the lock if not us. */
  lockedByName: string | null = null;
  /** True if another agent holds the lock. */
  lockedByAnother = false;
  /** Lock loading state. */
  lockLoading = false;
  /** Lock error message. */
  lockError: string | null = null;
  /** Lock renewal interval ID. */
  private lockRenewInterval: ReturnType<typeof setInterval> | null = null;

  /**
   * Acquire (or idempotently renew) the lock on a ticket, per the shared lock
   * contract (US-M3-I2 / FS-021.18, FS-021.20; BS-021.3).
   *
   * The lock is keyed by (ticket_id + staff_id), so the SAME staff re-opening the
   * ticket (a second tab / session) legitimately RENEWS their own lock — the backend
   * returns success (a lock id), and we must NOT surface any warning or the generic
   * "Unable to obtain a lock" error. Only a DIFFERENT staff holding a live lock is a
   * genuine conflict → the named banner + a disabled composer.
   *
   * Locked-by-another may be signalled two ways (the backend half is landing in
   * parallel); both are handled: (a) on the 200 body as a `locked_by_other` flag
   * (nested under `lock` or flat) carrying `locked_by_name`, or (b) as a 409/423
   * conflict whose message/fields carry the holder name.
   *
   * @implements FS-021.18: lock acquire/renew (TS-M3-I6)
   * @implements FS-021.20: named lock banner ("locked by <name>")
   * @implements BS-021.3: one active lock per ticket; same-staff renew is idempotent
   */
  async acquireLock(ticketId: number): Promise<boolean> {
    this.lockLoading = true;
    this.lockError = null;
    this.lockedByAnother = false;
    this.lockedByName = null;
    try {
      const result = await this.api.post<LockAcquireResponse>(
        `/api/staff/tickets/${ticketId}/lock`,
        {},
      );

      // Locked by ANOTHER staff, signalled on the (200) success body.
      const otherHolder = lockedByOtherFrom(result);
      if (otherHolder !== null) {
        runInAction(() => {
          this.lockedByAnother = true;
          this.lockedByName = otherHolder;
        });
        return false;
      }

      // Fresh acquire OR idempotent same-staff renewal — we hold the lock.
      const acquiredId = lockIdFrom(result);
      if (acquiredId !== null) {
        runInAction(() => {
          this.lockId = acquiredId;
          // Start renewal polling (every 60 seconds).
          this.startLockRenewal(ticketId);
        });
        return true;
      }

      // Success with neither a conflict flag nor a lock id: nothing to hold, but
      // this is NOT a failure — do not surface the generic error banner.
      return false;
    } catch (e) {
      // A locked-by-another conflict may instead arrive as a 409/423 error. Treat
      // it as the named banner, NOT the generic "Unable to obtain a lock" message.
      const conflictHolder = lockConflictHolder(e);
      if (conflictHolder !== null) {
        runInAction(() => {
          this.lockedByAnother = true;
          this.lockedByName = conflictHolder;
        });
        return false;
      }
      runInAction(() => {
        this.lockError = "Unable to obtain a lock on the ticket";
      });
      return false;
    } finally {
      runInAction(() => {
        this.lockLoading = false;
      });
    }
  }

  /**
   * Start lock renewal polling.
   */
  private startLockRenewal(ticketId: number): void {
    this.stopLockRenewal();
    this.lockRenewInterval = setInterval(() => {
      void this.renewLock(ticketId);
    }, 60000);
  }

  /**
   * Stop lock renewal polling.
   */
  private stopLockRenewal(): void {
    if (this.lockRenewInterval) {
      clearInterval(this.lockRenewInterval);
      this.lockRenewInterval = null;
    }
  }

  /**
   * Renew lock on ticket.
   */
  async renewLock(ticketId: number): Promise<void> {
    if (!this.lockId) return;
    try {
      await this.api.post(`/api/staff/tickets/${ticketId}/lock`, {});
    } catch {
      // Lock renewal failure is non-fatal.
    }
  }

  /**
   * Release lock on ticket.
   * @implements FS-021.18: lock release (TS-M3-I6)
   */
  async releaseLock(ticketId: number): Promise<void> {
    this.stopLockRenewal();
    if (!this.lockId) return;
    try {
      await this.api.delete(`/api/staff/tickets/${ticketId}/lock`);
    } catch {
      // Lock release failure is non-fatal.
    } finally {
      runInAction(() => {
        this.lockId = null;
        this.lockedByAnother = false;
        this.lockedByName = null;
      });
    }
  }

  /**
   * Clear lock state without releasing (for component unmount cleanup).
   */
  clearLockState(): void {
    this.stopLockRenewal();
    this.lockId = null;
    this.lockedByAnother = false;
    this.lockedByName = null;
    this.lockError = null;
  }
}

function messageOf(e: unknown): string {
  if (e instanceof ApiError) return e.message;
  return e instanceof Error ? e.message : "Something went wrong.";
}
