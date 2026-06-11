// Public "Open a New Ticket" form store (TS-M1-B3, TDD target).
//
// Owns the M1 field set (name / email / subject / message), the client-side
// validation that MIRRORS the backend rules (required + email syntax), submit
// gating, and the 201->confirmation / 422->field-error mapping. The backend is
// authoritative: a 422 envelope's per-field messages override client-side text
// and are shown regardless of touched state.
//
// @implements BS-011: public open-a-ticket submission (name/email/subject/message).
// @implements FS-011.8: required-field + email validation at the web boundary.
import { makeAutoObservable, runInAction } from "mobx";
import { ApiClient } from "../api/apiClient";
import { ApiError } from "../api/types";

/** The M1 open-ticket field set (no help topic / CAPTCHA / attachments). */
export type TicketField = "name" | "email" | "subject" | "message";

export const TICKET_FIELDS: TicketField[] = ["name", "email", "subject", "message"];

/** Loose email check mirroring the backend's `is_email` (something@something.tld). */
const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

interface CreateTicketResponse {
  ticketNumber: number | string;
}

export class OpenTicketStore {
  private readonly api: ApiClient;

  values: Record<TicketField, string> = { name: "", email: "", subject: "", message: "" };
  /** Fields the user has interacted with (used to gate when errors surface). */
  touched: Record<TicketField, boolean> = {
    name: false,
    email: false,
    subject: false,
    message: false,
  };
  /** Per-field errors returned by the backend (422). Override client-side text. */
  serverFieldErrors: Record<string, string> = {};
  /** Top-level (envelope) error message from the backend. */
  topError: string | null = null;

  submitting = false;
  submitted = false;
  ticketNumber: string | null = null;

  constructor(api: ApiClient) {
    this.api = api;
    makeAutoObservable(this, {}, { autoBind: true });
  }

  /** Client-side validation message for one field, or undefined when valid. */
  private validate(field: TicketField): string | undefined {
    const value = this.values[field].trim();
    if (value === "") return "This field is required.";
    if (field === "email" && !EMAIL_RE.test(value)) return "Enter a valid email address.";
    return undefined;
  }

  /** True when every field passes client-side validation. */
  get isValid(): boolean {
    return TICKET_FIELDS.every((f) => this.validate(f) === undefined);
  }

  get canSubmit(): boolean {
    return this.isValid && !this.submitting;
  }

  /**
   * Effective per-field errors. A server (422) message always wins; otherwise a
   * client-side message is surfaced only once the field has been touched (or
   * after a submit attempt forces every field touched).
   */
  get fieldErrors(): Partial<Record<TicketField, string>> {
    const out: Partial<Record<TicketField, string>> = {};
    for (const field of TICKET_FIELDS) {
      const server = this.serverFieldErrors[field];
      if (server) {
        out[field] = server;
        continue;
      }
      if (this.touched[field]) {
        const msg = this.validate(field);
        if (msg) out[field] = msg;
      }
    }
    return out;
  }

  setValue(field: TicketField, value: string): void {
    this.values[field] = value;
    this.touched[field] = true;
    // Editing a field clears its stale server error.
    if (this.serverFieldErrors[field]) {
      const { [field]: _drop, ...rest } = this.serverFieldErrors;
      this.serverFieldErrors = rest;
    }
  }

  touch(field: TicketField): void {
    this.touched[field] = true;
  }

  /** Force every field touched so all client-side errors surface (on submit). */
  private touchAll(): void {
    this.touched = { name: true, email: true, subject: true, message: true };
  }

  /**
   * Validate, then POST to /api/tickets. On 201 → confirmation (ticketNumber).
   * On 422 → map the envelope's per-field errors onto the fields (no confirmation).
   */
  async submit(): Promise<void> {
    this.touchAll();
    this.serverFieldErrors = {};
    this.topError = null;
    if (!this.isValid) return;

    this.submitting = true;
    try {
      const res = await this.api.post<CreateTicketResponse>("/api/tickets", {
        name: this.values.name.trim(),
        email: this.values.email.trim(),
        subject: this.values.subject.trim(),
        message: this.values.message.trim(),
      });
      runInAction(() => {
        this.ticketNumber = String(res.ticketNumber);
        this.submitted = true;
      });
    } catch (e) {
      runInAction(() => {
        if (e instanceof ApiError) {
          this.serverFieldErrors = e.fields;
          this.topError = e.message;
        } else {
          this.topError = e instanceof Error ? e.message : "Something went wrong.";
        }
      });
    } finally {
      runInAction(() => {
        this.submitting = false;
      });
    }
  }
}
