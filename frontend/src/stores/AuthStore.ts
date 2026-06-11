// Per-realm MobX auth store. Two independent instances (staff, client) live on the
// RootStore and NEVER share state (separate cookies, separate apiClient, separate user).
// Skeleton: the concrete login/logout endpoints are finalised by TS-M1-C1 (staff) and
// TS-M1-D1 (client); the store shape and observable contract are fixed here.
import { makeAutoObservable, runInAction } from "mobx";
import { ApiClient } from "../api/apiClient";
import { ApiError, type Realm } from "../api/types";

/** Minimal authenticated-user shape; realms refine it as their routes land. */
export interface AuthUser {
  id: number;
  [key: string]: unknown;
}

/** Realm-specific endpoint config so the same store serves both realms. */
export interface AuthEndpoints {
  login: string;
  logout: string;
  /**
   * Optional profile endpoint (e.g. GET /api/staff/me). When set, the store
   * fetches the profile after a successful login and treats THAT as the
   * authenticated user — the login response itself carries no profile (only
   * `{ ok, csrfToken }`). Also used by `loadProfile()` for session restore.
   */
  profile?: string;
}

export class AuthStore {
  readonly realm: Realm;
  private readonly api: ApiClient;
  private readonly endpoints: AuthEndpoints;

  user: AuthUser | null = null;
  loading = false;
  error: ApiError | null = null;

  constructor(realm: Realm, api: ApiClient, endpoints: AuthEndpoints) {
    this.realm = realm;
    this.api = api;
    this.endpoints = endpoints;
    makeAutoObservable(this, { realm: false }, { autoBind: true });
  }

  get isAuthenticated(): boolean {
    return this.user !== null;
  }

  /**
   * POST credentials to the realm login endpoint; on success store the user.
   * When a `profile` endpoint is configured, the authenticated user is the
   * profile fetched from it (the login response carries only `{ ok, csrfToken }`),
   * never a token.
   */
  async login(credentials: Record<string, unknown>): Promise<void> {
    this.loading = true;
    this.error = null;
    try {
      const loginResult = await this.api.post<AuthUser>(this.endpoints.login, credentials);
      const user = this.endpoints.profile
        ? await this.api.get<AuthUser>(this.endpoints.profile)
        : loginResult;
      runInAction(() => {
        this.user = user;
      });
    } catch (e) {
      runInAction(() => {
        this.error = e instanceof ApiError ? e : new ApiError(0, String(e));
      });
      throw e;
    } finally {
      runInAction(() => {
        this.loading = false;
      });
    }
  }

  /**
   * Best-effort session restore: fetch the profile endpoint and, on success,
   * mark the realm authenticated. A failure (e.g. no session) leaves the store
   * unauthenticated. No-op when no profile endpoint is configured.
   */
  async loadProfile(): Promise<void> {
    if (!this.endpoints.profile) return;
    try {
      const user = await this.api.get<AuthUser>(this.endpoints.profile);
      runInAction(() => {
        this.user = user;
      });
    } catch {
      runInAction(() => {
        this.user = null;
      });
    }
  }

  /** Clear local auth state; best-effort server logout. */
  async logout(): Promise<void> {
    try {
      await this.api.post(this.endpoints.logout, {});
    } finally {
      runInAction(() => {
        this.user = null;
        this.error = null;
      });
    }
  }
}
