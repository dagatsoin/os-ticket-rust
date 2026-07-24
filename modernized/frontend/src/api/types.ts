// Shared API contract types.
// The JSON error envelope is the contract owned by TS-M1-A1 (ROADMAP Decisions → 4):
//   { "error": { "message": "<top-level>", "fields": { "<field>": "<msg>" } } }

/** The two auth realms. Never share session/CSRF/state across realms. */
export type Realm = "staff" | "client";

/** Wire shape of the shared JSON error envelope. */
export interface ErrorEnvelope {
  error: {
    message: string;
    fields?: Record<string, string>;
  };
}

/**
 * Normalised, parsed API error thrown by the apiClient on a non-2xx response.
 * `message` is the top-level error; `fields` maps field name → message (422).
 */
export class ApiError extends Error {
  readonly status: number;
  readonly fields: Record<string, string>;

  constructor(status: number, message: string, fields: Record<string, string> = {}) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.fields = fields;
  }
}
