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
import { makeAutoObservable, runInAction } from "mobx";
import { ApiClient } from "../api/apiClient";
import { ApiError } from "../api/types";
import type { Attachment } from "../utils/downloadAttachment";
import { validateAttachment } from "../utils/validateAttachment";

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
}

/** One thread entry in a staff ticket detail. */
export interface StaffThreadEntry {
  id: number;
  /** `M` (client message) / `R` (staff response) / `N` (internal note). */
  threadType: "M" | "R" | "N";
  poster: string;
  body: string;
  /** Attachments carried on this entry (§7); empty/absent when none. */
  attachments?: Attachment[];
}

/** A staff ticket detail (GET /api/staff/tickets/:id; also the reply response). */
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

  /** GET /api/staff/tickets — the open queue, served newest-first. */
  async loadQueue(): Promise<void> {
    this.loadingQueue = true;
    this.queueError = null;
    try {
      const items = await this.api.get<QueueItem[]>("/api/staff/tickets");
      runInAction(() => {
        this.queue = items;
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
  }
}

function messageOf(e: unknown): string {
  if (e instanceof ApiError) return e.message;
  return e instanceof Error ? e.message : "Something went wrong.";
}
