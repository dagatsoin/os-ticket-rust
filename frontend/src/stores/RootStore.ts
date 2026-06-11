// Root MobX store: composes the two realm-scoped apiClients, the two independent
// auth stores (staff, client), and the health store. Provided to the React tree via
// the StoreProvider/useStores context.
import { ApiClient } from "../api/apiClient";
import { LOGIN_PATH } from "../api/apiClient";
import { AuthStore } from "./AuthStore";
import { HealthStore } from "./HealthStore";
import { OpenTicketStore } from "./OpenTicketStore";
import { StaffTicketStore } from "./StaffTicketStore";

/** Optional redirect hook so 401s navigate via react-router instead of a hard reload. */
export interface RootStoreOptions {
  onUnauthorized?: (loginPath: string) => void;
}

export class RootStore {
  readonly staffApi: ApiClient;
  readonly clientApi: ApiClient;
  readonly staffAuth: AuthStore;
  readonly clientAuth: AuthStore;
  readonly health: HealthStore;
  /** Public open-a-ticket form store (TS-M1-B3). */
  readonly openTicket: OpenTicketStore;
  /** Staff queue + ticket-detail/reply store (TS-M1-C4). */
  readonly staffTickets: StaffTicketStore;

  constructor(opts: RootStoreOptions = {}) {
    this.staffApi = new ApiClient({ realm: "staff", onUnauthorized: opts.onUnauthorized });
    this.clientApi = new ApiClient({ realm: "client", onUnauthorized: opts.onUnauthorized });

    this.staffAuth = new AuthStore("staff", this.staffApi, {
      login: "/api/staff/login",
      logout: "/api/staff/logout",
      // Login returns only { ok, csrfToken }; the profile is fetched from /me.
      profile: "/api/staff/me",
    });
    this.clientAuth = new AuthStore("client", this.clientApi, {
      login: "/api/client/login",
      logout: "/api/client/logout",
    });

    // Health checks share the client (unauthenticated, public) apiClient.
    this.health = new HealthStore(this.clientApi);

    // Public open-ticket form rides the unauthenticated client apiClient.
    this.openTicket = new OpenTicketStore(this.clientApi);
    this.staffTickets = new StaffTicketStore(this.staffApi);
  }
}

// Re-export so callers can reference realm login paths without importing apiClient.
export { LOGIN_PATH };
