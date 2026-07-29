// Client portal session + read-only thread store (TS-M1-D2, TDD target).
//
// The client realm is account-less and token-less: POST /api/client/login binds
// a cookie session to ONE ticket (by number + email), and GET /api/client/ticket
// returns that ticket's thread — `M` (client message) + `R` (staff response)
// ONLY (the backend never exposes internal `N` notes). The store holds NO token.
//
// @implements BS-010: a client reads only its own (session-bound) ticket.
// @implements FS-010 (security): client thread is M+R only, never N.
import { makeAutoObservable, runInAction } from "mobx";
import { ApiClient } from "../api/apiClient";
import { ApiError } from "../api/types";
import type { Attachment } from "../utils/downloadAttachment";

/** One thread entry returned to a client (only `M` / `R` ever reach here). */
export interface ClientThreadEntry {
  id: number;
  /** `M` (client message) or `R` (staff response) — never `N`. */
  threadType: "M" | "R";
  poster: string;
  body: string;
  /** When this entry was created — RFC3339 (`YYYY-MM-DDThh:mm:ssZ`). */
  created: string;
  /** Attachments carried on this entry (§7); empty/absent when none. */
  attachments?: Attachment[];
}

/** The client's own ticket plus its (M/R-only) thread. */
export interface ClientTicket {
  number: number;
  subject: string;
  status: string;
  created: string;
  entries: ClientThreadEntry[];
}

export class ClientPortalStore {
  private readonly api: ApiClient;

  ticket: ClientTicket | null = null;
  loading = false;
  error: string | null = null;

  constructor(api: ApiClient) {
    this.api = api;
    makeAutoObservable(this, {}, { autoBind: true });
  }

  get isAuthenticated(): boolean {
    return this.ticket !== null;
  }

  /**
   * POST ticket#/email to establish the client session, then load the bound
   * ticket thread. A single generic error is surfaced on failure (the backend
   * gives no field-level oracle).
   */
  async login(credentials: { ticketNumber: string; email: string }): Promise<void> {
    this.loading = true;
    this.error = null;
    try {
      await this.api.post("/api/client/login", credentials);
      const ticket = await this.api.get<ClientTicket>("/api/client/ticket");
      runInAction(() => {
        this.ticket = ticket;
      });
    } catch (e) {
      runInAction(() => {
        this.ticket = null;
        this.error = e instanceof ApiError ? e.message : "Something went wrong.";
      });
      throw e;
    } finally {
      runInAction(() => {
        this.loading = false;
      });
    }
  }

  /** Load the session-bound ticket on its own (e.g. session restore / refresh). */
  async loadTicket(): Promise<void> {
    this.loading = true;
    this.error = null;
    try {
      const ticket = await this.api.get<ClientTicket>("/api/client/ticket");
      runInAction(() => {
        this.ticket = ticket;
      });
    } catch (e) {
      runInAction(() => {
        this.error = e instanceof ApiError ? e.message : "Something went wrong.";
      });
    } finally {
      runInAction(() => {
        this.loading = false;
      });
    }
  }
}
