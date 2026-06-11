// Shared JSON fetch wrapper. ONE per realm (staff / client) — realms never share state.
//
// Responsibilities (ROADMAP Decisions → 2, 4; TS-M1-A5):
//  - Inject the X-CSRFToken header from the realm XSRF cookie on MUTATING requests
//    (POST/PUT/PATCH/DELETE). Reads XSRF-TOKEN-STAFF or XSRF-TOKEN-CLIENT per realm.
//  - Parse the shared JSON error envelope into a normalised ApiError (top-level + fields).
//  - On 401, invoke onUnauthorized(loginPath) so the host can redirect to the realm login.
import { ApiError, type ErrorEnvelope, type Realm } from "./types";

const MUTATING = new Set(["POST", "PUT", "PATCH", "DELETE"]);

/** Per-realm XSRF cookie name (non-HttpOnly; seeded by the backend at login). */
const XSRF_COOKIE: Record<Realm, string> = {
  staff: "XSRF-TOKEN-STAFF",
  client: "XSRF-TOKEN-CLIENT",
};

/** Per-realm login route the apiClient redirects to on a 401. */
export const LOGIN_PATH: Record<Realm, string> = {
  staff: "/staff/login",
  client: "/tickets/login",
};

export interface ApiClientOptions {
  realm: Realm;
  /** Called with the realm login path on a 401. Defaults to a window.location redirect. */
  onUnauthorized?: (loginPath: string) => void;
}

interface RequestOptions {
  /** Extra headers merged onto the defaults. */
  headers?: Record<string, string>;
  signal?: AbortSignal;
}

/** Read a cookie value by name (browser document.cookie). */
function readCookie(name: string): string | undefined {
  const prefix = `${name}=`;
  for (const part of document.cookie.split(";")) {
    const c = part.trim();
    if (c.startsWith(prefix)) return decodeURIComponent(c.slice(prefix.length));
  }
  return undefined;
}

export class ApiClient {
  private readonly realm: Realm;
  private readonly onUnauthorized: (loginPath: string) => void;

  constructor(opts: ApiClientOptions) {
    this.realm = opts.realm;
    this.onUnauthorized =
      opts.onUnauthorized ??
      ((loginPath) => {
        window.location.assign(loginPath);
      });
  }

  get<T = unknown>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("GET", path, undefined, opts);
  }

  post<T = unknown>(path: string, body?: unknown, opts?: RequestOptions): Promise<T> {
    return this.request<T>("POST", path, body, opts);
  }

  put<T = unknown>(path: string, body?: unknown, opts?: RequestOptions): Promise<T> {
    return this.request<T>("PUT", path, body, opts);
  }

  patch<T = unknown>(path: string, body?: unknown, opts?: RequestOptions): Promise<T> {
    return this.request<T>("PATCH", path, body, opts);
  }

  delete<T = unknown>(path: string, opts?: RequestOptions): Promise<T> {
    return this.request<T>("DELETE", path, undefined, opts);
  }

  private async request<T>(
    method: string,
    path: string,
    body?: unknown,
    opts?: RequestOptions,
  ): Promise<T> {
    const headers: Record<string, string> = {
      Accept: "application/json",
      ...opts?.headers,
    };

    if (body !== undefined) headers["Content-Type"] = "application/json";

    // CSRF double-submit: reflect the realm XSRF cookie into X-CSRFToken on mutations.
    if (MUTATING.has(method)) {
      const token = readCookie(XSRF_COOKIE[this.realm]);
      if (token) headers["X-CSRFToken"] = token;
    }

    const res = await fetch(path, {
      method,
      headers,
      credentials: "include",
      signal: opts?.signal,
      body: body !== undefined ? JSON.stringify(body) : undefined,
    });

    if (res.status === 401) {
      this.onUnauthorized(LOGIN_PATH[this.realm]);
    }

    if (!res.ok) {
      throw await this.toApiError(res);
    }

    if (res.status === 204) return undefined as T;
    const text = await res.text();
    return (text ? JSON.parse(text) : undefined) as T;
  }

  /** Parse a non-2xx response body into a normalised ApiError. */
  private async toApiError(res: Response): Promise<ApiError> {
    let message = res.statusText || `Request failed (${res.status})`;
    let fields: Record<string, string> = {};
    try {
      const data = (await res.json()) as Partial<ErrorEnvelope>;
      if (data?.error) {
        if (typeof data.error.message === "string") message = data.error.message;
        if (data.error.fields) fields = data.error.fields;
      }
    } catch {
      // Non-JSON or empty body — keep the status-line fallback.
    }
    return new ApiError(res.status, message, fields);
  }
}
