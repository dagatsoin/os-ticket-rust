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
// @implements FS-011.7: optional attachment — client pre-check + multipart submit + 422 mapping (TS-M2-A4).
import { makeAutoObservable, runInAction } from "mobx";
import { ApiClient } from "../api/apiClient";
import { ApiError } from "../api/types";
import { validateAttachment } from "../utils/validateAttachment";

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

  /**
   * Optional attachment (TS-M2-A4). When present, submit switches to multipart.
   * The confirmation chip label is the local `File.name` — no API echo (§11).
   */
  file: File | null = null;
  /** Client-side pre-check error OR a backend 422 `attachment` field message. */
  fileError: string | undefined = undefined;
  /** True once a backend 422 stamped fileError (so editing the file clears it). */
  private fileErrorFromServer = false;

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
    return this.isValid && this.fileError === undefined && !this.submitting;
  }

  /** Local filename for the confirmation chip (no API echo needed — §11). */
  get fileName(): string | null {
    return this.file?.name ?? null;
  }

  /**
   * Select (or replace) the attachment, running the shared client-side pre-check
   * (§14). A bad file is still stored so the input reflects the selection, but
   * `fileError` is set and gates submit.
   */
  setFile(file: File): void {
    this.file = file;
    this.fileErrorFromServer = false;
    this.fileError = validateAttachment(file);
  }

  /** Clear the selected attachment and any associated error. */
  clearFile(): void {
    this.file = null;
    this.fileError = undefined;
    this.fileErrorFromServer = false;
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
    // A failed client-side pre-check (bad type / too big) blocks submit.
    if (this.fileErrorFromServer) {
      this.fileError = undefined;
      this.fileErrorFromServer = false;
    }
    if (!this.isValid || this.fileError !== undefined) return;

    this.submitting = true;
    try {
      const res = await this.api.post<CreateTicketResponse>("/api/tickets", this.buildBody());
      runInAction(() => {
        this.ticketNumber = String(res.ticketNumber);
        this.submitted = true;
      });
    } catch (e) {
      runInAction(() => {
        if (e instanceof ApiError) {
          // The backend 422 `attachment` field error surfaces under the file input.
          const { attachment, ...rest } = e.fields;
          this.serverFieldErrors = rest;
          if (attachment) {
            this.fileError = attachment;
            this.fileErrorFromServer = true;
          }
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

  /**
   * Build the request body. With a file selected → `multipart/form-data` (each
   * field as its own part + an `attachment` file part), which the FormData-aware
   * apiClient sends raw (§2, §13). Without a file → the M1 JSON object (unchanged).
   */
  private buildBody(): FormData | Record<string, string> {
    const fields = {
      name: this.values.name.trim(),
      email: this.values.email.trim(),
      subject: this.values.subject.trim(),
      message: this.values.message.trim(),
    };
    if (!this.file) return fields;

    const form = new FormData();
    for (const [key, value] of Object.entries(fields)) form.append(key, value);
    // 2-arg append: the File already carries its own name. (A 3-arg append with
    // an explicit filename hangs undici's multipart serialize under jsdom.)
    form.append("attachment", this.file);
    return form;
  }
}
