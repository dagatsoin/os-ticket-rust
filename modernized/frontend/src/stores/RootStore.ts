// Root MobX store: composes the two realm-scoped apiClients, the two independent
// auth stores (staff, client), and the health store. Provided to the React tree via
// the StoreProvider/useStores context.
import { ApiClient } from "../api/apiClient";
import { LOGIN_PATH } from "../api/apiClient";
import { AuthStore } from "./AuthStore";
import { HealthStore } from "./HealthStore";
import { OpenTicketStore } from "./OpenTicketStore";
import { StaffTicketStore } from "./StaffTicketStore";
import { ClientPortalStore } from "./ClientPortalStore";
import { TicketOptionsStore } from "./TicketOptionsStore";
import { PublicPageStore } from "./PublicPageStore";
import { SnackbarStore } from "./SnackbarStore";
import { AdminSettingsStore } from "./AdminSettingsStore";
import { StaffAdminStore } from "./StaffAdminStore";
import { GroupAdminStore } from "./GroupAdminStore";
import { ProfileStore } from "./ProfileStore";
import { DirectoryStore } from "./DirectoryStore";
import { SlaAdminStore } from "./SlaAdminStore";
import { PriorityStore } from "./PriorityStore";
import { PageAdminStore } from "./PageAdminStore";
import { LogViewerStore } from "./LogViewerStore";
import { CannedAdminStore } from "./CannedAdminStore";
import { DeptAdminStore } from "./DeptAdminStore";
import { TeamAdminStore } from "./TeamAdminStore";
import { TopicAdminStore } from "./TopicAdminStore";
import { FaqCategoryStore } from "./FaqCategoryStore";

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
  /** Client portal session + read-only thread store (TS-M1-D2). */
  readonly clientPortal: ClientPortalStore;
  /** Staff ticket-options reference lists (transfer/assign/edit/search dropdowns). */
  readonly ticketOptions: TicketOptionsStore;
  /** Public site-page (`/pages/:slug`) viewer store. */
  readonly publicPage: PublicPageStore;
  /** Global success/error snackbar reused by every admin CRUD screen (TS-M4-A0). */
  readonly snackbar: SnackbarStore;
  /** Admin System Settings store (TS-M4-A2/A3) — rides the staff apiClient. */
  readonly adminSettings: AdminSettingsStore;
  /** Admin staff-management store (TS-M4-B2). */
  readonly staffAdmin: StaffAdminStore;
  /** Admin permission-group store (TS-M4-B4). */
  readonly groupAdmin: GroupAdminStore;
  /** Own-profile store (TS-M4-B6) — also the source of the forced-change banner. */
  readonly profile: ProfileStore;
  /** Staff directory store (TS-M4-B6). */
  readonly directory: DirectoryStore;
  /** Admin SLA-plan store (TS-M4-D2). */
  readonly slaAdmin: SlaAdminStore;
  /** Read-only priority reference store (TS-M4-D3). */
  readonly priorities: PriorityStore;
  /** Admin site-pages store (TS-M4-F2). */
  readonly pageAdmin: PageAdminStore;
  /** System-log viewer store (TS-M4-G3). */
  readonly logs: LogViewerStore;
  /** Admin canned-response store (TS-M4-H2) — reachable via can_manage_premade. */
  readonly canned: CannedAdminStore;
  /** Admin department store (TS-M4-C2). */
  readonly deptAdmin: DeptAdminStore;
  /** Admin team store (TS-M4-C4). */
  readonly teamAdmin: TeamAdminStore;
  /** Admin help-topic store (TS-M4-C6). */
  readonly topicAdmin: TopicAdminStore;
  /** FAQ-category store (TS-M4-E2) — reachable via can_manage_faq. */
  readonly faqCategories: FaqCategoryStore;

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
    this.clientPortal = new ClientPortalStore(this.clientApi);
    // Ticket-options rides the staff apiClient (any authenticated staff session).
    this.ticketOptions = new TicketOptionsStore(this.staffApi);
    // Public page viewer rides the (unauthenticated) client apiClient.
    this.publicPage = new PublicPageStore(this.clientApi);

    this.snackbar = new SnackbarStore();
    this.adminSettings = new AdminSettingsStore(this.staffApi, this.snackbar);
    this.staffAdmin = new StaffAdminStore(this.staffApi, this.snackbar, this.adminSettings);
    this.groupAdmin = new GroupAdminStore(this.staffApi, this.snackbar, this.adminSettings);
    this.profile = new ProfileStore(this.staffApi, this.snackbar);
    this.directory = new DirectoryStore(this.staffApi);
    this.slaAdmin = new SlaAdminStore(this.staffApi, this.snackbar);
    this.priorities = new PriorityStore(this.staffApi);
    this.pageAdmin = new PageAdminStore(this.staffApi, this.snackbar);
    this.logs = new LogViewerStore(this.staffApi, this.snackbar);
    this.canned = new CannedAdminStore(this.staffApi, this.snackbar);
    this.deptAdmin = new DeptAdminStore(this.staffApi, this.snackbar);
    this.teamAdmin = new TeamAdminStore(this.staffApi, this.snackbar);
    this.topicAdmin = new TopicAdminStore(this.staffApi, this.snackbar);
    this.faqCategories = new FaqCategoryStore(this.staffApi, this.snackbar);
  }
}

// Re-export so callers can reference realm login paths without importing apiClient.
export { LOGIN_PATH };
