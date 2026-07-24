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

/**
 * Delegated staff capability flags surfaced by GET /api/staff/me (TS-M4-A1).
 * A staff member either has full `isadmin` or a subset of these delegated flags
 * that unlock individual admin screens (FAQ categories, canned responses, …).
 */
export type StaffCapability =
  | "can_manage_faq"
  | "can_manage_premade"
  | "can_ban_emails"
  | "can_view_staff_stats";

/** Tri-state of the profile fetch, so guards render a spinner instead of flashing a redirect. */
export type ProfileLoadState = "loading" | "loaded" | "error";

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

  /**
   * Tri-state of the profile fetch (TS-M4-A0). Starts "loading" so route guards
   * render a spinner rather than briefly redirecting before /me resolves. Set to
   * "loaded" on a successful profile/login, "error" when the fetch fails (e.g. no
   * session). A realm with no profile endpoint resolves straight to "loaded".
   */
  profileLoaded: ProfileLoadState = "loading";
  /** Guard against duplicate concurrent profile fetches from multiple mounted guards. */
  private profileRequested = false;

  constructor(realm: Realm, api: ApiClient, endpoints: AuthEndpoints) {
    this.realm = realm;
    this.api = api;
    this.endpoints = endpoints;
    makeAutoObservable(this, { realm: false }, { autoBind: true });
  }

  get isAuthenticated(): boolean {
    return this.user !== null;
  }

  /** True when the authenticated staff member carries the `isadmin` flag (TS-M4-A0). */
  get isAdmin(): boolean {
    return Boolean(this.user?.isadmin);
  }

  /** Predicate for a delegated capability flag on the current profile (TS-M4-A0). */
  can(flag: StaffCapability | string): boolean {
    return Boolean(this.user?.[flag]);
  }

  /**
   * Idempotently kick off a best-effort profile fetch. Route guards call this on
   * mount; the first call performs the fetch, later calls are no-ops (so N mounted
   * guards don't issue N /me requests). Already-authenticated sessions (post-login)
   * skip straight past — `profileLoaded` is already "loaded".
   */
  ensureProfileLoaded(): void {
    if (this.profileRequested) return;
    this.profileRequested = true;
    void this.loadProfile();
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
        this.profileLoaded = "loaded";
        this.profileRequested = true;
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
    if (!this.endpoints.profile) {
      // No profile endpoint (e.g. client realm) — nothing to fetch; resolve immediately.
      runInAction(() => {
        this.profileLoaded = "loaded";
      });
      return;
    }
    runInAction(() => {
      this.profileLoaded = "loading";
    });
    try {
      const user = await this.api.get<AuthUser>(this.endpoints.profile);
      runInAction(() => {
        this.user = user;
        this.profileLoaded = "loaded";
      });
    } catch {
      runInAction(() => {
        this.user = null;
        this.profileLoaded = "error";
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
        // The session is resolved (known-absent), not loading — guards should
        // redirect to login rather than spin. Keep `profileRequested` set so a
        // re-mounted guard doesn't refetch a session we just cleared.
        this.profileLoaded = "loaded";
      });
    }
  }
}
