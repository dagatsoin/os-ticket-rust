// Staff queue + ticket-detail/reply store (TS-M1-C4, TDD target).
//
// Loads the open-tickets queue (served newest-first), a single ticket detail
// (full M/R/N thread, created ASC), and posts a reply that REFRESHES the thread
// from the reply RESPONSE (the backend returns the updated detail) — no
// optimistic local append.
//
// @implements BS-020: open-tickets queue (number/subject/email/created).
// @implements BS-021: ticket detail (ordered full thread) + reply-then-refresh.
import { makeAutoObservable, runInAction } from "mobx";
import { ApiClient } from "../api/apiClient";
import { ApiError } from "../api/types";

/** One row of the staff open-tickets queue (GET /api/staff/tickets). */
export interface QueueItem {
  id: number;
  number: number;
  subject: string;
  email: string;
  created: string;
}

/** One thread entry in a staff ticket detail. */
export interface StaffThreadEntry {
  id: number;
  /** `M` (client message) / `R` (staff response) / `N` (internal note). */
  threadType: "M" | "R" | "N";
  poster: string;
  body: string;
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

  constructor(api: ApiClient) {
    this.api = api;
    makeAutoObservable(this, {}, { autoBind: true });
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
   * POST a reply, then replace the thread with the reply RESPONSE (the backend
   * returns the refreshed detail). No optimistic local append.
   */
  async reply(id: number, body: string): Promise<void> {
    this.replying = true;
    this.replyError = null;
    try {
      const detail = await this.api.post<TicketDetail>(
        `/api/staff/tickets/${id}/reply`,
        { body },
      );
      runInAction(() => {
        this.detail = detail;
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

  clearDetail(): void {
    this.detail = null;
    this.detailError = null;
    this.replyError = null;
  }
}

function messageOf(e: unknown): string {
  if (e instanceof ApiError) return e.message;
  return e instanceof Error ? e.message : "Something went wrong.";
}
